//! The page owner (`Person`) and their linked social accounts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::id::EntityId;

/// A single social link shown alongside the owner's profile (e.g. a Twitter
/// or GitHub handle), distinct from the freeform [`crate::domain::link::Link`]
/// list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Social {
    pub platform: String,
    pub url: String,
}

/// A user / page owner. In `single` mode there is exactly one seeded person.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Person {
    pub id: EntityId,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub display_name: String,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub socials: Vec<Social>,
    #[serde(default)]
    pub avatar_path: Option<String>,
    #[serde(default)]
    pub cover_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Person {
    /// Builds a new person with a fresh id and matching created/updated
    /// timestamps. Optional fields default to empty/`None`.
    pub fn new(
        username: impl Into<String>,
        email: impl Into<String>,
        password_hash: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: EntityId::new(),
            username: username.into(),
            email: email.into(),
            password_hash: password_hash.into(),
            display_name: display_name.into(),
            bio: None,
            location: None,
            socials: Vec::new(),
            avatar_path: None,
            cover_path: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Partial update for a person's public profile fields. `None` means "leave
/// unchanged"; this mirrors the admin profile PATCH endpoint's semantics
/// without depending on any provider's update-document representation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PersonProfileUpdate {
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub socials: Option<Vec<Social>>,
}

/// Which stored image field an upload should replace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonImageSlot {
    Avatar,
    Cover,
}

/// Update for one image slot. `Some(path)` sets the slot to a newly stored
/// image path; `None` explicitly clears it (removes the stored reference).
/// The caller (storage provider/service) is responsible for deleting the
/// previously stored file itself — this type only carries the intent for
/// the `Person` record's `avatar_path`/`cover_path` field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PersonImageUpdate {
    pub slot: PersonImageSlot,
    #[serde(default)]
    pub path: Option<String>,
}

impl PersonImageUpdate {
    /// Sets `slot` to a newly stored image path.
    pub fn set(slot: PersonImageSlot, path: impl Into<String>) -> Self {
        Self {
            slot,
            path: Some(path.into()),
        }
    }

    /// Clears the stored image path for `slot`.
    pub fn clear(slot: PersonImageSlot) -> Self {
        Self { slot, path: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_defaults_and_matching_timestamps() {
        let person = Person::new("alice", "alice@example.com", "hash", "Alice");
        assert_eq!(person.username, "alice");
        assert_eq!(person.email, "alice@example.com");
        assert_eq!(person.display_name, "Alice");
        assert!(person.bio.is_none());
        assert!(person.socials.is_empty());
        assert_eq!(person.created_at, person.updated_at);
    }

    #[test]
    fn each_new_person_gets_a_unique_id() {
        let a = Person::new("a", "a@example.com", "h", "A");
        let b = Person::new("b", "b@example.com", "h", "B");
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn person_image_update_set_carries_the_new_path() {
        let update = PersonImageUpdate::set(PersonImageSlot::Avatar, "/uploads/a.png");
        assert_eq!(update.slot, PersonImageSlot::Avatar);
        assert_eq!(update.path.as_deref(), Some("/uploads/a.png"));
    }

    #[test]
    fn person_image_update_clear_has_no_path() {
        let update = PersonImageUpdate::clear(PersonImageSlot::Cover);
        assert_eq!(update.slot, PersonImageSlot::Cover);
        assert!(update.path.is_none());
    }
}
