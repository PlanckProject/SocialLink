//! Mongo-backed [`ThemeRepository`] implementation.

use anyhow::{Context, Result};
use bson::{Document, doc};
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database as MongoDb, IndexModel};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::{EntityId, Theme, ThemeFilter, ThemeSource, ThemeUpdate};

use crate::providers::database::traits::ThemeRepository;

/// Persistence record for [`Theme`]; see [`super::person::PersonRecord`]
/// for why timestamps use bson's chrono helpers instead of chrono's own
/// `Serialize` impl.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ThemeRecord {
    id: EntityId,
    user_id: EntityId,
    #[serde(default)]
    owner: Option<String>,
    name: String,
    #[serde(default)]
    is_active: bool,
    #[serde(default)]
    is_favorite: bool,
    #[serde(default)]
    is_public: bool,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    source: ThemeSource,
    #[serde(default)]
    download_count: i64,
    config: Value,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    created_at: DateTime<Utc>,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    updated_at: DateTime<Utc>,
}

impl From<&Theme> for ThemeRecord {
    fn from(theme: &Theme) -> Self {
        Self {
            id: theme.id,
            user_id: theme.user_id,
            owner: theme.owner.clone(),
            name: theme.name.clone(),
            is_active: theme.is_active,
            is_favorite: theme.is_favorite,
            is_public: theme.is_public,
            description: theme.description.clone(),
            tags: theme.tags.clone(),
            source: theme.source,
            download_count: theme.download_count,
            config: theme.config.clone(),
            created_at: theme.created_at,
            updated_at: theme.updated_at,
        }
    }
}

impl From<ThemeRecord> for Theme {
    fn from(record: ThemeRecord) -> Self {
        Self {
            id: record.id,
            user_id: record.user_id,
            owner: record.owner,
            name: record.name,
            is_active: record.is_active,
            is_favorite: record.is_favorite,
            is_public: record.is_public,
            description: record.description,
            tags: record.tags,
            source: record.source,
            download_count: record.download_count,
            config: record.config,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

/// Builds the filter document for a [`ThemeFilter`]: always scoped to
/// `user_id`, optionally restricted by favorite flag and/or source.
fn theme_filter_doc(filter: &ThemeFilter) -> Document {
    let mut doc = doc! { "user_id": filter.user_id.to_string() };
    if let Some(favorite) = filter.favorite {
        doc.insert("is_favorite", favorite);
    }
    if let Some(source) = filter.source {
        doc.insert("source", source_str(source));
    }
    doc
}

fn source_str(source: ThemeSource) -> &'static str {
    match source {
        ThemeSource::Custom => "custom",
        ThemeSource::Imported => "imported",
        ThemeSource::Preset => "preset",
        ThemeSource::Marketplace => "marketplace",
    }
}

#[derive(Debug, Clone)]
pub struct MongoThemeRepository {
    collection: Collection<ThemeRecord>,
}

impl MongoThemeRepository {
    pub(super) fn new(database: MongoDb) -> Self {
        Self {
            collection: database.collection("themes"),
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
            .context("create themes id index")?;
        self.collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "user_id": 1, "is_active": 1 })
                    .build(),
            )
            .await
            .context("create themes active index")?;
        self.collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "user_id": 1, "source": 1, "name": 1 })
                    .build(),
            )
            .await
            .context("create themes preset lookup index")?;
        Ok(())
    }

    async fn find_owned(&self, id: EntityId, user_id: EntityId) -> Result<Option<Theme>> {
        let record = self
            .collection
            .find_one(doc! { "id": id.to_string(), "user_id": user_id.to_string() })
            .await
            .context("find theme")?;
        Ok(record.map(Theme::from))
    }
}

impl ThemeRepository for MongoThemeRepository {
    async fn list(&self, filter: &ThemeFilter) -> Result<Vec<Theme>> {
        let cursor = self
            .collection
            .find(theme_filter_doc(filter))
            .sort(doc! { "is_favorite": -1, "created_at": 1 })
            .await
            .context("list themes")?;
        let records: Vec<ThemeRecord> = cursor.try_collect().await.context("collect themes")?;
        Ok(records.into_iter().map(Theme::from).collect())
    }

    async fn count_all(&self, user_id: EntityId) -> Result<u64> {
        self.collection
            .count_documents(doc! { "user_id": user_id.to_string() })
            .await
            .context("count themes")
    }

    async fn count_saveable(&self, user_id: EntityId) -> Result<u64> {
        self.collection
            .count_documents(doc! {
                "user_id": user_id.to_string(),
                "source": { "$in": ["custom", "imported"] },
            })
            .await
            .context("count saveable themes")
    }

    async fn count_presets(&self, user_id: EntityId) -> Result<u64> {
        self.collection
            .count_documents(doc! {
                "user_id": user_id.to_string(),
                "source": source_str(ThemeSource::Preset),
            })
            .await
            .context("count preset themes")
    }

    async fn active(&self, user_id: EntityId) -> Result<Option<Theme>> {
        let record = self
            .collection
            .find_one(doc! { "user_id": user_id.to_string(), "is_active": true })
            .await
            .context("find active theme")?;
        Ok(record.map(Theme::from))
    }

