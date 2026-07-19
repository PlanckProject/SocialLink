//! Provider-neutral domain model.
//!
//! Types here describe the application's core entities (person, link
//! groups, links, themes, analytics events) and the inputs/queries used to
//! create, update and filter them — independent of any specific storage
//! provider (Mongo/BSON or otherwise). Nothing in this module imports
//! `bson`, `mongodb`, `ObjectId`, `bson::DateTime`, or `bson::Document`.
//!
//! Identity is represented by [`id::EntityId`] (a UUID wrapper) and
//! timestamps use `chrono::DateTime<Utc>`. Adapters/providers are
//! responsible for mapping these types to and from their storage
//! representation.
//!
//! This module is not yet wired into the crate's module tree (see the
//! integration caveats in the task summary) — add `mod domain;` to
//! `main.rs` to enable it.

pub mod analytics;
pub mod id;
pub mod link;
pub mod link_group;
pub mod person;
pub mod theme;

/// Shared upper bound (in Unicode scalar values) for free-text fields the
/// owner edits: profile bio, group description and link description. Keeping a
/// single constant means every entry point enforces the same limit.
pub const MAX_TEXT_LEN: usize = 256;

pub use analytics::{
    AnalyticsEvent, AnalyticsOverview, DailyClickCount, DailyCount, DateRange, EventKind,
    LinkAnalytics, RequestMetadata, TopLink,
};
pub use id::EntityId;
pub use link::{Link, LinkFilter, LinkInput, LinkOrdering};
pub use link_group::{GroupOrdering, GroupStyle, LinkGroup, LinkGroupInput};
pub use person::{
    Branding, Person, PersonImageSlot, PersonImageUpdate, PersonProfileUpdate, Social,
};
pub use theme::{Theme, ThemeFilter, ThemeSource, ThemeUpdate};
