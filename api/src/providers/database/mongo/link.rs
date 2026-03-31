//! Mongo-backed [`LinkRepository`] implementation.

use anyhow::{Context, Result};
use bson::{Bson, Document, doc};
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database as MongoDb, IndexModel};
use serde::{Deserialize, Serialize};

use crate::domain::{EntityId, Link, LinkFilter, LinkInput, LinkOrdering};

use crate::providers::database::traits::LinkRepository;

/// Persistence record for [`Link`]; see [`super::person::PersonRecord`] for
/// why timestamps use bson's chrono helpers instead of chrono's own
/// `Serialize` impl.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LinkRecord {
    id: EntityId,
    user_id: EntityId,
    #[serde(default)]
    group_id: Option<EntityId>,
    title: String,
    url: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    icon_image: Option<String>,
    #[serde(default)]
    icon_font: Option<String>,
    #[serde(default)]
    sort_order: i32,
    #[serde(default)]
    is_active: bool,
    #[serde(default)]
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime_optional")]
    expires_at: Option<DateTime<Utc>>,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    created_at: DateTime<Utc>,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    updated_at: DateTime<Utc>,
}

impl From<&Link> for LinkRecord {
    fn from(link: &Link) -> Self {
        Self {
            id: link.id,
            user_id: link.user_id,
            group_id: link.group_id,
            title: link.title.clone(),
            url: link.url.clone(),
            description: link.description.clone(),
            icon: link.icon.clone(),
            icon_image: link.icon_image.clone(),
            icon_font: link.icon_font.clone(),
            sort_order: link.sort_order,
            is_active: link.is_active,
            expires_at: link.expires_at,
            created_at: link.created_at,
            updated_at: link.updated_at,
        }
    }
}

impl From<LinkRecord> for Link {
    fn from(record: LinkRecord) -> Self {
        Self {
            id: record.id,
            user_id: record.user_id,
            group_id: record.group_id,
            title: record.title,
            url: record.url,
            description: record.description,
            icon: record.icon,
            icon_image: record.icon_image,
            icon_font: record.icon_font,
            sort_order: record.sort_order,
            is_active: record.is_active,
            expires_at: record.expires_at,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

/// Builds the filter document for a [`LinkFilter`]: always scoped to
/// `user_id`; `group_id: Some(None)` restricts to ungrouped links,
/// `group_id: Some(Some(id))` restricts to one group, `group_id: None`
/// leaves the group unrestricted.
fn link_filter_doc(filter: &LinkFilter) -> Document {
    let mut doc = doc! { "user_id": filter.user_id.to_string() };
    if let Some(group_id) = filter.group_id {
        match group_id {
            Some(id) => {
                doc.insert("group_id", id.to_string());
            }
            None => {
                doc.insert("group_id", Bson::Null);
            }
        }
    }
    doc
}

/// Filter matching an owner's active, non-expired links (public page):
/// `is_active == true` and `expires_at` is either unset or in the future.
fn public_filter_doc(user_id: EntityId, now: DateTime<Utc>) -> Document {
    doc! {
        "user_id": user_id.to_string(),
        "is_active": true,
        "$or": [
            { "expires_at": Bson::Null },
            { "expires_at": { "$gt": bson::DateTime::from_chrono(now) } },
        ],
    }
}

#[derive(Debug, Clone)]
pub struct MongoLinkRepository {
    collection: Collection<LinkRecord>,
}

impl MongoLinkRepository {
    pub(super) fn new(database: MongoDb) -> Self {
        Self {
            collection: database.collection("links"),
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
            .context("create links id index")?;
        self.collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "user_id": 1, "group_id": 1, "sort_order": 1 })
                    .build(),
            )
            .await
            .context("create links sort index")?;
        Ok(())
    }

    async fn find_with_filter(&self, filter: Document) -> Result<Vec<Link>> {
        let cursor = self
            .collection
            .find(filter)
            .sort(doc! { "sort_order": 1 })
            .await
            .context("list links")?;
        let records: Vec<LinkRecord> = cursor.try_collect().await.context("collect links")?;
        Ok(records.into_iter().map(Link::from).collect())
    }
}

impl LinkRepository for MongoLinkRepository {
    async fn list(&self, user_id: EntityId) -> Result<Vec<Link>> {
        self.find_with_filter(doc! { "user_id": user_id.to_string() })
            .await
    }

    async fn list_public(&self, user_id: EntityId) -> Result<Vec<Link>> {
        self.find_with_filter(public_filter_doc(user_id, Utc::now()))
            .await
    }

    async fn count(&self, filter: &LinkFilter) -> Result<u64> {
        self.collection
            .count_documents(link_filter_doc(filter))
            .await
            .context("count links")
    }

    async fn get(&self, id: EntityId, user_id: EntityId) -> Result<Option<Link>> {
        let record = self
            .collection
            .find_one(doc! { "id": id.to_string(), "user_id": user_id.to_string() })
            .await
            .context("get link")?;
        Ok(record.map(Link::from))
    }

    async fn get_by_id(&self, id: EntityId) -> Result<Option<Link>> {
        let record = self
            .collection
            .find_one(doc! { "id": id.to_string() })
            .await
            .context("get link by id")?;
        Ok(record.map(Link::from))
    }

