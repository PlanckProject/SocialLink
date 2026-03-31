//! Provider-neutral database boundary and runtime dispatch.
//!
//! Repository provider enums keep the composite [`Database`] trait
//! extensible: adding another database backend only requires adding variants
//! to these enums and forwarding its repository implementations.

mod mongo;
mod traits;

use anyhow::Result;

use crate::config::{DatabaseConfig, DbProvider};
use crate::domain::{
    EntityId, GroupOrdering, Link, LinkFilter, LinkGroup, LinkGroupInput, LinkInput, LinkOrdering,
    Person, PersonImageUpdate, PersonProfileUpdate, Theme, ThemeFilter, ThemeUpdate,
};

pub use mongo::{
    MongoDatabase, MongoLinkGroupRepository, MongoLinkRepository, MongoPersonRepository,
    MongoThemeRepository,
};
pub use traits::{
    Database, LinkGroupRepository, LinkRepository, PersonRepository, ThemeRepository,
};

macro_rules! forward_repository {
    ($self:expr, $method:ident $(, $argument:expr)* $(,)?) => {
        match $self {
            Self::Mongo(repository) => repository.$method($($argument),*).await,
        }
    };
}

#[derive(Debug, Clone)]
pub enum PersonRepositoryProvider {
    Mongo(MongoPersonRepository),
}

