use crate::domain::EntityId;

pub fn person_by_id(id: EntityId) -> String {
    format!("person:id:{id}")
}

pub fn person_by_username(username: &str) -> String {
    format!("person:username:{}", username.to_ascii_lowercase())
}

pub fn primary_person() -> String {
    "person:primary".to_string()
}

pub fn groups(user_id: EntityId) -> String {
    format!("groups:user:{user_id}")
}

pub fn active_groups(user_id: EntityId) -> String {
    format!("groups:active:{user_id}")
}

pub fn links(user_id: EntityId) -> String {
    format!("links:user:{user_id}")
}

pub fn public_links(user_id: EntityId) -> String {
    format!("links:public:{user_id}")
}

pub fn link(link_id: EntityId) -> String {
    format!("link:id:{link_id}")
}

pub fn active_theme(user_id: EntityId) -> String {
    format!("theme:active:{user_id}")
}

pub fn theme(theme_id: EntityId) -> String {
    format!("theme:id:{theme_id}")
}

pub fn themes(user_id: EntityId) -> String {
    format!("themes:user:{user_id}")
}

pub fn public_profile(user_id: EntityId) -> String {
    format!("profile:public:{user_id}")
}

pub fn person_invalidation(id: EntityId, usernames: &[&str]) -> Vec<String> {
    let mut keys = vec![person_by_id(id), primary_person(), public_profile(id)];
    keys.extend(
        usernames
            .iter()
            .map(|username| person_by_username(username)),
    );
    keys
}

pub fn group_invalidation(user_id: EntityId) -> Vec<String> {
    vec![
        groups(user_id),
        active_groups(user_id),
        public_profile(user_id),
    ]
}

pub fn link_invalidation(user_id: EntityId, link_id: Option<EntityId>) -> Vec<String> {
    let mut keys = vec![
        links(user_id),
        public_links(user_id),
        public_profile(user_id),
    ];
    if let Some(link_id) = link_id {
        keys.push(link(link_id));
    }
    keys
}

pub fn theme_invalidation(user_id: EntityId, theme_id: Option<EntityId>) -> Vec<String> {
    let mut keys = vec![
        active_theme(user_id),
        themes(user_id),
        public_profile(user_id),
    ];
    if let Some(theme_id) = theme_id {
        keys.push(theme(theme_id));
    }
    keys
}
