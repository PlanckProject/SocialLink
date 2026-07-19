use anyhow::{Context, Result};

use crate::domain::{EntityId, Link, LinkInput, LinkOrdering, MAX_TEXT_LEN};
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
    input.url = normalize_link_url(&input.url)?;
    input.title = validate_title(&input.title)?;
    input.description = validate_description(input.description)?;
    Ok(input)
}

fn validate_title(title: &str) -> Result<String> {
    let title = title.trim();
    if title.is_empty() {
        return Err(ErrorKind::BadRequest("link title is required".to_string()).into());
    }
    if title.chars().count() > MAX_TEXT_LEN {
        return Err(ErrorKind::BadRequest(format!(
            "link title must be at most {MAX_TEXT_LEN} characters"
        ))
        .into());
    }
    Ok(title.to_string())
}

fn validate_description(description: Option<String>) -> Result<Option<String>> {
    if let Some(description) = &description
        && description.chars().count() > MAX_TEXT_LEN
    {
        return Err(ErrorKind::BadRequest(format!(
            "link description must be at most {MAX_TEXT_LEN} characters"
        ))
        .into());
    }
    Ok(description)
}

/// Schemes that can execute script or read local resources; never allowed in a
/// user-supplied link `href`/redirect target.
const BLOCKED_URL_SCHEMES: [&str; 4] = ["javascript", "data", "vbscript", "file"];

/// Opaque (non-hierarchical) schemes we explicitly accept as-is. Anything with
/// an explicit `scheme://` authority is also accepted (custom app deep links),
/// while bare `scheme:opaque` input is only trusted for this allowlist so that
/// host-like input such as `example.com:8080` still gets an `https://` prefix.
const OPAQUE_URL_SCHEMES: [&str; 20] = [
    "mailto", "tel", "sms", "mms", "geo", "maps", "facetime", "bitcoin", "ethereum", "xmpp",
    "irc", "ircs", "matrix", "magnet", "webcal", "whatsapp", "skype", "callto", "tg", "signal",
];

/// Extracts an RFC 3986 scheme (`ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`)
/// if the string is of the form `scheme:...`.
fn extract_scheme(url: &str) -> Option<&str> {
    let bytes = url.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_alphabetic() {
        return None;
    }
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b':' => return Some(&url[..i]),
            b if b.is_ascii_alphanumeric() || b == b'+' || b == b'-' || b == b'.' => continue,
            _ => return None,
        }
    }
    None
}

fn validate_web_host(after_scheme: &str) -> Result<()> {
    let host = after_scheme
        .trim_start_matches("//")
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    // Strip any userinfo and port so only the hostname is validated.
    let host = host.rsplit('@').next().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host);
    if !host.contains('.') || host.starts_with('.') || host.ends_with('.') {
        return Err(
            ErrorKind::BadRequest("link URL must include a valid domain".to_string()).into(),
        );
    }
    Ok(())
}

/// Normalizes and validates a user-supplied link target. Accepts http/https,
/// common opaque schemes (mailto:, tel:, sms:, …), and custom `scheme://` deep
/// links, while rejecting dangerous schemes (javascript:, data:, …). Bare hosts
/// are upgraded to `https://`.
fn normalize_link_url(url: &str) -> Result<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(ErrorKind::BadRequest("link URL is required".to_string()).into());
    }

    if let Some(scheme) = extract_scheme(trimmed) {
        let lower = scheme.to_ascii_lowercase();
        if BLOCKED_URL_SCHEMES.contains(&lower.as_str()) {
            return Err(
                ErrorKind::BadRequest("link URL scheme is not allowed".to_string()).into(),
            );
        }

        let rest = &trimmed[scheme.len() + 1..];
        if lower == "http" || lower == "https" {
            validate_web_host(rest)?;
            return Ok(trimmed.to_string());
        }

        // Explicit authority-based deep link (e.g. myapp://open) or a known
        // opaque scheme (mailto:, tel:, …) — accept verbatim.
        if rest.starts_with("//") || OPAQUE_URL_SCHEMES.contains(&lower.as_str()) {
            if rest.trim_start_matches("//").trim().is_empty() {
                return Err(ErrorKind::BadRequest(format!(
                    "link URL is missing a target after {lower}:"
                ))
                .into());
            }
            return Ok(trimmed.to_string());
        }
        // Otherwise fall through and treat the whole string as a bare host
        // (covers `example.com:8080/path`).
    }

    let normalized = format!("https://{trimmed}");
    validate_web_host(&normalized["https://".len()..])?;
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
            normalize_link_url(" example.com/path ").unwrap(),
            "https://example.com/path"
        );
    }

    #[test]
    fn accepts_http_and_https_with_valid_host() {
        assert_eq!(
            normalize_link_url("http://example.com").unwrap(),
            "http://example.com"
        );
        assert_eq!(
            normalize_link_url("https://example.com/a?b#c").unwrap(),
            "https://example.com/a?b#c"
        );
    }

    #[test]
    fn upgrades_bare_host_with_port() {
        assert_eq!(
            normalize_link_url("example.com:8080/path").unwrap(),
            "https://example.com:8080/path"
        );
    }

    #[test]
    fn accepts_common_opaque_schemes() {
        assert_eq!(
            normalize_link_url("mailto:hi@example.com").unwrap(),
            "mailto:hi@example.com"
        );
        assert_eq!(
            normalize_link_url("tel:+15551234567").unwrap(),
            "tel:+15551234567"
        );
        assert_eq!(
            normalize_link_url("sms:+15551234567").unwrap(),
            "sms:+15551234567"
        );
    }

    #[test]
    fn accepts_custom_scheme_deep_links() {
        assert_eq!(
            normalize_link_url("myapp://open/thing").unwrap(),
            "myapp://open/thing"
        );
    }

    #[test]
    fn rejects_dangerous_schemes() {
        assert!(normalize_link_url("javascript:alert(1)").is_err());
        assert!(normalize_link_url("JavaScript:alert(1)").is_err());
        assert!(normalize_link_url("data:text/html;base64,PHN2Zz4=").is_err());
        assert!(normalize_link_url("vbscript:msgbox(1)").is_err());
        assert!(normalize_link_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn rejects_invalid_hosts() {
        assert!(normalize_link_url("https://localhost").is_err());
        assert!(normalize_link_url("http://localhost").is_err());
    }

    #[test]
    fn rejects_empty_opaque_target() {
        assert!(normalize_link_url("mailto:").is_err());
    }

    #[test]
    fn rejects_blank_title() {
        assert!(validate_title("   ").is_err());
    }

    #[test]
    fn rejects_overlong_title_and_description() {
        let long = "a".repeat(MAX_TEXT_LEN + 1);
        assert!(validate_title(&long).is_err());
        assert!(validate_description(Some(long)).is_err());
    }

    #[test]
    fn accepts_max_length_description() {
        let ok = "a".repeat(MAX_TEXT_LEN);
        assert_eq!(validate_description(Some(ok.clone())).unwrap(), Some(ok));
    }
}
