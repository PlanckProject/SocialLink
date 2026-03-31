//! MongoDB implementation of the [`super::traits`] repositories.
//!
//! Uses a fresh schema keyed by the domain [`crate::domain::EntityId`]
//! (stored as a plain UUID string, via `EntityId`'s own `Serialize`/
//! `Deserialize`) rather than Mongo's `ObjectId`. Domain `chrono` timestamps
//! do not round-trip through the generic BSON serializer as native BSON
//! dates without an explicit `#[serde(with = "...")]` helper, so each
//! sub-module stores a private persistence record (`*Record`) alongside a
//! mapping to/from the corresponding domain type instead of storing domain
//! structs directly.

mod link;
mod link_group;
mod person;
mod theme;

use std::path::PathBuf;

use anyhow::{Context, Result};
use bson::doc;
use mongodb::Client;
use mongodb::options::{ClientOptions, Credential, ServerAddress, Tls, TlsOptions};

use crate::config::DatabaseConfig;

pub use link::MongoLinkRepository;
pub use link_group::MongoLinkGroupRepository;
pub use person::MongoPersonRepository;
pub use theme::MongoThemeRepository;

use super::traits::Database;

/// MongoDB-backed [`Database`]. Holds one repository per entity, each
/// sharing the same underlying [`mongodb::Database`] handle.
#[derive(Debug, Clone)]
pub struct MongoDatabase {
    people: MongoPersonRepository,
    groups: MongoLinkGroupRepository,
    links: MongoLinkRepository,
    themes: MongoThemeRepository,
}

impl MongoDatabase {
    /// Connects to MongoDB using the typed [`DatabaseConfig`].
    ///
    /// `connection_string`, when non-empty, takes full precedence: the
    /// discrete `host`/`port`/`db`/`username`/`password`/`certificate`
    /// fields are ignored entirely and the selected database is the one
    /// encoded in the URI itself (connecting fails clearly if the URI has
    /// no default database). Only when `connection_string` is empty do the
    /// discrete fields apply, with `db` selecting the database. Pings the
    /// server once connected so startup fails fast if Mongo is unreachable.
    /// Never logs the connection string, username, password or certificate
    /// path.
    pub async fn connect(config: &DatabaseConfig) -> Result<Self> {
        let (client, db_name) = build_client(config).await.context("build MongoDB client")?;
        let database = client.database(&db_name);
        database
            .run_command(doc! { "ping": 1 })
            .await
            .context("ping MongoDB")?;

        Ok(Self {
            people: MongoPersonRepository::new(database.clone()),
            groups: MongoLinkGroupRepository::new(database.clone()),
            links: MongoLinkRepository::new(database.clone()),
            themes: MongoThemeRepository::new(database),
        })
    }
}

impl Database for MongoDatabase {
    type People = MongoPersonRepository;
    type Groups = MongoLinkGroupRepository;
    type Links = MongoLinkRepository;
    type Themes = MongoThemeRepository;

    fn people(&self) -> &Self::People {
        &self.people
    }

    fn groups(&self) -> &Self::Groups {
        &self.groups
    }

    fn links(&self) -> &Self::Links {
        &self.links
    }

    fn themes(&self) -> &Self::Themes {
        &self.themes
    }

    async fn bootstrap(&self) -> Result<()> {
        self.people
            .ensure_indexes()
            .await
            .context("ensure person indexes")?;
        self.groups
            .ensure_indexes()
            .await
            .context("ensure link group indexes")?;
        self.links
            .ensure_indexes()
            .await
            .context("ensure link indexes")?;
        self.themes
            .ensure_indexes()
            .await
            .context("ensure theme indexes")?;
        Ok(())
    }
}

