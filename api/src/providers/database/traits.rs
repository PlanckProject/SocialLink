//! Provider-neutral, native async repository traits.
//!
//! These traits describe every database operation the application needs for
//! people, link groups, links and themes, plus a composite [`Database`]
//! trait that groups the four repositories together with a `bootstrap`
//! hook for index creation. Nothing here uses `async_trait`, `dyn` trait
//! objects, or imports any Mongo/BSON type — only [`crate::domain`] types
//! and [`anyhow::Result`] cross this boundary, so a future non-Mongo
//! provider can implement the exact same surface.
//!
//! Ownership enforcement, filtering, ordering, uniqueness and mutation
//! documents are all adapter responsibilities: every mutation that targets
//! a single owned entity takes both the entity id and the owner's
//! `user_id`, and returns `None`/`false` when the two don't match a stored
//! document (the caller cannot distinguish "not found" from "not owned",
//! matching existing handler behavior).

use anyhow::Result;

use crate::domain::{
    EntityId, GroupOrdering, Link, LinkFilter, LinkGroup, LinkGroupInput, LinkInput, LinkOrdering,
    Person, PersonImageUpdate, PersonProfileUpdate, Theme, ThemeFilter, ThemeUpdate,
};

/// Persistence operations for [`Person`] (the page owner / account).
pub trait PersonRepository: Send + Sync {
    /// Looks up a person by their domain id.
    async fn find_by_id(&self, id: EntityId) -> Result<Option<Person>>;

    /// Looks up a person by their unique username.
    async fn find_by_username(&self, username: &str) -> Result<Option<Person>>;

    /// Looks up a person by their unique email.
    async fn find_by_email(&self, email: &str) -> Result<Option<Person>>;

    /// Returns the earliest-created person. Used as the single-mode public
    /// page fallback when no admin username match is found.
    async fn find_first_created(&self) -> Result<Option<Person>>;

    /// Returns a person whose username or email conflicts with the given
    /// values, excluding `exclude_id` when set. Used both for signup
    /// (`exclude_id = None`) and for a profile's username change
    /// (`exclude_id = Some(self)`).
    async fn find_username_or_email_conflict(
        &self,
        username: &str,
        email: &str,
        exclude_id: Option<EntityId>,
    ) -> Result<Option<Person>>;

    /// Inserts a brand-new person.
    async fn insert(&self, person: &Person) -> Result<()>;

    /// Applies a partial profile update, returning the updated person, or
    /// `None` when `id` does not exist.
    async fn update_profile(
        &self,
        id: EntityId,
        update: &PersonProfileUpdate,
    ) -> Result<Option<Person>>;

    /// Sets a new password hash.
    async fn update_password(&self, id: EntityId, password_hash: &str) -> Result<()>;

    /// Sets a newly stored avatar/cover image path.
    async fn update_image(&self, id: EntityId, update: &PersonImageUpdate) -> Result<()>;

    /// Drops legacy fields (e.g. the removed `tagline`) from every person
    /// document. Safe to call on every boot; a no-op once cleaned.
    async fn cleanup_legacy_fields(&self) -> Result<()>;
}

/// Persistence operations for [`LinkGroup`].
pub trait LinkGroupRepository: Send + Sync {
    /// All of an owner's groups, ordered by `sort_order`.
    async fn list(&self, user_id: EntityId) -> Result<Vec<LinkGroup>>;

    /// An owner's active groups, ordered by `sort_order` (public page).
    async fn list_active(&self, user_id: EntityId) -> Result<Vec<LinkGroup>>;

    /// Number of groups owned by `user_id`. Used to append new groups at
    /// the end of the list.
    async fn count(&self, user_id: EntityId) -> Result<u64>;

    /// Creates a new group owned by `user_id`, appended at the end of the
    /// owner's current list (the adapter computes `sort_order`).
    async fn create(&self, user_id: EntityId, input: &LinkGroupInput) -> Result<LinkGroup>;

    /// Updates an owned group's fields, returning `None` if `id` is not
    /// owned by `user_id`.
    async fn update(
        &self,
        id: EntityId,
        user_id: EntityId,
        input: &LinkGroupInput,
    ) -> Result<Option<LinkGroup>>;

    /// Deletes an owned group, returning whether a document was removed.
    async fn delete(&self, id: EntityId, user_id: EntityId) -> Result<bool>;

    /// Applies a new `sort_order` for each id in `ordering`, restricted to
    /// groups owned by `user_id`.
    async fn reorder(&self, user_id: EntityId, ordering: &GroupOrdering) -> Result<()>;
}

/// Persistence operations for [`Link`].
pub trait LinkRepository: Send + Sync {
    /// All of an owner's links, ordered by `sort_order` (admin list).
    async fn list(&self, user_id: EntityId) -> Result<Vec<Link>>;

    /// An owner's active, non-expired links, ordered by `sort_order`
    /// (public page).
    async fn list_public(&self, user_id: EntityId) -> Result<Vec<Link>>;