    async fn create(&self, user_id: EntityId, input: &LinkInput) -> Result<Link> {
        let scope = LinkFilter::for_user(user_id).in_group(input.group_id);
        let sort_order = self.count(&scope).await? as i32;

        let mut link = Link::new(
            user_id,
            input.group_id,
            input.title.clone(),
            input.url.clone(),
            sort_order,
        );
        link.description = input.description.clone();
        link.icon = input.icon.clone();
        link.icon_image = input.icon_image.clone();
        link.icon_font = input.icon_font.clone();
        link.expires_at = input.expires_at;
        if let Some(is_active) = input.is_active {
            link.is_active = is_active;
        }

        let record = LinkRecord::from(&link);
        self.collection
            .insert_one(&record)
            .await
            .context("insert link")?;
        Ok(link)
    }

    async fn update(
        &self,
        id: EntityId,
        user_id: EntityId,
        input: &LinkInput,
    ) -> Result<Option<Link>> {
        let mut set = doc! {
            "title": input.title.clone(),
            "url": input.url.clone(),
            "description": input.description.clone().unwrap_or_default(),
            "icon": input.icon.clone().unwrap_or_default(),
            "icon_image": input.icon_image.clone().unwrap_or_default(),
            "icon_font": input.icon_font.clone().unwrap_or_default(),
            "group_id": input.group_id.map(|id| Bson::String(id.to_string())).unwrap_or(Bson::Null),
            "expires_at": input
                .expires_at
                .map(bson::DateTime::from_chrono)
                .map(Bson::DateTime)
                .unwrap_or(Bson::Null),
            "updated_at": bson::DateTime::from_chrono(Utc::now()),
        };
        if let Some(is_active) = input.is_active {
            set.insert("is_active", is_active);
        }

        let result = self
            .collection
            .update_one(
                doc! { "id": id.to_string(), "user_id": user_id.to_string() },
                doc! { "$set": set },
            )
            .await
            .context("update link")?;
        if result.matched_count == 0 {
            return Ok(None);
        }
        self.get(id, user_id).await
    }

    async fn delete(&self, id: EntityId, user_id: EntityId) -> Result<bool> {
        let result = self
            .collection
            .delete_one(doc! { "id": id.to_string(), "user_id": user_id.to_string() })
            .await
            .context("delete link")?;
        Ok(result.deleted_count > 0)
    }

    async fn reorder(&self, user_id: EntityId, ordering: &LinkOrdering) -> Result<()> {
        let group_bson = ordering
            .group_id
            .map(|id| Bson::String(id.to_string()))
            .unwrap_or(Bson::Null);
        for (index, id) in ordering.ordered_ids.iter().enumerate() {
            self.collection
                .update_one(
                    doc! { "id": id.to_string(), "user_id": user_id.to_string() },
                    doc! { "$set": {
                        "sort_order": index as i32,
                        "group_id": group_bson.clone(),
                    } },
                )
                .await
                .context("reorder link")?;
        }
        Ok(())
    }

    async fn unassign_group(&self, user_id: EntityId, group_id: EntityId) -> Result<()> {
        self.collection
            .update_many(
                doc! { "user_id": user_id.to_string(), "group_id": group_id.to_string() },
                doc! { "$set": { "group_id": Bson::Null } },
            )
            .await
            .context("unassign link group")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_link() -> Link {
        Link::new(EntityId::new(), None, "GitHub", "https://github.com", 0)
    }

    #[test]
    fn record_round_trips_through_link() {
        let link = sample_link();
        let record = LinkRecord::from(&link);
        let back: Link = record.into();
        assert_eq!(back, link);
    }

    #[test]
    fn link_filter_doc_scopes_to_user_when_group_unset() {
        let user_id = EntityId::new();
        let filter = LinkFilter::for_user(user_id);
        assert_eq!(
            link_filter_doc(&filter),
            doc! { "user_id": user_id.to_string() }
        );
    }

    #[test]
    fn link_filter_doc_matches_ungrouped_links() {
        let user_id = EntityId::new();
        let filter = LinkFilter::for_user(user_id).in_group(None);
        assert_eq!(
            link_filter_doc(&filter),
            doc! { "user_id": user_id.to_string(), "group_id": Bson::Null }
        );
    }

    #[test]
    fn link_filter_doc_matches_one_group() {
        let user_id = EntityId::new();
        let group_id = EntityId::new();
        let filter = LinkFilter::for_user(user_id).in_group(Some(group_id));
        assert_eq!(
            link_filter_doc(&filter),
            doc! { "user_id": user_id.to_string(), "group_id": group_id.to_string() }
        );
    }

    #[test]
    fn public_filter_doc_requires_active_and_unexpired() {
        let user_id = EntityId::new();
        let now = Utc::now();
        let filter = public_filter_doc(user_id, now);
        assert_eq!(
            filter,
            doc! {
                "user_id": user_id.to_string(),
                "is_active": true,
                "$or": [
                    { "expires_at": Bson::Null },
                    { "expires_at": { "$gt": bson::DateTime::from_chrono(now) } },
                ],
            }
        );
    }
}
