//! Mongo-backed [`PersonRepository`] implementation.

use anyhow::{Context, Result};
use bson::{Document, doc};
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use mongodb::options::IndexOptions;
use mongodb::{Collection, Database as MongoDb, IndexModel};
use serde::{Deserialize, Serialize};

use crate::domain::{
    Branding, EntityId, Person, PersonImageSlot, PersonImageUpdate, PersonProfileUpdate, Social,
};

use crate::providers::database::traits::PersonRepository;

/// Local default for a record that predates the `ungrouped_position` field.
/// Mirrors [`crate::domain::person::default_ungrouped_position`]; kept local so
/// the persistence layer owns its own serde defaults.
fn default_ungrouped_position() -> i32 {
    i32::MAX
}

/// Persistence record for [`Person`]. Mirrors the domain type field for
/// field, except timestamps go through bson's chrono helpers so they are
/// stored as native BSON dates (sortable/comparable in Mongo) rather than
/// via chrono's own string `Serialize` impl.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersonRecord {
    id: EntityId,
    username: String,
    email: String,
    password_hash: String,
    display_name: String,
    #[serde(default)]
    bio: Option<String>,
    #[serde(default)]
    location: Option<String>,
    #[serde(default)]
    socials: Vec<Social>,
    #[serde(default)]
    avatar_path: Option<String>,
    #[serde(default)]
    cover_path: Option<String>,
    #[serde(default)]
    branding: Branding,
    #[serde(default = "default_ungrouped_position")]
    ungrouped_position: i32,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    created_at: DateTime<Utc>,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    updated_at: DateTime<Utc>,
}

impl From<&Person> for PersonRecord {
    fn from(person: &Person) -> Self {
        Self {
            id: person.id,
            username: person.username.clone(),
            email: person.email.clone(),
            password_hash: person.password_hash.clone(),
            display_name: person.display_name.clone(),
            bio: person.bio.clone(),
            location: person.location.clone(),
            socials: person.socials.clone(),
            avatar_path: person.avatar_path.clone(),
            cover_path: person.cover_path.clone(),
            branding: person.branding.clone(),
            ungrouped_position: person.ungrouped_position,
            created_at: person.created_at,
            updated_at: person.updated_at,
        }
    }
}

impl From<PersonRecord> for Person {
    fn from(record: PersonRecord) -> Self {
        Self {
            id: record.id,
            username: record.username,
            email: record.email,
            password_hash: record.password_hash,
            display_name: record.display_name,
            bio: record.bio,
            location: record.location,
            socials: record.socials,
            avatar_path: record.avatar_path,
            cover_path: record.cover_path,
            branding: record.branding,
            ungrouped_position: record.ungrouped_position,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

/// Builds the `{"$or": [{"username": ...}, {"email": ...}]}` conflict
/// filter, optionally excluding one id (used when changing an existing
/// person's username).
fn conflict_filter(username: &str, email: &str, exclude_id: Option<EntityId>) -> Document {
    let or_clause = vec![doc! { "username": username }, doc! { "email": email }];
    match exclude_id {
        Some(id) => doc! {
            "$and": [
                doc! { "$or": or_clause },
                doc! { "id": { "$ne": id.to_string() } },
            ],
        },
        None => doc! { "$or": or_clause },
    }
}

#[derive(Debug, Clone)]
pub struct MongoPersonRepository {
    collection: Collection<PersonRecord>,
}

impl MongoPersonRepository {
    pub(super) fn new(database: MongoDb) -> Self {
        Self {
            collection: database.collection("persons"),
        }
    }

    pub(super) async fn ensure_indexes(&self) -> Result<()> {
        let unique = IndexOptions::builder().unique(true).build();
        self.collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "id": 1 })
                    .options(unique.clone())
                    .build(),
            )
            .await
            .context("create persons id index")?;
        self.collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "username": 1 })
                    .options(unique.clone())
                    .build(),
            )
            .await
            .context("create persons username index")?;
        self.collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "email": 1 })
                    .options(unique)
                    .build(),
            )
            .await
            .context("create persons email index")?;
        Ok(())
    }

    async fn find_one(&self, filter: Document) -> Result<Option<Person>> {
        let record = self
            .collection
            .find_one(filter)
            .await
            .context("find person")?;
        Ok(record.map(Person::from))
    }
}

