use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::auth::password;
use crate::domain::{
    EntityId, Link, LinkGroup, Person, PersonImageSlot, PersonImageUpdate, PersonProfileUpdate,
    Theme, ThemeSource,
};
use crate::error::ErrorKind;
use crate::providers::database::{Database, PersonRepository, ThemeRepository};
use crate::providers::timeseries::TimeSeries;
use crate::util::normalize_theme;

use super::AppServices;
use super::cache_keys;
use super::media::MediaKind;

const MAX_BIO_LEN: usize = 500;

#[derive(Debug, Clone)]
pub struct RegisterPerson {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UsernameAvailability {
    pub username: String,
    pub valid: bool,
    pub available: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicProfileGroup {
    pub group: LinkGroup,
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicProfileBase {
    pub person: Person,
    pub groups: Vec<PublicProfileGroup>,
    pub ungrouped: Vec<Link>,
    pub theme: Value,
}

#[derive(Debug, Clone)]
pub struct PublicProfile {
    pub base: PublicProfileBase,
    pub views: Option<i64>,
    pub click_totals: Option<HashMap<EntityId, i64>>,
}

impl AppServices {
    pub async fn register_person(
        &self,
        input: RegisterPerson,
        initial_theme: Value,
    ) -> Result<Person> {
        let username = normalize_username(&input.username)?;
        let email = normalize_email(&input.email)?;
        password::validate_password(&input.password)?;

        if self
            .database
            .people()
            .find_username_or_email_conflict(&username, &email, None)
            .await
            .context("check registration uniqueness")?
            .is_some()
        {
            return Err(typed_error(
                ErrorKind::Conflict("username or email already taken".into()),
                "register person",
            ));
        }

        let display_name = input
            .display_name
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| username.clone());
        let password_hash =
            password::hash_password(&input.password).context("hash registration password")?;
        let person = Person::new(username, email, password_hash, display_name);

        self.database
            .people()
            .insert(&person)
            .await
            .context("insert registered person")?;
        self.invalidate_person(&person, &person).await;

        self.seed_theme_if_absent(person.id, &person.username, initial_theme)
            .await
            .context("seed registered person's theme")?;

        Ok(person)
    }

    pub async fn authenticate_person(
        &self,
        username: &str,
        password_value: &str,
    ) -> Result<Person> {
        let username = match normalize_username(username) {
            Ok(username) => username,
            Err(_) => {
                return Err(typed_error(ErrorKind::Unauthorized, "authenticate person"));
            }
        };
        let person = self
            .find_person_by_username(&username)
            .await
            .context("lookup login person")?
            .ok_or_else(|| typed_error(ErrorKind::Unauthorized, "authenticate person"))?;

        if !password::verify_password(&person.password_hash, password_value) {
            return Err(typed_error(ErrorKind::Unauthorized, "authenticate person"));
        }
        Ok(person)
    }

    pub async fn find_person_by_id(&self, id: EntityId) -> Result<Option<Person>> {
        let key = cache_keys::person_by_id(id);
        if let Some(person) = self.cache.get(&key).await {
            return Ok(Some(person));
        }

        let person = self
            .database
            .people()
            .find_by_id(id)
            .await
            .context("find person by id")?;
        if let Some(person) = &person {
            self.cache_person(person).await;
        }
        Ok(person)
    }

    pub async fn person_by_id(&self, id: EntityId) -> Result<Person> {
        self.find_person_by_id(id).await?.ok_or_else(|| {
            typed_error(
                ErrorKind::NotFound("profile not found".into()),
                "load person",
            )
        })
    }

    pub async fn find_person_by_username(&self, username: &str) -> Result<Option<Person>> {
        let username = username.trim().to_ascii_lowercase();
        let key = cache_keys::person_by_username(&username);
        if let Some(person) = self.cache.get(&key).await {
            return Ok(Some(person));
        }

        let person = self
            .database
            .people()
            .find_by_username(&username)
            .await
            .context("find person by username")?;
        if let Some(person) = &person {
            self.cache_person(person).await;
        }
        Ok(person)
    }

    pub async fn person_by_username(&self, username: &str) -> Result<Person> {
        self.find_person_by_username(username)
            .await?
            .ok_or_else(|| {
                typed_error(
                    ErrorKind::NotFound("profile not found".into()),
                    "load person",
                )
            })
    }

    pub async fn find_person_by_email(&self, email: &str) -> Result<Option<Person>> {
        self.database
            .people()
            .find_by_email(&email.trim().to_ascii_lowercase())
            .await
            .context("find person by email")
    }

    pub async fn primary_person(&self, configured_username: &str) -> Result<Person> {
        let key = cache_keys::primary_person();
        if let Some(person) = self.cache.get(&key).await {
            return Ok(person);
        }

        let person = match self
            .database
            .people()
            .find_by_username(&configured_username.trim().to_ascii_lowercase())
            .await
            .context("find configured primary person")?
        {
            Some(person) => Some(person),
            None => self
                .database
                .people()
                .find_first_created()
                .await
                .context("find first-created primary person")?,
        }
        .ok_or_else(|| {
            typed_error(
                ErrorKind::NotFound("no profile configured".into()),
                "load primary person",
            )
        })?;

        self.cache.set(&key, &person).await;
        self.cache_person(&person).await;
        Ok(person)
    }

    pub async fn username_availability(&self, raw: &str) -> Result<UsernameAvailability> {
        let username = match normalize_username(raw) {
            Ok(username) => username,
            Err(error) => {
                let reason = error
                    .chain()
                    .find_map(|cause| cause.downcast_ref::<ErrorKind>())
                    .map(ToString::to_string)
                    .unwrap_or_else(|| "invalid username".to_string());
                return Ok(UsernameAvailability {
                    username: raw.trim().to_ascii_lowercase(),
                    valid: false,
                    available: false,
                    reason: Some(reason),
                });
            }
        };

        let available = self
            .find_person_by_username(&username)
            .await
            .context("check username availability")?
            .is_none();
        Ok(UsernameAvailability {
            username,
            valid: true,
            available,
            reason: (!available).then(|| "username already taken".to_string()),
        })
    }

    pub async fn update_person_profile(
        &self,
        id: EntityId,
        mut update: PersonProfileUpdate,
    ) -> Result<Person> {
        let old = self
            .person_by_id(id)
            .await
            .context("load profile to update")?;

        if let Some(raw_username) = update.username.take() {
            let username = normalize_username(&raw_username)?;
            if username != old.username
                && self
                    .database
                    .people()
                    .find_username_or_email_conflict(&username, &old.email, Some(id))
                    .await
                    .context("check profile username uniqueness")?
                    .is_some()
            {
                return Err(typed_error(
                    ErrorKind::Conflict("username already taken".into()),
                    "update person profile",
                ));
            }
            update.username = Some(username);
        }

        if update
            .bio
            .as_ref()
            .is_some_and(|bio| bio.chars().count() > MAX_BIO_LEN)
        {
            return Err(typed_error(
                ErrorKind::BadRequest(format!("bio must be {MAX_BIO_LEN} characters or fewer")),
                "update person profile",
            ));
        }

        let updated = self
            .database
            .people()
            .update_profile(id, &update)
            .await
            .context("update person profile")?
            .ok_or_else(|| {
                typed_error(
                    ErrorKind::NotFound("profile not found".into()),
                    "update person profile",
                )
            })?;

        if let Err(error) = self
            .database
            .themes()
            .update_owner(id, &updated.username)
            .await
            .context("reconcile theme owner after profile update")
        {
            tracing::warn!(
                user_id = %id,
                error = %format_args!("{error:#}"),
                "profile updated but denormalized theme owner reconciliation failed"
            );
        }
        self.invalidate_person(&old, &updated).await;

        Ok(updated)
    }

    pub async fn change_person_password(
        &self,
        id: EntityId,
        current_password: &str,
        new_password: &str,
    ) -> Result<()> {
        let person = self
            .person_by_id(id)
            .await
            .context("load person for password change")?;
        if !password::verify_password(&person.password_hash, current_password) {
            return Err(typed_error(
                ErrorKind::BadRequest("current password is incorrect".into()),
                "change password",
            ));
        }
        password::validate_password(new_password)?;
        if password::verify_password(&person.password_hash, new_password) {
            return Err(typed_error(
                ErrorKind::BadRequest(
                    "new password must be different from the current password".into(),
                ),
                "change password",
            ));
        }

        let password_hash =
            password::hash_password(new_password).context("hash replacement password")?;
        self.database
            .people()
            .update_password(id, &password_hash)
            .await
            .context("persist replacement password")?;
        self.invalidate_person(&person, &person).await;
        Ok(())
    }

    pub async fn store_person_image(
        &self,
        id: EntityId,
        slot: PersonImageSlot,
        bytes: Bytes,
        content_type: Option<&str>,
        file_name: Option<&str>,
    ) -> Result<Person> {
        let old_person = self
            .person_by_id(id)
            .await
            .context("load person for image upload")?;
        let old_path = image_path(&old_person, slot).map(str::to_string);
        let path = self
            .store_media(bytes, content_type, file_name, MediaKind::Image)
            .await?;

        let update = PersonImageUpdate::set(slot, path.clone());
        if let Err(error) = self.database.people().update_image(id, &update).await {
            self.delete_media_best_effort(&path).await;
            return Err(error).context("update person image");
        }

        self.invalidate_person(&old_person, &old_person).await;
        if let Some(old_path) = old_path.filter(|old_path| old_path != &path) {
            self.delete_media_best_effort(&old_path).await;
        }
        self.person_by_id(id)
            .await
            .context("reload person after image upload")
    }

    pub async fn delete_person_image(&self, id: EntityId, slot: PersonImageSlot) -> Result<Person> {
        let old_person = self
            .person_by_id(id)
            .await
            .context("load person for image deletion")?;
        let Some(old_path) = image_path(&old_person, slot).map(str::to_string) else {
            return Ok(old_person);
        };

        self.database
            .people()
            .update_image(id, &PersonImageUpdate::clear(slot))
            .await
            .context("clear person image")?;
        self.invalidate_person(&old_person, &old_person).await;
        self.delete_media_best_effort(&old_path).await;
        self.person_by_id(id)
            .await
            .context("reload person after image deletion")
    }

    pub async fn store_media(
        &self,
        bytes: Bytes,
        content_type: Option<&str>,
        file_name: Option<&str>,
        kind: MediaKind,
    ) -> Result<String> {
        self.media
            .store(bytes, content_type, file_name, kind)
            .await
            .map_err(|error| {
                let message = error.to_string();
                if message.contains("uploaded file exceeds")
                    || message.contains("unsupported uploaded file type")
                {
                    typed_error(
                        ErrorKind::BadRequest(if message.contains("exceeds") {
                            "file exceeds size limit".into()
                        } else {
                            "unsupported file type".into()
                        }),
                        "store uploaded media",
                    )
                } else {
                    error.context("store uploaded media")
                }
            })
    }

    pub async fn public_profile(&self, person: &Person) -> Result<PublicProfile> {
        let key = cache_keys::public_profile(person.id);
        let mut base = if let Some(base) = self.cache.get::<PublicProfileBase>(&key).await {
            base
        } else {
            let base = self
                .load_public_profile_base(person)
                .await
                .context("assemble public profile base")?;
            self.cache.set(&key, &base).await;
            base
        };

        let now = chrono::Utc::now();
        for group in &mut base.groups {
            group
                .links
                .retain(|link| link.is_active && link.expires_at.is_none_or(|expiry| expiry > now));
        }
        base.ungrouped
            .retain(|link| link.is_active && link.expires_at.is_none_or(|expiry| expiry > now));

        let features = base.theme.get("features");
        let show_view_count = features
            .and_then(|features| features.get("show_view_count"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let show_click_count = features
            .and_then(|features| features.get("show_click_count"))
            .and_then(Value::as_bool)
            .unwrap_or(false);

        let views = if show_view_count {
            Some(
                self.timeseries
                    .total_views(person.id)
                    .await
                    .context("query public profile view total")?,
            )
        } else {
            None
        };

        let click_totals = if show_click_count {
            let link_ids = base
                .groups
                .iter()
                .flat_map(|group| group.links.iter())
                .chain(base.ungrouped.iter())
                .map(|link| link.id)
                .collect::<Vec<_>>();
            Some(
                self.timeseries
                    .click_totals(person.id, &link_ids)
                    .await
                    .context("query public profile click totals")?,
            )
        } else {
            None
        };

        Ok(PublicProfile {
            base,
            views,
            click_totals,
        })
    }

    pub async fn seed_theme_if_absent(
        &self,
        user_id: EntityId,
        owner: &str,
        config: Value,
    ) -> Result<()> {
        if self
            .database
            .themes()
            .count_all(user_id)
            .await
            .context("count person's themes")?
            > 0
        {
            return Ok(());
        }

        let config = normalize_theme(config)
            .map_err(crate::error::AppError::into_inner)
            .context("normalize initial theme")?;
        let name = config
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("Default")
            .to_string();
        let theme = Theme::new(
            user_id,
            Some(owner.to_string()),
            name,
            config,
            ThemeSource::Preset,
            true,
        );
        self.database
            .themes()
            .create(&theme)
            .await
            .context("seed person's initial theme")?;
        self.cache
            .invalidate(&cache_keys::theme_invalidation(user_id, Some(theme.id)))
            .await;
        Ok(())
    }

    pub async fn cleanup_legacy_person_fields(&self) -> Result<()> {
        self.database
            .people()
            .cleanup_legacy_fields()
            .await
            .context("cleanup legacy person fields")
    }

    async fn load_public_profile_base(&self, person: &Person) -> Result<PublicProfileBase> {
        let (groups, links, theme) = tokio::try_join!(
            self.list_active_groups(person.id),
            self.list_public_links(person.id),
            self.active_theme_config(person.id),
        )?;

        let (mut grouped, ungrouped) = partition_public_links(&groups, links);

        let groups = groups
            .into_iter()
            .map(|group| PublicProfileGroup {
                links: grouped.remove(&group.id).unwrap_or_default(),
                group,
            })
            .collect();

        Ok(PublicProfileBase {
            person: person.clone(),
            groups,
            ungrouped,
            theme,
        })
    }

    async fn cache_person(&self, person: &Person) {
        self.cache
            .set(&cache_keys::person_by_id(person.id), person)
            .await;
        self.cache
            .set(&cache_keys::person_by_username(&person.username), person)
            .await;
    }

    async fn invalidate_person(&self, old: &Person, new: &Person) {
        self.cache
            .invalidate(&cache_keys::person_invalidation(
                old.id,
                &[old.username.as_str(), new.username.as_str()],
            ))
            .await;
    }

    async fn delete_media_best_effort(&self, path: &str) {
        if let Err(error) = self.media.delete(path).await {
            tracing::warn!(
                error = %format_args!("{error:#}"),
                "failed to delete replaced media"
            );
        }
    }
}

fn partition_public_links(
    groups: &[LinkGroup],
    links: Vec<Link>,
) -> (HashMap<EntityId, Vec<Link>>, Vec<Link>) {
    let active_group_ids = groups.iter().map(|group| group.id).collect::<HashSet<_>>();
    let mut grouped = HashMap::<EntityId, Vec<Link>>::new();
    let mut ungrouped = Vec::new();
    for link in links {
        match link.group_id {
            Some(group_id) if active_group_ids.contains(&group_id) => {
                grouped.entry(group_id).or_default().push(link);
            }
            Some(_) => {}
            None => ungrouped.push(link),
        }
    }
    (grouped, ungrouped)
}

fn normalize_username(raw: &str) -> Result<String> {
    let username = raw.trim().to_ascii_lowercase();
    if !(2..=30).contains(&username.len()) {
        return Err(typed_error(
            ErrorKind::BadRequest("username must be between 2 and 30 characters".into()),
            "validate username",
        ));
    }
    let starts_ok = username
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_alphanumeric());
    let chars_ok = username
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '.'));
    if !starts_ok || !chars_ok {
        return Err(typed_error(
            ErrorKind::BadRequest(
                "username may contain only lowercase letters, digits, '.' or '_', and must start with a letter or digit".into(),
            ),
            "validate username",
        ));
    }
    Ok(username)
}

fn normalize_email(raw: &str) -> Result<String> {
    let email = raw.trim().to_ascii_lowercase();
    let mut parts = email.split('@');
    let local = parts.next().unwrap_or_default();
    let domain = parts.next().unwrap_or_default();
    let valid = !local.is_empty()
        && !domain.is_empty()
        && parts.next().is_none()
        && !email.chars().any(char::is_whitespace);
    if !valid {
        return Err(typed_error(
            ErrorKind::BadRequest("invalid email address".into()),
            "validate email",
        ));
    }
    Ok(email)
}

fn image_path(person: &Person, slot: PersonImageSlot) -> Option<&str> {
    match slot {
        PersonImageSlot::Avatar => person.avatar_path.as_deref(),
        PersonImageSlot::Cover => person.cover_path.as_deref(),
    }
}

fn typed_error(kind: ErrorKind, context: &'static str) -> anyhow::Error {
    anyhow::Error::new(kind).context(context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_from_inactive_groups_are_not_exposed_as_ungrouped() {
        let owner = EntityId::new();
        let active_group = LinkGroup::new(owner, "Active", 0);
        let inactive_group_id = EntityId::new();
        let grouped_link = Link::new(
            owner,
            Some(active_group.id),
            "Grouped",
            "https://example.com/grouped",
            0,
        );
        let hidden_link = Link::new(
            owner,
            Some(inactive_group_id),
            "Hidden",
            "https://example.com/hidden",
            0,
        );
        let ungrouped_link =
            Link::new(owner, None, "Ungrouped", "https://example.com/ungrouped", 0);

        let (grouped, ungrouped) = partition_public_links(
            std::slice::from_ref(&active_group),
            vec![grouped_link.clone(), hidden_link, ungrouped_link.clone()],
        );

        assert_eq!(grouped.get(&active_group.id), Some(&vec![grouped_link]));
        assert_eq!(ungrouped, vec![ungrouped_link]);
    }
}