impl PersonRepository for PersonRepositoryProvider {
    async fn find_by_id(&self, id: EntityId) -> Result<Option<Person>> {
        forward_repository!(self, find_by_id, id)
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<Person>> {
        forward_repository!(self, find_by_username, username)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<Person>> {
        forward_repository!(self, find_by_email, email)
    }

    async fn find_first_created(&self) -> Result<Option<Person>> {
        forward_repository!(self, find_first_created)
    }

    async fn find_username_or_email_conflict(
        &self,
        username: &str,
        email: &str,
        exclude_id: Option<EntityId>,
    ) -> Result<Option<Person>> {
        forward_repository!(
            self,
            find_username_or_email_conflict,
            username,
            email,
            exclude_id
        )
    }

    async fn insert(&self, person: &Person) -> Result<()> {
        forward_repository!(self, insert, person)
    }

    async fn update_profile(
        &self,
        id: EntityId,
        update: &PersonProfileUpdate,
    ) -> Result<Option<Person>> {
        forward_repository!(self, update_profile, id, update)
    }

    async fn update_password(&self, id: EntityId, password_hash: &str) -> Result<()> {
        forward_repository!(self, update_password, id, password_hash)
    }

    async fn update_image(&self, id: EntityId, update: &PersonImageUpdate) -> Result<()> {
        forward_repository!(self, update_image, id, update)
    }

    async fn cleanup_legacy_fields(&self) -> Result<()> {
        forward_repository!(self, cleanup_legacy_fields)
    }
}

#[derive(Debug, Clone)]
pub enum LinkGroupRepositoryProvider {
    Mongo(MongoLinkGroupRepository),
}

impl LinkGroupRepository for LinkGroupRepositoryProvider {
    async fn list(&self, user_id: EntityId) -> Result<Vec<LinkGroup>> {
        forward_repository!(self, list, user_id)
    }

    async fn list_active(&self, user_id: EntityId) -> Result<Vec<LinkGroup>> {
        forward_repository!(self, list_active, user_id)
    }

    async fn count(&self, user_id: EntityId) -> Result<u64> {
        forward_repository!(self, count, user_id)
    }

    async fn create(&self, user_id: EntityId, input: &LinkGroupInput) -> Result<LinkGroup> {
        forward_repository!(self, create, user_id, input)
    }

    async fn update(
        &self,
        id: EntityId,
        user_id: EntityId,
        input: &LinkGroupInput,
    ) -> Result<Option<LinkGroup>> {
        forward_repository!(self, update, id, user_id, input)
    }

    async fn delete(&self, id: EntityId, user_id: EntityId) -> Result<bool> {
        forward_repository!(self, delete, id, user_id)
    }

    async fn reorder(&self, user_id: EntityId, ordering: &GroupOrdering) -> Result<()> {
        forward_repository!(self, reorder, user_id, ordering)
    }
}

#[derive(Debug, Clone)]
pub enum LinkRepositoryProvider {
    Mongo(MongoLinkRepository),
}

impl LinkRepository for LinkRepositoryProvider {
    async fn list(&self, user_id: EntityId) -> Result<Vec<Link>> {
        forward_repository!(self, list, user_id)
    }

    async fn list_public(&self, user_id: EntityId) -> Result<Vec<Link>> {
        forward_repository!(self, list_public, user_id)
    }

    async fn count(&self, filter: &LinkFilter) -> Result<u64> {
        forward_repository!(self, count, filter)
    }

    async fn get(&self, id: EntityId, user_id: EntityId) -> Result<Option<Link>> {
        forward_repository!(self, get, id, user_id)
    }

    async fn get_by_id(&self, id: EntityId) -> Result<Option<Link>> {
        forward_repository!(self, get_by_id, id)
    }

    async fn create(&self, user_id: EntityId, input: &LinkInput) -> Result<Link> {
        forward_repository!(self, create, user_id, input)
    }

    async fn update(
        &self,
        id: EntityId,
        user_id: EntityId,
        input: &LinkInput,
    ) -> Result<Option<Link>> {
        forward_repository!(self, update, id, user_id, input)
    }

    async fn delete(&self, id: EntityId, user_id: EntityId) -> Result<bool> {
        forward_repository!(self, delete, id, user_id)
    }

    async fn reorder(&self, user_id: EntityId, ordering: &LinkOrdering) -> Result<()> {
        forward_repository!(self, reorder, user_id, ordering)
    }

    async fn unassign_group(&self, user_id: EntityId, group_id: EntityId) -> Result<()> {
        forward_repository!(self, unassign_group, user_id, group_id)
    }
}

#[derive(Debug, Clone)]
pub enum ThemeRepositoryProvider {
    Mongo(MongoThemeRepository),
}

impl ThemeRepository for ThemeRepositoryProvider {
    async fn list(&self, filter: &ThemeFilter) -> Result<Vec<Theme>> {
        forward_repository!(self, list, filter)
    }

    async fn count_all(&self, user_id: EntityId) -> Result<u64> {
        forward_repository!(self, count_all, user_id)
    }

    async fn count_saveable(&self, user_id: EntityId) -> Result<u64> {
        forward_repository!(self, count_saveable, user_id)
    }

    async fn count_presets(&self, user_id: EntityId) -> Result<u64> {
        forward_repository!(self, count_presets, user_id)
    }

    async fn active(&self, user_id: EntityId) -> Result<Option<Theme>> {
        forward_repository!(self, active, user_id)
    }

    async fn by_id(&self, id: EntityId, user_id: EntityId) -> Result<Option<Theme>> {
        forward_repository!(self, by_id, id, user_id)
    }

    async fn by_preset_name(&self, user_id: EntityId, name: &str) -> Result<Option<Theme>> {
        forward_repository!(self, by_preset_name, user_id, name)
    }

    async fn create(&self, theme: &Theme) -> Result<()> {
        forward_repository!(self, create, theme)
    }

    async fn update(
        &self,
        id: EntityId,
        user_id: EntityId,
        update: &ThemeUpdate,
    ) -> Result<Option<Theme>> {
        forward_repository!(self, update, id, user_id, update)
    }

    async fn delete(&self, id: EntityId, user_id: EntityId) -> Result<bool> {
        forward_repository!(self, delete, id, user_id)
    }

    async fn deactivate_all(&self, user_id: EntityId) -> Result<()> {
        forward_repository!(self, deactivate_all, user_id)
    }

    async fn activate(&self, id: EntityId, user_id: EntityId) -> Result<Option<Theme>> {
        forward_repository!(self, activate, id, user_id)
    }

    async fn update_owner(&self, user_id: EntityId, owner: &str) -> Result<()> {
        forward_repository!(self, update_owner, user_id, owner)
    }

    async fn increment_download_count(&self, id: EntityId, user_id: EntityId) -> Result<()> {
        forward_repository!(self, increment_download_count, id, user_id)
    }
}

#[derive(Debug, Clone)]
pub enum DatabaseProvider {
    Mongo {
        database: MongoDatabase,
        people: PersonRepositoryProvider,
        groups: LinkGroupRepositoryProvider,
        links: LinkRepositoryProvider,
        themes: ThemeRepositoryProvider,
    },
}

impl DatabaseProvider {
    pub async fn connect(config: &DatabaseConfig) -> Result<Self> {
        match config.provider {
            DbProvider::Mongo => {
                let database = MongoDatabase::connect(config).await?;
                Ok(Self::Mongo {
                    people: PersonRepositoryProvider::Mongo(database.people().clone()),
                    groups: LinkGroupRepositoryProvider::Mongo(database.groups().clone()),
                    links: LinkRepositoryProvider::Mongo(database.links().clone()),
                    themes: ThemeRepositoryProvider::Mongo(database.themes().clone()),
                    database,
                })
            }
        }
    }
}

impl Database for DatabaseProvider {
    type People = PersonRepositoryProvider;
    type Groups = LinkGroupRepositoryProvider;
    type Links = LinkRepositoryProvider;
    type Themes = ThemeRepositoryProvider;

    fn people(&self) -> &Self::People {
        match self {
            Self::Mongo { people, .. } => people,
        }
    }

    fn groups(&self) -> &Self::Groups {
        match self {
            Self::Mongo { groups, .. } => groups,
        }
    }

    fn links(&self) -> &Self::Links {
        match self {
            Self::Mongo { links, .. } => links,
        }
    }

    fn themes(&self) -> &Self::Themes {
        match self {
            Self::Mongo { themes, .. } => themes,
        }
    }

    async fn bootstrap(&self) -> Result<()> {
        match self {
            Self::Mongo { database, .. } => database.bootstrap().await,
        }
    }
}