/// Builds a [`Client`] and resolves the database name to use with it.
///
/// A non-empty `connection_string` takes full precedence over every
/// discrete field: `host`/`port`/`db`/`username`/`password`/`certificate`
/// are all ignored, the client is built from the URI verbatim, and the
/// database is the one encoded in the URI's own path
/// (`ClientOptions::default_database`) — connecting fails clearly if the
/// URI does not specify one. Only when `connection_string` is empty do the
/// discrete fields apply: `db` selects the database, `certificate` is
/// treated as a CA certificate path (`TlsOptions::ca_file_path`, not a
/// client certificate/key), and `username`/`password` build a credential
/// when both are non-empty. Never logs the resolved credentials/URI.
async fn build_client(config: &DatabaseConfig) -> Result<(Client, String)> {
    if let Some(connection_string) = config
        .connection_string
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let options = ClientOptions::parse(connection_string)
            .await
            .context("parse MongoDB connection string")?;
        let db_name = options
            .default_database
            .clone()
            .map(|name| name.trim().to_string())
            .filter(|name| !name.is_empty())
            .context(
                "database.config.connection_string must include a default database, e.g. mongodb://host/dbname",
            )?;
        let client =
            Client::with_options(options).context("build MongoDB client from connection string")?;
        return Ok((client, db_name));
    }

    let db_name = config
        .db
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .context("database.config.db must be set")?
        .to_string();

    let host = config
        .host
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("localhost");
    let port = config.port.filter(|port| *port != 0);

    let credential: Option<Credential> = match (
        config.username.as_deref().filter(|v| !v.is_empty()),
        config.password.as_deref().filter(|v| !v.is_empty()),
    ) {
        (Some(username), Some(password)) => Some(
            Credential::builder()
                .username(username.to_string())
                .password(password.to_string())
                .build(),
        ),
        _ => None,
    };

    let tls: Option<Tls> = config
        .certificate
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|certificate| {
            Tls::Enabled(
                TlsOptions::builder()
                    .ca_file_path(PathBuf::from(certificate))
                    .build(),
            )
        });

    let options = ClientOptions::builder()
        .hosts(vec![ServerAddress::Tcp {
            host: host.to_string(),
            port,
        }])
        .credential(credential)
        .tls(tls)
        .build();

    let client = Client::with_options(options)
        .context("build MongoDB client from discrete connection settings")?;
    Ok((client, db_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> DatabaseConfig {
        DatabaseConfig {
            provider: crate::config::DbProvider::Mongo,
            host: None,
            port: None,
            db: None,
            certificate: None,
            username: None,
            password: None,
            connection_string: None,
        }
    }

    #[tokio::test]
    async fn connect_rejects_missing_db_name() {
        let config = base_config();
        let error = MongoDatabase::connect(&config)
            .await
            .expect_err("missing db name should fail before any network call");
        assert!(format!("{error:#}").contains("database.config.db"));
    }

    #[tokio::test]
    async fn connect_rejects_blank_db_name() {
        let mut config = base_config();
        config.db = Some("   ".to_string());
        let error = MongoDatabase::connect(&config)
            .await
            .expect_err("blank db name should fail before any network call");
        assert!(format!("{error:#}").contains("database.config.db"));
    }

    #[tokio::test]
    async fn build_client_accepts_discrete_settings_without_a_live_server() {
        // Building the client only parses/validates options; it does not
        // perform any network I/O, so this exercises the non-connection-string
        // path (host/port/credential/tls resolution) without a live Mongo
        // service.
        let mut config = base_config();
        config.db = Some("discrete_db".to_string());
        config.host = Some("127.0.0.1".to_string());
        config.port = Some(27017);
        config.username = Some("user".to_string());
        config.password = Some("pass".to_string());
        let (_client, db_name) = build_client(&config)
            .await
            .expect("client options are valid");
        assert_eq!(db_name, "discrete_db");
    }

    #[tokio::test]
    async fn build_client_rejects_missing_db_in_discrete_mode() {
        let config = base_config();
        let error = build_client(&config)
            .await
            .expect_err("missing db name should fail before any network call");
        assert!(error.to_string().contains("database.config.db"));
    }

    #[tokio::test]
    async fn build_client_uses_connection_string_database_and_ignores_discrete_fields() {
        // A non-empty connection string takes full precedence: host/db here
        // are deliberately set to values that would produce a different
        // outcome if they were consulted, proving they are ignored.
        let mut config = base_config();
        config.connection_string = Some("mongodb://127.0.0.1:27017/uri_db".to_string());
        config.host = Some("ignored-host".to_string());
        config.db = Some("ignored_db".to_string());
        config.certificate = Some("ignored-cert.pem".to_string());
        config.username = Some("ignored-user".to_string());
        config.password = Some("ignored-pass".to_string());

        let (_client, db_name) = build_client(&config)
            .await
            .expect("connection string with a database parses");
        assert_eq!(db_name, "uri_db");
    }

    #[tokio::test]
    async fn build_client_rejects_connection_string_without_a_database() {
        let mut config = base_config();
        config.connection_string = Some("mongodb://127.0.0.1:27017".to_string());
        let error = build_client(&config)
            .await
            .expect_err("connection string without a database should fail clearly");
        assert!(error.to_string().contains("default database"));
    }
}
