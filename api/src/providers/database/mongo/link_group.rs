//! Mongo-backed [`LinkGroupRepository`] implementation.

use anyhow::{Context, Result};
use bson::{Document, doc};
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database as MongoDb, IndexModel};
use serde::{Deserialize, Serialize};

use crate::domain::{EntityId, GroupOrdering, GroupStyle, LinkGroup, LinkGroupInput};

use crate::providers::database::traits::LinkGroupRepository;

/// Persistence record for [`LinkGroup`]; see [`super::person::PersonRecord`]
/// for why timestamps use bson's chrono helpers instead of chrono's own
/// `Serialize` impl.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LinkGroupRecord {
    id: EntityId,
    user_id: EntityId,
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    sort_order: i32,
    #[serde(default)]
    is_collapsible: bool,
    #[serde(default)]
    is_active: bool,
    #[serde(default)]
    style: GroupStyle,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    created_at: DateTime<Utc>,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    updated_at: DateTime<Utc>,
}

impl From<&LinkGroup> for LinkGroupRecord {
    fn from(group: &LinkGroup) -> Self {
        Self {
            id: group.id,
            user_id: group.user_id,
            title: group.title.clone(),
            description: group.description.clone(),
            sort_order: group.sort_order,
            is_collapsible: group.is_collapsible,
            is_active: group.is_active,
            style: group.style,
            created_at: group.created_at,
            updated_at: group.updated_at,
        }
    }
}

impl From<LinkGroupRecord> for LinkGroup {
    fn from(record: LinkGroupRecord) -> Self {
        Self {
            id: record.id,
            user_id: record.user_id,
            title: record.title,
            description: record.description,
            sort_order: record.sort_order,
            is_collapsible: record.is_collapsible,
            is_active: record.is_active,
            style: record.style,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

/// Filter matching every group owned by `user_id`, optionally restricted to
/// active groups only (used by the public page).
fn owner_filter(user_id: EntityId, active_only: bool) -> Document {
    let mut filter = doc! { "user_id": user_id.to_string() };
    if active_only {
        filter.insert("is_active", true);
    }
    filter
}

#[derive(Debug, Clone)]
pub struct MongoLinkGroupRepository {
    collection: Collection<LinkGroupRecord>,
}

impl MongoLinkGroupRepository {
    pub(super) fn new(database: MongoDb) -> Self {
        Self {
            collection: database.collection("link_groups"),
        }
    }

    pub(super) async fn ensure_indexes(&self) -> Result<()> {
        let unique = IndexOptions::builder().unique(true).build();
        self.collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "id": 1 })
                    .options(unique)
                    .build(),
            )
            .await
            .context("create link_groups id index")?;
        self.collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "user_id": 1, "sort_order": 1 })
                    .build(),
            )
            .await
            .context("create link_groups sort index")?;
        Ok(())
    }

    async fn find_owned(&self, id: EntityId, user_id: EntityId) -> Result<Option<LinkGroup>> {
        let record = self
            .collection
            .find_one(doc! { "id": id.to_string(), "user_id": user_id.to_string() })
            .await
            .context("find link group")?;
        Ok(record.map(LinkGroup::from))
    }

    async fn list_with_filter(&self, filter: Document) -> Result<Vec<LinkGroup>> {
        let cursor = self
            .collection
            .find(filter)
            .sort(doc! { "sort_order": 1 })
            .await
            .context("list link groups")?;
        let records: Vec<LinkGroupRecord> =
            cursor.try_collect().await.context("collect link groups")?;
        Ok(records.into_iter().map(LinkGroup::from).collect())
    }
}

impl LinkGroupRepository for MongoLinkGroupRepository {
    async fn list(&self, user_id: EntityId) -> Result<Vec<LinkGroup>> {
        self.list_with_filter(owner_filter(user_id, false)).await
    }

    async fn list_active(&self, user_id: EntityId) -> Result<Vec<LinkGroup>> {
        self.list_with_filter(owner_filter(user_id, true)).await
    }

    async fn count(&self, user_id: EntityId) -> Result<u64> {
        self.collection
            .count_documents(doc! { "user_id": user_id.to_string() })
            .await
            .context("count link groups")
    }

    async fn create(&self, user_id: EntityId, input: &LinkGroupInput) -> Result<LinkGroup> {
        let sort_order = self.count(user_id).await? as i32;
        let mut group = LinkGroup::new(user_id, input.title.clone(), sort_order);
        group.description = input.description.clone();
        if let Some(is_collapsible) = input.is_collapsible {
            group.is_collapsible = is_collapsible;
        }
        if let Some(style) = input.style {
            group.style = style;
        }

        let record = LinkGroupRecord::from(&group);
        self.collection
            .insert_one(&record)
            .await
            .context("insert link group")?;
        Ok(group)
    }

    async fn update(
        &self,
        id: EntityId,
        user_id: EntityId,
        input: &LinkGroupInput,
    ) -> Result<Option<LinkGroup>> {
        let mut set = doc! {
            "title": input.title.clone(),
            "description": input.description.clone().unwrap_or_default(),
            "updated_at": bson::DateTime::from_chrono(Utc::now()),
        };
        if let Some(is_collapsible) = input.is_collapsible {
            set.insert("is_collapsible", is_collapsible);
        }
        if let Some(style) = input.style {
            set.insert(
                "style",
                bson::to_bson(&style).context("serialize group style")?,
            );
        }

        let result = self
            .collection
            .update_one(
                doc! { "id": id.to_string(), "user_id": user_id.to_string() },
                doc! { "$set": set },
            )
            .await
            .context("update link group")?;
        if result.matched_count == 0 {
            return Ok(None);
        }
        self.find_owned(id, user_id).await
    }

    async fn delete(&self, id: EntityId, user_id: EntityId) -> Result<bool> {
        let result = self
            .collection
            .delete_one(doc! { "id": id.to_string(), "user_id": user_id.to_string() })
            .await
            .context("delete link group")?;
        Ok(result.deleted_count > 0)
    }

    async fn reorder(&self, user_id: EntityId, ordering: &GroupOrdering) -> Result<()> {
        for (index, id) in ordering.ordered_ids.iter().enumerate() {
            self.collection
                .update_one(
                    doc! { "id": id.to_string(), "user_id": user_id.to_string() },
                    doc! { "$set": {
                        "sort_order": index as i32,
                        "updated_at": bson::DateTime::from_chrono(Utc::now()),
                    } },
                )
                .await
                .context("reorder link group")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_group() -> LinkGroup {
        LinkGroup::new(EntityId::new(), "Shopping", 2)
    }

    #[test]
    fn record_round_trips_through_link_group() {
        let group = sample_group();
        let record = LinkGroupRecord::from(&group);
        let back: LinkGroup = record.into();
        assert_eq!(back, group);
    }

    #[test]
    fn owner_filter_scopes_to_user() {
        let user_id = EntityId::new();
        assert_eq!(
            owner_filter(user_id, false),
            doc! { "user_id": user_id.to_string() }
        );
    }

    #[test]
    fn owner_filter_active_only_adds_is_active_clause() {
        let user_id = EntityId::new();
        assert_eq!(
            owner_filter(user_id, true),
            doc! { "user_id": user_id.to_string(), "is_active": true }
        );
    }
}
