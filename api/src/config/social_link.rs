use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::AppMode;
use super::error::ConfigError;

// ---------------------------------------------------------------------
// Public, validated types
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationConfig {
    pub mode: AppMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminConfig {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UploadsConfig {
    pub max_mb: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentConfig {
    pub max_groups: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemesConfig {
    pub seed_file: PathBuf,
    pub max_preset_per_user: usize,
    pub max_custom_per_user: usize,
}

// ---------------------------------------------------------------------
// Raw (wire-format) structures mirroring social-link.yaml exactly.
// ---------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SocialLinkFile {
    application: ApplicationSectionRaw,
    admin: AdminSectionRaw,
    uploads: UploadsSectionRaw,
    themes: ThemesSectionRaw,
    #[serde(default)]
    content: ContentSectionRaw,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ApplicationSectionRaw {
    mode: AppMode,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdminSectionRaw {
    username: String,
    email: String,
    password: String,
    display_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UploadsSectionRaw {
    max_mb: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemesSectionRaw {
    seed_file: String,
    max_preset_per_user: usize,
    max_custom_per_user: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContentSectionRaw {
    max_groups: usize,
}

impl Default for ContentSectionRaw {
    fn default() -> Self {
        Self { max_groups: 5 }
    }
}

// ---------------------------------------------------------------------
// Validation: raw -> public typed sections
// ---------------------------------------------------------------------

pub(super) struct ValidatedSocialLink {
    pub(super) application: ApplicationConfig,
    pub(super) admin: AdminConfig,
    pub(super) uploads: UploadsConfig,
    pub(super) themes: ThemesConfig,
    pub(super) content: ContentConfig,
}

impl SocialLinkFile {
    pub(super) fn into_validated(self, path: &Path) -> Result<ValidatedSocialLink, ConfigError> {
        if self.admin.username.trim().is_empty() {
            return Err(ConfigError::validation(
                path,
                "admin.username must not be empty",
            ));
        }
        if self.admin.email.trim().is_empty() {
            return Err(ConfigError::validation(
                path,
                "admin.email must not be empty",
            ));
        }
        if self.admin.password.is_empty() {
            return Err(ConfigError::validation(
                path,
                "admin.password must not be empty",
            ));
        }
        if self.admin.display_name.trim().is_empty() {
            return Err(ConfigError::validation(
                path,
                "admin.display_name must not be empty",
            ));
        }
        if self.uploads.max_mb == 0 {
            return Err(ConfigError::validation(
                path,
                "uploads.max_mb must be nonzero",
            ));
        }
        if self.themes.seed_file.trim().is_empty() {
            return Err(ConfigError::validation(
                path,
                "themes.seed_file must not be empty",
            ));
        }
        if self.themes.max_preset_per_user == 0 {
            return Err(ConfigError::validation(
                path,
                "themes.max_preset_per_user must be nonzero",
            ));
        }
        if self.themes.max_custom_per_user == 0 {
            return Err(ConfigError::validation(
                path,
                "themes.max_custom_per_user must be nonzero",
            ));
        }
        if self.content.max_groups == 0 {
            return Err(ConfigError::validation(
                path,
                "content.max_groups must be nonzero",
            ));
        }

        Ok(ValidatedSocialLink {
            application: ApplicationConfig {
                mode: self.application.mode,
            },
            admin: AdminConfig {
                username: self.admin.username,
                email: self.admin.email,
                password: self.admin.password,
                display_name: self.admin.display_name,
            },
            uploads: UploadsConfig {
                max_mb: self.uploads.max_mb,
            },
            themes: ThemesConfig {
                seed_file: PathBuf::from(self.themes.seed_file),
                max_preset_per_user: self.themes.max_preset_per_user,
                max_custom_per_user: self.themes.max_custom_per_user,
            },
            content: ContentConfig {
                max_groups: self.content.max_groups,
            },
        })
    }
}
