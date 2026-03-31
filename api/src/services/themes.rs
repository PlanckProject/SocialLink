use anyhow::{Context, Result};
use serde_json::Value;

use crate::domain::{EntityId, Theme, ThemeFilter, ThemeSource, ThemeUpdate};
use crate::error::{AppError, ErrorKind};
use crate::providers::database::{Database, ThemeRepository};
use crate::util::{default_theme_value, normalize_theme, parse_jsonc, theme_jsonc_bytes};

use crate::services::{AppServices, cache_keys};

pub struct ThemeDownload {
    pub bytes: Vec<u8>,
    pub filename: String,
}

impl AppServices {
    pub async fn list_themes(
        &self,
        user_id: EntityId,
        favorite: Option<bool>,
        source: Option<&str>,
    ) -> Result<Vec<Theme>> {
        let key = cache_keys::themes(user_id);
        let themes = if let Some(themes) = self.cache.get::<Vec<Theme>>(&key).await {
            themes
        } else {
            let themes = self
                .database
                .themes()
                .list(&ThemeFilter::for_user(user_id))
                .await
                .context("list theme library")?;
            self.cache.set(&key, &themes).await;
            themes
        };
        for theme in &themes {
            self.cache.set(&cache_keys::theme(theme.id), theme).await;
        }

        Ok(themes
            .into_iter()
            .filter(|theme| favorite.is_none_or(|favorite| theme.is_favorite == favorite))
            .filter(|theme| source.is_none_or(|source| theme_source_name(theme.source) == source))
            .collect())
    }

    pub async fn get_theme(&self, id: EntityId, user_id: EntityId) -> Result<Theme> {
        let mut theme = self.cached_theme(id, user_id).await?;
        theme.config = normalize_config(theme.config)?;
        Ok(theme)
    }

    async fn cached_theme(&self, id: EntityId, user_id: EntityId) -> Result<Theme> {
        let key = cache_keys::theme(id);
        if let Some(theme) = self.cache.get::<Theme>(&key).await {
            return if theme.user_id == user_id {
                Ok(theme)
            } else {
                Err(theme_not_found())
            };
        }

        let theme = self
            .database
            .themes()
            .by_id(id, user_id)
            .await
            .context("get theme")?
            .ok_or_else(theme_not_found)?;
        self.cache.set(&key, &theme).await;
        Ok(theme)
    }

    pub async fn active_theme(&self, user_id: EntityId) -> Result<Option<Theme>> {
        let key = cache_keys::active_theme(user_id);
        if let Some(theme) = self.cache.get::<Option<Theme>>(&key).await {
            if let Some(theme) = &theme {
                self.cache.set(&cache_keys::theme(theme.id), theme).await;
            }
            return Ok(theme);
        }

        let theme = self
            .database
            .themes()
            .active(user_id)
            .await
            .context("get active theme")?;
        self.cache.set(&key, &theme).await;
        if let Some(theme) = &theme {
            self.cache.set(&cache_keys::theme(theme.id), theme).await;
        }
        Ok(theme)
    }