impl PersonRepository for MongoPersonRepository {
    async fn find_by_id(&self, id: EntityId) -> Result<Option<Person>> {
        self.find_one(doc! { "id": id.to_string() }).await
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<Person>> {
        self.find_one(doc! { "username": username }).await
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<Person>> {
        self.find_one(doc! { "email": email }).await
    }

    async fn find_first_created(&self) -> Result<Option<Person>> {
        let mut cursor = self
            .collection
            .find(doc! {})
            .sort(doc! { "created_at": 1 })
            .limit(1)
            .await
            .context("find first-created person")?;
        let record = cursor
            .try_next()
            .await
            .context("read first-created person")?;
        Ok(record.map(Person::from))
    }

    async fn find_username_or_email_conflict(
        &self,
        username: &str,
        email: &str,
        exclude_id: Option<EntityId>,
    ) -> Result<Option<Person>> {
        self.find_one(conflict_filter(username, email, exclude_id))
            .await
    }

    async fn insert(&self, person: &Person) -> Result<()> {
        let record = PersonRecord::from(person);
        self.collection
            .insert_one(&record)
            .await
            .context("insert person")?;
        Ok(())
    }

    async fn update_profile(
        &self,
        id: EntityId,
        update: &PersonProfileUpdate,
    ) -> Result<Option<Person>> {
        let mut set = Document::new();
        if let Some(username) = &update.username {
            set.insert("username", username.clone());
        }
        if let Some(display_name) = &update.display_name {
            set.insert("display_name", display_name.clone());
        }
        if let Some(bio) = &update.bio {
            set.insert("bio", bio.clone());
        }
        if let Some(location) = &update.location {
            set.insert("location", location.clone());
        }
        if let Some(socials) = &update.socials {
            set.insert(
                "socials",
                bson::to_bson(socials).context("serialize socials")?,
            );
        }
        if let Some(branding) = &update.branding {
            set.insert(
                "branding",
                bson::to_bson(branding).context("serialize branding")?,
            );
        }
        if let Some(position) = update.ungrouped_position {
            set.insert("ungrouped_position", position);
        }
        set.insert("updated_at", bson::DateTime::from_chrono(Utc::now()));

        let result = self
            .collection
            .update_one(doc! { "id": id.to_string() }, doc! { "$set": set })
            .await
            .context("update person profile")?;
        if result.matched_count == 0 {
            return Ok(None);
        }
        self.find_by_id(id).await
    }

    async fn update_password(&self, id: EntityId, password_hash: &str) -> Result<()> {
        self.collection
            .update_one(
                doc! { "id": id.to_string() },
                doc! { "$set": {
                    "password_hash": password_hash,
                    "updated_at": bson::DateTime::from_chrono(Utc::now()),
                } },
            )
            .await
            .context("update person password")?;
        Ok(())
    }

    async fn update_image(&self, id: EntityId, update: &PersonImageUpdate) -> Result<()> {
        let field = match update.slot {
            PersonImageSlot::Avatar => "avatar_path",
            PersonImageSlot::Cover => "cover_path",
        };
        let mut set = doc! { "updated_at": bson::DateTime::from_chrono(Utc::now()) };
        let update_document = match &update.path {
            Some(path) => {
                set.insert(field, path);
                doc! { "$set": set }
            }
            None => {
                let mut unset = Document::new();
                unset.insert(field, "");
                doc! { "$set": set, "$unset": unset }
            }
        };
        self.collection
            .update_one(doc! { "id": id.to_string() }, update_document)
            .await
            .context("update person image")?;
        Ok(())
    }

    async fn cleanup_legacy_fields(&self) -> Result<()> {
        self.collection
            .update_many(
                doc! { "tagline": { "$exists": true } },
                doc! { "$unset": { "tagline": "" } },
            )
            .await
            .context("cleanup legacy person fields")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_person() -> Person {
        Person::new("alice", "alice@example.com", "hash", "Alice")
    }

    #[test]
    fn record_round_trips_through_person() {
        let person = sample_person();
        let record = PersonRecord::from(&person);
        let back: Person = record.into();
        assert_eq!(back, person);
    }

    #[test]
    fn conflict_filter_without_exclude_is_a_plain_or() {
        let filter = conflict_filter("alice", "alice@example.com", None);
        assert_eq!(
            filter,
            doc! { "$or": [
                { "username": "alice" },
                { "email": "alice@example.com" },
            ] }
        );
    }

    #[test]
    fn conflict_filter_with_exclude_adds_ne_id_clause() {
        let id = EntityId::new();
        let filter = conflict_filter("alice", "alice@example.com", Some(id));
        assert_eq!(
            filter,
            doc! { "$and": [
                { "$or": [
                    { "username": "alice" },
                    { "email": "alice@example.com" },
                ] },
                { "id": { "$ne": id.to_string() } },
            ] }
        );
    }
}