    async fn by_id(&self, id: EntityId, user_id: EntityId) -> Result<Option<Theme>> {
        self.find_owned(id, user_id).await
    }

    async fn by_preset_name(&self, user_id: EntityId, name: &str) -> Result<Option<Theme>> {
        let record = self
            .collection
            .find_one(doc! {
                "user_id": user_id.to_string(),
                "source": source_str(ThemeSource::Preset),
                "name": name,
            })
            .await
            .context("find preset theme")?;
        Ok(record.map(Theme::from))
    }

    async fn create(&self, theme: &Theme) -> Result<()> {
        let record = ThemeRecord::from(theme);
        self.collection
            .insert_one(&record)
            .await
            .context("insert theme")?;
        Ok(())
    }

    async fn update(
        &self,
        id: EntityId,
        user_id: EntityId,
        update: &ThemeUpdate,
    ) -> Result<Option<Theme>> {
        let mut set = Document::new();
        if let Some(name) = &update.name {
            set.insert("name", name.clone());
        }
        if let Some(description) = &update.description {
            set.insert("description", description.clone());
        }
        if let Some(tags) = &update.tags {
            set.insert("tags", bson::to_bson(tags).context("serialize theme tags")?);
        }
        if let Some(is_favorite) = update.is_favorite {
            set.insert("is_favorite", is_favorite);
        }
        if let Some(is_public) = update.is_public {
            set.insert("is_public", is_public);
        }
        if let Some(config) = &update.config {
            set.insert(
                "config",
                bson::to_bson(config).context("serialize theme config")?,
            );
        }
        set.insert("updated_at", bson::DateTime::from_chrono(Utc::now()));

        let result = self
            .collection
            .update_one(
                doc! { "id": id.to_string(), "user_id": user_id.to_string() },
                doc! { "$set": set },
            )
            .await
            .context("update theme")?;
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
            .context("delete theme")?;
        Ok(result.deleted_count > 0)
    }

    async fn deactivate_all(&self, user_id: EntityId) -> Result<()> {
        self.collection
            .update_many(
                doc! { "user_id": user_id.to_string() },
                doc! { "$set": { "is_active": false } },
            )
            .await
            .context("deactivate themes")?;
        Ok(())
    }

    async fn activate(&self, id: EntityId, user_id: EntityId) -> Result<Option<Theme>> {
        self.deactivate_all(user_id).await?;
        let result = self
            .collection
            .update_one(
                doc! { "id": id.to_string(), "user_id": user_id.to_string() },
                doc! { "$set": {
                    "is_active": true,
                    "updated_at": bson::DateTime::from_chrono(Utc::now()),
                } },
            )
            .await
            .context("activate theme")?;
        if result.matched_count == 0 {
            return Ok(None);
        }
        self.find_owned(id, user_id).await
    }

    async fn update_owner(&self, user_id: EntityId, owner: &str) -> Result<()> {
        self.collection
            .update_many(
                doc! { "user_id": user_id.to_string() },
                doc! { "$set": { "owner": owner } },
            )
            .await
            .context("update theme owner")?;
        Ok(())
    }

    async fn increment_download_count(&self, id: EntityId, user_id: EntityId) -> Result<()> {
        self.collection
            .update_one(
                doc! { "id": id.to_string(), "user_id": user_id.to_string() },
                doc! { "$inc": { "download_count": 1_i64 } },
            )
            .await
            .context("increment theme download count")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_theme() -> Theme {
        Theme::new(
            EntityId::new(),
            Some("alice".to_string()),
            "Midnight",
            serde_json::json!({ "name": "Midnight" }),
            ThemeSource::Custom,
            true,
        )
    }

    #[test]
    fn record_round_trips_through_theme() {
        let theme = sample_theme();
        let record = ThemeRecord::from(&theme);
        let back: Theme = record.into();
        assert_eq!(back, theme);
    }

    #[test]
    fn theme_filter_doc_scopes_to_user_by_default() {
        let user_id = EntityId::new();
        let filter = ThemeFilter::for_user(user_id);
        assert_eq!(
            theme_filter_doc(&filter),
            doc! { "user_id": user_id.to_string() }
        );
    }

    #[test]
    fn theme_filter_doc_adds_favorite_and_source_clauses() {
        let user_id = EntityId::new();
        let mut filter = ThemeFilter::for_user(user_id);
        filter.favorite = Some(true);
        filter.source = Some(ThemeSource::Preset);
        assert_eq!(
            theme_filter_doc(&filter),
            doc! {
                "user_id": user_id.to_string(),
                "is_favorite": true,
                "source": "preset",
            }
        );
    }

    #[test]
    fn source_str_matches_serde_rename() {
        for source in [
            ThemeSource::Custom,
            ThemeSource::Imported,
            ThemeSource::Preset,
            ThemeSource::Marketplace,
        ] {
            let expected = serde_json::to_value(source).unwrap();
            assert_eq!(expected, Value::String(source_str(source).to_string()));
        }
    }
}
