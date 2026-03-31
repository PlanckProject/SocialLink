use anyhow::{Context, Result};

use crate::domain::{EntityId, Link, LinkInput, LinkOrdering};
use crate::error::ErrorKind;
use crate::io::http::handlers::dto::AdminLinkDto;
use crate::providers::database::{Database, LinkRepository};
use crate::providers::timeseries::TimeSeries;

use super::{AppServices, cache_keys};

impl AppServices {
    pub async fn list_links(&self, user_id: EntityId) -> Result<Vec<Link>> {
        let key = cache_keys::links(user_id);
        if let Some(links) = self.cache.get(&key).await {
            return Ok(links);
        }

        let links = self
            .database
            .links()
            .list(user_id)
            .await
            .context("list links")?;
        self.cache.set(&key, &links).await;
        Ok(links)
    }

    pub async fn list_public_links(&self, user_id: EntityId) -> Result<Vec<Link>> {
        let key = cache_keys::public_links(user_id);
        if let Some(links) = self.cache.get(&key).await {
            return Ok(links);
        }

        let links = self
            .database
            .links()
            .list_public(user_id)
            .await
            .context("list public links")?;
        self.cache.set(&key, &links).await;
        Ok(links)
    }

    pub async fn get_link(&self, user_id: EntityId, id: EntityId) -> Result<Link> {
        let key = cache_keys::link(id);
        if let Some(link) = self.cache.get::<Link>(&key).await {
            return if link.user_id == user_id {
                Ok(link)
            } else {
                Err(link_not_found())
            };
        }

        let link = self
            .database
            .links()
            .get(id, user_id)
            .await
            .context("get link")?
            .ok_or_else(link_not_found)?;
        self.cache.set(&key, &link).await;
        Ok(link)
    }

    pub async fn create_link(&self, user_id: EntityId, input: LinkInput) -> Result<Link> {
        let input = prepare_link_input(input)?;
        let link = self
            .database
            .links()
            .create(user_id, &input)
            .await
            .context("create link")?;

        self.invalidate_link_change(user_id, &[link.id]).await;
        Ok(link)
    }

    pub async fn update_link(
        &self,
        user_id: EntityId,
        id: EntityId,
        input: LinkInput,
    ) -> Result<Link> {
        let input = prepare_link_input(input)?;
        let link = self
            .database
            .links()
            .update(id, user_id, &input)
            .await
            .context("update link")?
            .ok_or_else(link_not_found)?;

        self.invalidate_link_change(user_id, &[id]).await;
        Ok(link)
    }

    pub async fn delete_link(&self, user_id: EntityId, id: EntityId) -> Result<()> {
        let deleted = self
            .database
            .links()
            .delete(id, user_id)
            .await
            .context("delete link")?;
        if !deleted {
            return Err(link_not_found());
        }

        self.invalidate_link_change(user_id, &[id]).await;
        Ok(())
    }

    pub async fn reorder_links(&self, user_id: EntityId, ordering: LinkOrdering) -> Result<()> {
        self.database
            .links()
            .reorder(user_id, &ordering)
            .await
            .context("reorder links")?;

        self.invalidate_link_change(user_id, &ordering.ordered_ids)
            .await;
        Ok(())
    }

    pub async fn admin_links(&self, user_id: EntityId) -> Result<Vec<AdminLinkDto>> {
        let links = self.list_links(user_id).await?;
        self.enrich_admin_links(user_id, &links).await
    }

    pub async fn admin_link(&self, user_id: EntityId, link: &Link) -> Result<AdminLinkDto> {
        let mut links = self
            .enrich_admin_links(user_id, std::slice::from_ref(link))
            .await?;
        Ok(links
            .pop()
            .expect("one input link always produces one admin DTO"))
    }

    pub async fn enrich_admin_links(
        &self,
        user_id: EntityId,
        links: &[Link],
    ) -> Result<Vec<AdminLinkDto>> {
        let link_ids: Vec<EntityId> = links.iter().map(|link| link.id).collect();
        let totals = self
            .timeseries
            .click_totals(user_id, &link_ids)
            .await
            .context("load link click totals")?;

        Ok(links
            .iter()
            .map(|link| {
                AdminLinkDto::from_model(link, totals.get(&link.id).copied().unwrap_or_default())
            })
            .collect())
    }

    pub async fn link_for_redirect(&self, id: EntityId) -> Result<Link> {
        let key = cache_keys::link(id);
        if let Some(link) = self.cache.get(&key).await {
            return Ok(link);
        }

        let link = self
            .database
            .links()
            .get_by_id(id)
            .await
            .context("get redirect link")?
            .ok_or_else(link_not_found)?;
        self.cache.set(&key, &link).await;
        Ok(link)
    }

    pub async fn preview_link(&self, user_id: EntityId, id: EntityId) -> Result<Link> {
        self.get_link(user_id, id).await
    }

    async fn invalidate_link_change(&self, user_id: EntityId, link_ids: &[EntityId]) {
        let mut keys = cache_keys::link_invalidation(user_id, None);
        keys.extend(link_ids.iter().copied().map(cache_keys::link));
        keys.sort_unstable();
        keys.dedup();
        self.cache.invalidate(&keys).await;
    }
}

fn prepare_link_input(mut input: LinkInput) -> Result<LinkInput> {
    input.url = normalize_https_url(&input.url)?;
    input.title = validate_title(&input.title)?;
    Ok(input)
}

fn validate_title(title: &str) -> Result<String> {
    let title = title.trim();
    if title.is_empty() {
        return Err(ErrorKind::BadRequest("link title is required".to_string()).into());
    }
    Ok(title.to_string())
}

fn normalize_https_url(url: &str) -> Result<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(ErrorKind::BadRequest("link URL is required".to_string()).into());
    }

    let normalized = if trimmed.to_ascii_lowercase().starts_with("https://") {
        trimmed.to_string()
    } else if trimmed.contains("://") {
        return Err(ErrorKind::BadRequest("link URL must use https://".to_string()).into());
    } else {
        format!("https://{trimmed}")
    };

    let host = normalized["https://".len()..]
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    if !host.contains('.') || host.starts_with('.') || host.ends_with('.') {
        return Err(
            ErrorKind::BadRequest("link URL must include a valid domain".to_string()).into(),
        );
    }
    Ok(normalized)
}

fn link_not_found() -> anyhow::Error {
    ErrorKind::NotFound("link not found".to_string()).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_scheme_less_https_url() {
        assert_eq!(
            normalize_https_url(" example.com/path ").unwrap(),
            "https://example.com/path"
        );
    }

    #[test]
    fn rejects_non_https_and_invalid_hosts() {
        assert!(normalize_https_url("http://example.com").is_err());
        assert!(normalize_https_url("https://localhost").is_err());
    }

    #[test]
    fn rejects_blank_title() {
        assert!(validate_title("   ").is_err());
    }
}
