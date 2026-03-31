use anyhow::{Context, Result};

use crate::domain::{EntityId, GroupOrdering, LinkGroup, LinkGroupInput};
use crate::error::ErrorKind;
use crate::providers::database::{Database, LinkGroupRepository, LinkRepository};

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
    ) -> Result<LinkGroup> {
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

    pub async fn reorder_groups(&self, user_id: EntityId, ordering: GroupOrdering) -> Result<()> {
        self.database
            .groups()
            .reorder(user_id, &ordering)
            .await
            .context("reorder groups")?;

        self.invalidate_group_change(user_id, &[]).await;
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
