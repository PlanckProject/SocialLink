//! Provider-neutral entity identifier.
//!
//! [`EntityId`] wraps a [`uuid::Uuid`] so domain types never depend on a
//! specific storage provider's identifier type (e.g. Mongo's `ObjectId`).
//! It serializes as a plain string (`"<uuid>"`) so it round-trips cleanly
//! through JSON APIs and any cache/provider mapping layer.
//!
//! Only the `uuid` crate's default + `v4` features are required; the
//! string (de)serialization below is implemented by hand so the `serde`
//! feature of the `uuid` crate is not needed.

use std::fmt;
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// A strongly typed, provider-neutral identifier for a persisted domain
/// entity. Backed by a UUID (v4 when freshly generated), it is `Copy`,
/// hashable and orderable so it can be used directly as a map key or in
/// sorted collections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(Uuid);

impl EntityId {
    /// Generates a new random (v4) entity id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Wraps an existing [`Uuid`] as an [`EntityId`].
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the underlying [`Uuid`].
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Parses an [`EntityId`] from its canonical string representation.
    pub fn parse(value: &str) -> Result<Self, uuid::Error> {
        Uuid::parse_str(value).map(Self)
    }

    /// Returns the hyphenated, lowercase string representation.
    pub fn to_hyphenated_string(self) -> String {
        self.0.to_string()
    }
}

impl Default for EntityId {
    /// Defaults to a freshly generated random id, matching the behavior of
    /// `EntityId::new()`. Useful for `#[derive(Default)]` update/input types
    /// that embed an id-bearing field, though most constructors call `new()`
    /// explicitly for clarity.
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl FromStr for EntityId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl From<Uuid> for EntityId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<EntityId> for Uuid {
    fn from(id: EntityId) -> Self {
        id.0
    }
}

impl AsRef<Uuid> for EntityId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

impl Serialize for EntityId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for EntityId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EntityIdVisitor;

        impl Visitor<'_> for EntityIdVisitor {
            type Value = EntityId;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a UUID string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                EntityId::parse(v).map_err(|e| E::custom(format!("invalid EntityId: {e}")))
            }
        }

        deserializer.deserialize_str(EntityIdVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_generates_distinct_v4_ids() {
        let a = EntityId::new();
        let b = EntityId::new();
        assert_ne!(a, b);
        assert_eq!(a.as_uuid().get_version_num(), 4);
    }

    #[test]
    fn display_and_parse_round_trip() {
        let id = EntityId::new();
        let text = id.to_string();
        let parsed = EntityId::parse(&text).expect("valid uuid string parses");
        assert_eq!(id, parsed);
    }

    #[test]
    fn from_str_matches_parse() {
        let id = EntityId::new();
        let text = id.to_string();
        let parsed: EntityId = text.parse().expect("FromStr should parse a valid uuid");
        assert_eq!(id, parsed);
    }

    #[test]
    fn parse_rejects_invalid_input() {
        assert!(EntityId::parse("not-a-uuid").is_err());
    }

    #[test]
    fn serde_round_trips_as_plain_string() {
        let id = EntityId::new();
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, format!("\"{id}\""));

        let back: EntityId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, back);
    }

    #[test]
    fn serde_rejects_invalid_string() {
        let err = serde_json::from_str::<EntityId>("\"nope\"");
        assert!(err.is_err());
    }

    #[test]
    fn uuid_conversions_round_trip() {
        let uuid = Uuid::new_v4();
        let id: EntityId = uuid.into();
        let back: Uuid = id.into();
        assert_eq!(uuid, back);
        assert_eq!(id.as_uuid(), uuid);
    }

    #[test]
    fn ordering_and_hash_are_consistent_with_uuid() {
        use std::collections::HashSet;

        let a = EntityId::new();
        let b = EntityId::new();
        let (lo, hi) = if a.as_uuid() < b.as_uuid() {
            (a, b)
        } else {
            (b, a)
        };
        assert!(lo <= hi);

        let mut set = HashSet::new();
        set.insert(a);
        set.insert(b);
        assert_eq!(set.len(), 2);
    }
}
