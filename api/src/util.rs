use axum::http::HeaderMap;
use axum::http::header::{REFERER, USER_AGENT};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::domain::EntityId;
use crate::error::{AppError, AppResult};

pub fn parse_entity_id(id: &str) -> AppResult<EntityId> {
    EntityId::parse(id).map_err(|_| AppError::bad_request("invalid id"))
}

pub fn utc_to_iso(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.to_rfc3339()
}

/// Salted, truncated SHA-256 of a visitor IP — never stores the raw address.
pub fn hash_ip(salt: &str, ip: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b"|");
    hasher.update(ip.as_bytes());
    hex::encode(hasher.finalize())[..32].to_string()
}

/// The built-in default theme, embedded at compile time. Used to seed the first
/// theme and as a fallback when no active theme exists.
pub fn default_theme_value() -> serde_json::Value {
    serde_json::from_str(include_str!("../theme.json")).unwrap_or(serde_json::Value::Null)
}

/// Strips `//` line comments, `/* */` block comments and trailing commas from a
/// JSONC document, preserving comment-like sequences inside string literals so
/// the result parses as strict JSON.
fn strip_jsonc(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let n = chars.len();
    let mut out: Vec<char> = Vec::with_capacity(n);
    let mut i = 0;
    let mut in_string = false;
    let mut escaped = false;

    while i < n {
        let c = chars[i];
        if in_string {
            out.push(c);
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < n && chars[i + 1] == '/' {
            i += 2;
            while i < n && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < n && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
            continue;
        }
        if c == '}' || c == ']' {
            let mut k = out.len();
            while k > 0 && out[k - 1].is_whitespace() {
                k -= 1;
            }
            if k > 0 && out[k - 1] == ',' {
                out.remove(k - 1);
            }
            out.push(c);
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out.into_iter().collect()
}

/// Parses a JSONC (JSON-with-comments) string into a JSON value.
pub fn parse_jsonc(input: &str) -> AppResult<Value> {
    let stripped = strip_jsonc(input);
    serde_json::from_str(&stripped)
        .map_err(|e| AppError::bad_request(format!("invalid theme JSON: {e}")))
}

/// Recursively merges `over` into `base`. Objects merge key-by-key; any other
/// value in `over` replaces the corresponding value in `base`.
pub fn deep_merge(base: &mut Value, over: &Value) {
    match (base, over) {
        (Value::Object(base_map), Value::Object(over_map)) => {
            for (key, value) in over_map {
                deep_merge(base_map.entry(key.clone()).or_insert(Value::Null), value);
            }
        }
        (base_slot, other) => {
            *base_slot = other.clone();
        }
    }
}

fn radius_percentage(value: Option<&Value>, fallback: f64, max: f64) -> String {
    let parsed = match value {
        Some(Value::Number(number)) => number.as_f64(),
        Some(Value::String(value)) => {
            let value = value.trim().to_ascii_lowercase();
            let number = value
                .strip_suffix('%')
                .or_else(|| value.strip_suffix("px"))
                .unwrap_or(&value)
                .trim();
            number.parse::<f64>().ok()
        }
        _ => None,
    }
    .filter(|value| value.is_finite())
    .unwrap_or(fallback)
    .clamp(0.0, max);

    format!("{parsed}%")
}

fn migrate_legacy_radius(theme: &mut Value) {
    let legacy = theme
        .get("radius")
        .and_then(Value::as_object)
        .map(|radius| {
            (
                radius.get("sm").cloned(),
                radius.get("md").cloned(),
                radius.get("lg").cloned(),
                radius.get("pill").cloned(),
            )
        });
    let avatar_shape = theme
        .get("layout")
        .and_then(Value::as_object)
        .and_then(|layout| layout.get("avatar_shape"))
        .and_then(Value::as_str)
        .map(str::to_owned);

    if let (Some((sm, md, lg, pill)), Some(radius)) = (
        legacy,
        theme.get_mut("radius").and_then(Value::as_object_mut),
    ) {
        let avatar = match avatar_shape.as_deref() {
            Some("square") => sm.clone(),
            Some("rounded") => lg.clone(),
            _ => pill.clone(),
        };
        for (key, value) in [
            ("link", lg.clone()),
            ("link_icon", md.clone()),
            ("background", lg),
            ("avatar", avatar),
            ("social_icon", pill),
        ] {
            if let (false, Some(value)) = (radius.contains_key(key), value) {
                radius.insert(key.to_string(), value);
            }
        }
        for key in ["sm", "md", "lg", "pill"] {
            radius.remove(key);
        }
    }
    if let Some(radius) = theme.get_mut("radius").and_then(Value::as_object_mut) {
        for key in ["button", "input", "icon"] {
            radius.remove(key);
        }
    }

    if let Some(layout) = theme.get_mut("layout").and_then(Value::as_object_mut) {
        layout.remove("avatar_shape");
        layout.remove("avatar_size");
    }
    if let Some(effects) = theme.get_mut("effects").and_then(Value::as_object_mut) {
        effects.remove("glass");
        effects.remove("animations");
    }
}

fn normalize_radius_percentages(theme: &mut Value) {
    let Some(radius) = theme.get_mut("radius").and_then(Value::as_object_mut) else {
        return;
    };
    for (key, fallback, max) in [
        ("link", 22.0, 50.0),
        ("link_icon", 14.0, 50.0),
        ("background", 20.0, 20.0),
        ("avatar", 50.0, 50.0),
        ("social_icon", 50.0, 50.0),
    ] {
        let normalized = radius_percentage(radius.get(key), fallback, max);
        radius.insert(key.to_string(), Value::String(normalized));
    }
}

/// Normalizes an uploaded/posted theme into a complete, valid theme object:
/// unwraps a `{ "config": {...} }` wrapper, merges it over the built-in default
/// so every field is present, and validates the core sections.
pub fn normalize_theme(raw: Value) -> AppResult<Value> {
    let mut incoming = if raw.get("colors").is_some() {
        raw
    } else if let Some(inner) = raw.get("config").cloned() {
        inner
    } else {
        raw
    };
    if !incoming.is_object() {
        return Err(AppError::bad_request("theme must be a JSON object"));
    }
    migrate_legacy_radius(&mut incoming);

    let mut merged = default_theme_value();
    deep_merge(&mut merged, &incoming);

    for section in [
        "colors",
        "fonts",
        "radius",
        "layout",
        "button",
        "background",
        "effects",
        "branding",
        "features",
    ] {
        let is_object = merged.get(section).map(Value::is_object).unwrap_or(false);
        if !is_object {
            return Err(AppError::bad_request(format!(
                "theme section '{section}' must be an object"
            )));
        }
    }
    normalize_radius_percentages(&mut merged);
    Ok(merged)
}

/// Serializes a theme config as a downloadable JSONC document with an owner
/// comment header. The comments are ignored when the file is re-imported.
pub fn theme_jsonc_bytes(
    name: &str,
    owner: Option<&str>,
    source: &str,
    config: &Value,
) -> AppResult<Vec<u8>> {
    let clean = |s: &str| s.replace(['\n', '\r'], " ");
    let owner = owner.unwrap_or("unknown");
    let mut out = String::new();
    out.push_str("// SocialLink theme export\n");
    out.push_str(&format!("// Name: {}\n", clean(name)));
    out.push_str(&format!("// Owner: @{}\n", clean(owner)));
    out.push_str(&format!("// Source: {}\n", clean(source)));
    out.push_str(&format!(
        "// Exported: {}\n",
        chrono::Utc::now().to_rfc3339()
    ));
    out.push_str("// Comments are ignored when this file is re-imported.\n");
    let json = serde_json::to_string_pretty(config)
        .map_err(|error| AppError::internal(anyhow::Error::new(error)))?;
    out.push_str(&json);
    out.push('\n');
    Ok(out.into_bytes())
}

pub struct ReqMeta {
    pub referrer: Option<String>,
    pub user_agent: Option<String>,
    pub ip: String,
}

/// Extracts privacy-relevant request metadata. Behind the Nuxt proxy the real
/// client IP arrives via `X-Forwarded-For`.
pub fn req_meta(headers: &HeaderMap) -> ReqMeta {
    let header_str = |name: &str| headers.get(name).and_then(|v| v.to_str().ok());

    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| header_str("x-real-ip").map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    ReqMeta {
        referrer: headers
            .get(REFERER)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
        user_agent: headers
            .get(USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
        ip,
    }
}