    pub async fn active_theme_config(&self, user_id: EntityId) -> Result<Value> {
        match self.active_theme(user_id).await? {
            Some(theme) => normalize_config(theme.config),
            None => Ok(default_theme_value()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_theme(
        &self,
        user_id: EntityId,
        owner: &str,
        name: Option<String>,
        config: Value,
        description: Option<String>,
        tags: Option<Vec<String>>,
        activate: bool,
        max_custom: usize,
    ) -> Result<Theme> {
        self.ensure_custom_capacity(user_id, max_custom).await?;
        let config = normalize_config(config)?;
        let name = theme_name(name, &config, "Untitled");
        let mut theme = Theme::new(
            user_id,
            Some(owner.to_string()),
            name,
            config,
            ThemeSource::Custom,
            false,
        );
        theme.description = description;
        theme.tags = tags.unwrap_or_default();

        self.database
            .themes()
            .create(&theme)
            .await
            .context("create theme")?;
        self.invalidate_theme_cache(user_id, Some(theme.id)).await;
        let theme = if activate {
            self.activate_owned(theme.id, user_id, "activate created theme")
                .await?
        } else {
            theme
        };
        Ok(theme)
    }

    pub async fn update_theme(
        &self,
        id: EntityId,
        user_id: EntityId,
        mut update: ThemeUpdate,
    ) -> Result<Theme> {
        if update
            .name
            .as_ref()
            .is_some_and(|name| name.trim().is_empty())
        {
            update.name = None;
        }
        if let Some(config) = update.config.take() {
            update.config = Some(normalize_config(config)?);
        }

        let theme = self
            .database
            .themes()
            .update(id, user_id, &update)
            .await
            .context("update theme")?
            .ok_or_else(theme_not_found)?;
        self.invalidate_theme_cache(user_id, Some(id)).await;
        Ok(theme)
    }

    pub async fn activate_theme(&self, id: EntityId, user_id: EntityId) -> Result<Value> {
        let theme = self.cached_theme(id, user_id).await?;
        let config = normalize_config(theme.config.clone())?;
        if config != theme.config {
            self.database
                .themes()
                .update(
                    id,
                    user_id,
                    &ThemeUpdate {
                        config: Some(config.clone()),
                        ..ThemeUpdate::default()
                    },
                )
                .await
                .context("normalize theme before activation")?
                .ok_or_else(theme_not_found)?;
            self.invalidate_theme_cache(user_id, Some(id)).await;
        }

        self.activate_owned(id, user_id, "activate theme").await?;
        Ok(config)
    }

    pub async fn delete_theme(&self, id: EntityId, user_id: EntityId) -> Result<()> {
        if !self
            .database
            .themes()
            .delete(id, user_id)
            .await
            .context("delete theme")?
        {
            return Err(theme_not_found());
        }
        self.invalidate_theme_cache(user_id, Some(id)).await;
        Ok(())
    }

    pub async fn apply_preset(
        &self,
        user_id: EntityId,
        owner: &str,
        name: Option<String>,
        config: Value,
        max_presets: usize,
    ) -> Result<Value> {
        let config = normalize_config(config)?;
        let name = theme_name(name, &config, "Preset");
        let existing = self
            .database
            .themes()
            .by_preset_name(user_id, &name)
            .await
            .context("find preset slot")?;

        let id = if let Some(existing) = existing {
            let updated = self
                .database
                .themes()
                .update(
                    existing.id,
                    user_id,
                    &ThemeUpdate {
                        name: Some(name),
                        config: Some(config.clone()),
                        ..ThemeUpdate::default()
                    },
                )
                .await
                .context("update preset slot")?
                .ok_or_else(theme_not_found)?;
            self.invalidate_theme_cache(user_id, Some(updated.id)).await;
            updated.id
        } else {
            let count = self
                .database
                .themes()
                .count_presets(user_id)
                .await
                .context("count preset themes")?;
            if count as usize >= max_presets {
                return Err(ErrorKind::Conflict(format!(
                    "preset limit reached: at most {max_presets} preset themes are allowed"
                ))
                .into());
            }
            let theme = Theme::new(
                user_id,
                Some(owner.to_string()),
                name,
                config.clone(),
                ThemeSource::Preset,
                false,
            );
            self.database
                .themes()
                .create(&theme)
                .await
                .context("create preset theme")?;
            self.invalidate_theme_cache(user_id, Some(theme.id)).await;
            theme.id
        };

        self.activate_owned(id, user_id, "activate preset").await?;
        Ok(config)
    }

    pub async fn update_active_theme(
        &self,
        user_id: EntityId,
        owner: &str,
        config: Value,
        max_custom: usize,
    ) -> Result<Value> {
        let config = normalize_config(config)?;
        let name = theme_name(None, &config, "Custom");

        if let Some(active) = self.active_theme(user_id).await? {
            self.database
                .themes()
                .update(
                    active.id,
                    user_id,
                    &ThemeUpdate {
                        name: Some(name),
                        config: Some(config.clone()),
                        ..ThemeUpdate::default()
                    },
                )
                .await
                .context("update active theme")?
                .ok_or_else(theme_not_found)?;
            self.invalidate_theme_cache(user_id, Some(active.id)).await;
        } else {
            self.ensure_custom_capacity(user_id, max_custom).await?;
            let theme = Theme::new(
                user_id,
                Some(owner.to_string()),
                name,
                config.clone(),
                ThemeSource::Custom,
                false,
            );
            self.database
                .themes()
                .create(&theme)
                .await
                .context("create active theme")?;
            self.invalidate_theme_cache(user_id, Some(theme.id)).await;
            self.activate_owned(theme.id, user_id, "activate new theme")
                .await?;
        }

        Ok(config)
    }

    pub async fn import_theme(
        &self,
        user_id: EntityId,
        owner: &str,
        bytes: &[u8],
        activate: bool,
        max_custom: usize,
    ) -> Result<Theme> {
        self.ensure_custom_capacity(user_id, max_custom).await?;
        let text = std::str::from_utf8(bytes)
            .map_err(|_| ErrorKind::BadRequest("file must be UTF-8 text".into()))?;
        let config = parse_jsonc(text)
            .map_err(AppError::into_inner)
            .and_then(normalize_config)?;
        let name = theme_name(None, &config, "Imported");
        let theme = Theme::new(
            user_id,
            Some(owner.to_string()),
            name,
            config,
            ThemeSource::Imported,
            false,
        );

        self.database
            .themes()
            .create(&theme)
            .await
            .context("import theme")?;
        self.invalidate_theme_cache(user_id, Some(theme.id)).await;
        let theme = if activate {
            self.activate_owned(theme.id, user_id, "activate imported theme")
                .await?
        } else {
            theme
        };
        Ok(theme)
    }

    pub async fn download_active_theme(
        &self,
        user_id: EntityId,
        owner: &str,
    ) -> Result<ThemeDownload> {
        let active = self.active_theme(user_id).await?;
        let (name, source, export_owner, config, id) = match active {
            Some(theme) => {
                let config = normalize_config(theme.config)?;
                (
                    theme.name,
                    theme.source,
                    theme.owner.unwrap_or_else(|| owner.to_string()),
                    config,
                    Some(theme.id),
                )
            }
            None => (
                "theme".to_string(),
                ThemeSource::Custom,
                owner.to_string(),
                default_theme_value(),
                None,
            ),
        };

        if let Some(id) = id {
            self.increment_theme_downloads(id, user_id).await;
        }
        build_download(&name, &export_owner, source, &config)
    }

    pub async fn download_theme(
        &self,
        id: EntityId,
        user_id: EntityId,
        owner: &str,
    ) -> Result<ThemeDownload> {
        let theme = self.get_theme(id, user_id).await?;
        let config = normalize_config(theme.config)?;
        let export_owner = theme.owner.unwrap_or_else(|| owner.to_string());
        self.increment_theme_downloads(id, user_id).await;
        build_download(&theme.name, &export_owner, theme.source, &config)
    }

    pub async fn toggle_theme_favorite(&self, id: EntityId, user_id: EntityId) -> Result<Theme> {
        let theme = self.cached_theme(id, user_id).await?;
        let theme = self
            .database
            .themes()
            .update(
                id,
                user_id,
                &ThemeUpdate {
                    is_favorite: Some(!theme.is_favorite),
                    ..ThemeUpdate::default()
                },
            )
            .await
            .context("toggle theme favorite")?
            .ok_or_else(theme_not_found)?;
        self.invalidate_theme_cache(user_id, Some(id)).await;
        Ok(theme)
    }

    pub async fn increment_theme_downloads(&self, id: EntityId, user_id: EntityId) {
        match self
            .database
            .themes()
            .increment_download_count(id, user_id)
            .await
        {
            Ok(()) => self.invalidate_theme_cache(user_id, Some(id)).await,
            Err(error) => tracing::warn!(
                error = %format_args!("{error:#}"),
                theme_id = %id,
                "failed to increment theme download count"
            ),
        }
    }

    async fn ensure_custom_capacity(&self, user_id: EntityId, max_custom: usize) -> Result<()> {
        let count = self
            .database
            .themes()
            .count_saveable(user_id)
            .await
            .context("count custom themes")?;
        if count as usize >= max_custom {
            return Err(ErrorKind::Conflict(format!(
                "theme limit reached: at most {max_custom} custom or imported themes are allowed"
            ))
            .into());
        }
        Ok(())
    }

    async fn activate_owned(
        &self,
        id: EntityId,
        user_id: EntityId,
        context: &'static str,
    ) -> Result<Theme> {
        let theme = self
            .database
            .themes()
            .activate(id, user_id)
            .await
            .context(context)?
            .ok_or_else(theme_not_found)?;
        self.invalidate_theme_cache(user_id, Some(id)).await;
        Ok(theme)
    }

    async fn invalidate_theme_cache(&self, user_id: EntityId, theme_id: Option<EntityId>) {
        self.cache
            .invalidate(&cache_keys::theme_invalidation(user_id, theme_id))
            .await;
    }
}

fn normalize_config(config: Value) -> Result<Value> {
    normalize_theme(config).map_err(AppError::into_inner)
}

fn theme_name(name: Option<String>, config: &Value, fallback: &str) -> String {
    name.filter(|name| !name.trim().is_empty())
        .or_else(|| {
            config
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| fallback.to_string())
}

fn theme_source_name(source: ThemeSource) -> &'static str {
    match source {
        ThemeSource::Custom => "custom",
        ThemeSource::Imported => "imported",
        ThemeSource::Preset => "preset",
        ThemeSource::Marketplace => "marketplace",
    }
}

fn build_download(
    name: &str,
    owner: &str,
    source: ThemeSource,
    config: &Value,
) -> Result<ThemeDownload> {
    let bytes = theme_jsonc_bytes(name, Some(owner), theme_source_name(source), config)
        .map_err(AppError::into_inner)?;
    let slug: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect();
    let filename = if slug.trim_matches('-').is_empty() {
        "theme.jsonc".to_string()
    } else {
        format!("{slug}.jsonc")
    };
    Ok(ThemeDownload { bytes, filename })
}

fn theme_not_found() -> anyhow::Error {
    ErrorKind::NotFound("theme not found".into()).into()
}
