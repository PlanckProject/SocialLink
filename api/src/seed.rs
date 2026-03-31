use anyhow::{Context, Result};

use crate::config::Config;
use crate::error::AppError;
use crate::services::AppServices;
use crate::services::people::RegisterPerson;
use crate::util::normalize_theme;

pub async fn run(services: &AppServices, config: &Config) -> Result<()> {
    services.cleanup_legacy_person_fields().await?;
    let theme = load_theme_config(config).await?;

    let existing = match services
        .find_person_by_email(&config.admin.email)
        .await
        .context("find seeded admin by email")?
    {
        Some(person) => Some(person),
        None => services
            .find_person_by_username(&config.admin.username)
            .await
            .context("find seeded admin by username")?,
    };

    let person = match existing {
        Some(person) => person,
        None => {
            let person = services
                .register_person(
                    RegisterPerson {
                        username: config.admin.username.clone(),
                        email: config.admin.email.clone(),
                        password: config.admin.password.clone(),
                        display_name: Some(config.admin.display_name.clone()),
                    },
                    theme.clone(),
                )
                .await
                .context("seed admin person")?;
            tracing::info!(username = %person.username, "seeded admin user");
            person
        }
    };

    services
        .seed_theme_if_absent(person.id, &person.username, theme)
        .await
        .context("seed admin theme")
}

async fn load_theme_config(config: &Config) -> Result<serde_json::Value> {
    let text = tokio::fs::read_to_string(&config.themes.seed_file)
        .await
        .with_context(|| {
            format!(
                "read configured theme seed file {}",
                config.themes.seed_file.display()
            )
        })?;
    let theme = serde_json::from_str(&text).with_context(|| {
        format!(
            "parse configured theme seed file {}",
            config.themes.seed_file.display()
        )
    })?;
    normalize_theme(theme)
        .map_err(AppError::into_inner)
        .context("normalize configured theme seed")
}