    /// Number of links matching `filter`. Used to append new links at the
    /// end of their (group-scoped) list.
    async fn count(&self, filter: &LinkFilter) -> Result<u64>;

    /// Looks up a single owned link.
    async fn get(&self, id: EntityId, user_id: EntityId) -> Result<Option<Link>>;

    /// Looks up a link by id without an ownership predicate. Used only by
    /// the public redirect flow, which records analytics for the link owner.
    async fn get_by_id(&self, id: EntityId) -> Result<Option<Link>>;

    /// Creates a new link owned by `user_id`, appended at the end of its
    /// target group's (or ungrouped) list (the adapter computes
    /// `sort_order`).
    async fn create(&self, user_id: EntityId, input: &LinkInput) -> Result<Link>;

    /// Updates an owned link's fields, returning `None` if `id` is not
    /// owned by `user_id`.
    async fn update(
        &self,
        id: EntityId,
        user_id: EntityId,
        input: &LinkInput,
    ) -> Result<Option<Link>>;

    /// Deletes an owned link, returning whether a document was removed.
    async fn delete(&self, id: EntityId, user_id: EntityId) -> Result<bool>;

    /// Applies a new `sort_order` (and target `group_id`) for each id in
    /// `ordering`, restricted to links owned by `user_id`.
    async fn reorder(&self, user_id: EntityId, ordering: &LinkOrdering) -> Result<()>;

    /// Clears `group_id` on every one of `user_id`'s links currently
    /// assigned to `group_id`. Called when a group is deleted so its links
    /// fall back to the ungrouped section instead of being orphaned.
    async fn unassign_group(&self, user_id: EntityId, group_id: EntityId) -> Result<()>;
}

/// Persistence operations for [`Theme`], including the per-owner theme
/// library (favorites, presets, marketplace metadata).
pub trait ThemeRepository: Send + Sync {
    /// Themes matching `filter`, ordered favorites-first then by creation
    /// time.
    async fn list(&self, filter: &ThemeFilter) -> Result<Vec<Theme>>;

    /// Total number of themes owned by `user_id`, regardless of source.
    /// Used to decide whether a default theme still needs seeding.
    async fn count_all(&self, user_id: EntityId) -> Result<u64>;

    /// Number of `custom`/`imported` themes owned by `user_id`. Used to
    /// enforce the saved-theme capacity limit.
    async fn count_saveable(&self, user_id: EntityId) -> Result<u64>;

    /// Number of `preset` themes owned by `user_id`. Used to enforce the
    /// preset-slot capacity limit.
    async fn count_presets(&self, user_id: EntityId) -> Result<u64>;

    /// The owner's currently active theme, if any.
    async fn active(&self, user_id: EntityId) -> Result<Option<Theme>>;

    /// Looks up a single owned theme.
    async fn by_id(&self, id: EntityId, user_id: EntityId) -> Result<Option<Theme>>;

    /// Looks up an owned preset slot by name (`source == preset`).
    async fn by_preset_name(&self, user_id: EntityId, name: &str) -> Result<Option<Theme>>;

    /// Inserts a fully constructed theme (see [`Theme::new`]).
    async fn create(&self, theme: &Theme) -> Result<()>;

    /// Applies a partial update to an owned theme, returning `None` if `id`
    /// is not owned by `user_id`.
    async fn update(
        &self,
        id: EntityId,
        user_id: EntityId,
        update: &ThemeUpdate,
    ) -> Result<Option<Theme>>;

    /// Deletes an owned theme, returning whether a document was removed.
    async fn delete(&self, id: EntityId, user_id: EntityId) -> Result<bool>;

    /// Marks every theme owned by `user_id` as inactive.
    async fn deactivate_all(&self, user_id: EntityId) -> Result<()>;

    /// Deactivates every other theme owned by `user_id` and marks `id` as
    /// the active one, returning the updated theme (or `None` if `id` is
    /// not owned by `user_id`).
    async fn activate(&self, id: EntityId, user_id: EntityId) -> Result<Option<Theme>>;

    /// Updates the denormalized `owner` (username) field on every theme
    /// owned by `user_id`. Called when an owner renames their account.
    async fn update_owner(&self, user_id: EntityId, owner: &str) -> Result<()>;

    /// Atomically increments an owned theme's `download_count` by one.
    async fn increment_download_count(&self, id: EntityId, user_id: EntityId) -> Result<()>;
}

/// Composite database boundary: one repository per entity, plus a
/// `bootstrap` hook that a provider uses to create any indexes it relies
/// on. Safe to call `bootstrap` on every process start.
pub trait Database: Send + Sync {
    type People: PersonRepository;
    type Groups: LinkGroupRepository;
    type Links: LinkRepository;
    type Themes: ThemeRepository;

    fn people(&self) -> &Self::People;
    fn groups(&self) -> &Self::Groups;
    fn links(&self) -> &Self::Links;
    fn themes(&self) -> &Self::Themes;

    /// Creates/ensures every index the provider relies on. Idempotent.
    async fn bootstrap(&self) -> Result<()>;
}
