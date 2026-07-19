use anyhow::{Context, Result};

use crate::domain::{
    EntityId, GroupOrdering, LinkGroup, LinkGroupInput, MAX_TEXT_LEN, PersonProfileUpdate,
};
use crate::error::ErrorKind;
use crate::providers::database::{Database, LinkGroupRepository, LinkRepository, PersonRepository};

use super::{AppServices, cache_keys};

impl AppServices {
    pub async fn list_groups(&self, user_id: EntityId) -> Result<Vec<LinkGroup>> {
        let key = cache_keys::groups(user_id);
        if let Some(groups) = self.cache.get(&key).await {
            return Ok(groups);
        }

        let groups = self
            .database
            .groups()
            .list(user_id)
            .await
            .context("list groups")?;
        self.cache.set(&key, &groups).await;
        Ok(groups)
    }

    pub async fn list_active_groups(&self, user_id: EntityId) -> Result<Vec<LinkGroup>> {
        let key = cache_keys::active_groups(user_id);
        if let Some(groups) = self.cache.get(&key).await {
            return Ok(groups);
        }

        let groups = self
            .database
            .groups()
            .list_active(user_id)
            .await
            .context("list active groups")?;
        self.cache.set(&key, &groups).await;
        Ok(groups)
    }

    pub async fn create_group(
        &self,
        user_id: EntityId,
        input: LinkGroupInput,
        max_groups: usize,
    ) -> Result<LinkGroup> {
        validate_group_input(&input)?;

        let count = self
            .database
            .groups()
            .count(user_id)
            .await
            .context("count groups")?;
        if count as usize >= max_groups {
            return Err(ErrorKind::Conflict(format!(
                "group limit reached: at most {max_groups} groups are allowed"
            ))
            .into());
        }

        let group = self
            .database
            .groups()
            .create(user_id, &input)
            .await
            .context("create group")?;

        self.invalidate_group_change(user_id, &[]).await;
        Ok(group)
    }

    pub async fn update_group(
        &self,
        user_id: EntityId,
        id: EntityId,
        input: LinkGroupInput,
    ) -> Result<LinkGroup> {
        validate_group_input(&input)?;

        let group = self
            .database
            .groups()
            .update(id, user_id, &input)
            .await
            .context("update group")?
            .ok_or_else(group_not_found)?;

        self.invalidate_group_change(user_id, &[]).await;
        Ok(group)
    }

    pub async fn delete_group(&self, user_id: EntityId, id: EntityId) -> Result<()> {
        let affected_link_ids: Vec<EntityId> = self
            .database
            .links()
            .list(user_id)
            .await
            .context("list links assigned to group")?
            .into_iter()
            .filter(|link| link.group_id == Some(id))
            .map(|link| link.id)
            .collect();

        let deleted = self
            .database
            .groups()
            .delete(id, user_id)
            .await
            .context("delete group")?;
        if !deleted {
            return Err(group_not_found());
        }

        let unassign_result = self
            .database
            .links()
            .unassign_group(user_id, id)
            .await
            .context("unassign links from deleted group");

        self.invalidate_group_change(user_id, &affected_link_ids)
            .await;
        unassign_result
    }

    /// Reorders the owner's named groups. When `ungrouped_position` is
    /// `Some`, it also persists where the synthetic "Ungrouped" block sits
    /// among them (its insertion index in the combined ordering). `None`
    /// leaves the stored ungrouped position untouched (e.g. legacy clients
    /// that only send group ids).
    pub async fn reorder_groups(
        &self,
        user_id: EntityId,
        ordering: GroupOrdering,
        ungrouped_position: Option<i32>,
    ) -> Result<()> {
        self.database
            .groups()
            .reorder(user_id, &ordering)
            .await
            .context("reorder groups")?;

        self.invalidate_group_change(user_id, &[]).await;

        if let Some(position) = ungrouped_position {
            let update = PersonProfileUpdate {
                ungrouped_position: Some(position),
                ..Default::default()
            };
            if let Some(person) = self
                .database
                .people()
                .update_profile(user_id, &update)
                .await
                .context("persist ungrouped block position")?
            {
                self.cache
                    .invalidate(&cache_keys::person_invalidation(
                        person.id,
                        &[person.username.as_str()],
                    ))
                    .await;
            }
        }
        Ok(())
    }

    async fn invalidate_group_change(&self, user_id: EntityId, link_ids: &[EntityId]) {
        let mut keys = cache_keys::group_invalidation(user_id);
        keys.extend(cache_keys::link_invalidation(user_id, None));
        keys.extend(link_ids.iter().copied().map(cache_keys::link));
        keys.sort_unstable();
        keys.dedup();
        self.cache.invalidate(&keys).await;
    }
}

fn group_not_found() -> anyhow::Error {
    ErrorKind::NotFound("group not found".to_string()).into()
}

fn validate_group_input(input: &LinkGroupInput) -> Result<()> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(ErrorKind::BadRequest("group title must not be empty".into()).into());
    }
    if title.chars().count() > MAX_TEXT_LEN {
        return Err(ErrorKind::BadRequest(format!(
            "group title must be at most {MAX_TEXT_LEN} characters"
        ))
        .into());
    }
    if let Some(description) = &input.description
        && description.chars().count() > MAX_TEXT_LEN
    {
        return Err(ErrorKind::BadRequest(format!(
            "group description must be at most {MAX_TEXT_LEN} characters"
        ))
        .into());
    }
    Ok(())
}
