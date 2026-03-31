use std::path::Path;

use serde_yaml::Value;

use super::error::ConfigError;

/// Replaces `${VAR_NAME}` and `${VAR_NAME:-default}` placeholders inside
/// parsed YAML string values. Interpolating after parsing prevents secret
/// characters such as `:`, `#`, quotes, or newlines from changing the YAML
/// document structure. `${VAR_NAME:-default}` substitutes `default` when
/// `VAR_NAME` is unset or empty; a bare `${VAR_NAME}` has no fallback and a
/// missing variable is a hard, clearly reported error.
pub(super) fn interpolate(value: &mut Value, path: &Path) -> Result<(), ConfigError> {
    match value {
        Value::String(value) => *value = interpolate_string(value, path)?,
        Value::Sequence(values) => {
            for value in values {
                interpolate(value, path)?;
            }
        }
        Value::Mapping(mapping) => {
            for value in mapping.values_mut() {
                interpolate(value, path)?;
            }
        }
        Value::Tagged(tagged) => interpolate(&mut tagged.value, path)?,
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
    Ok(())
}

fn interpolate_string(input: &str, path: &Path) -> Result<String, ConfigError> {
    let mut result = String::with_capacity(input.len());
    let mut rest = input;

    loop {
        match rest.find("${") {
            None => {
                result.push_str(rest);
                break;
            }
            Some(start) => {
                result.push_str(&rest[..start]);
                let after = &rest[start + 2..];
                let end = after.find('}').ok_or_else(|| ConfigError::Interpolation {
                    path: path.to_path_buf(),
                    reason: "unterminated ${...} placeholder".to_string(),
                })?;

                let placeholder = &after[..end];
                // Support a shell-style default: `${VAR:-default}`. The default
                // is used when VAR is unset or empty; a bare `${VAR}` stays
                // required.
                let (var_name, default) = match placeholder.split_once(":-") {
                    Some((name, default)) => (name, Some(default)),
                    None => (placeholder, None),
                };

                let is_valid_name = !var_name.is_empty()
                    && var_name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_');
                if !is_valid_name {
                    return Err(ConfigError::Interpolation {
                        path: path.to_path_buf(),
                        reason: format!("invalid environment variable name `{var_name}`"),
                    });
                }

                let value = match std::env::var(var_name) {
                    Ok(value) if !value.is_empty() => value,
                    _ => match default {
                        Some(default) => default.to_string(),
                        // No default: preserve strict semantics. A set-but-empty
                        // variable interpolates to "" (caught later by field
                        // validation); only a truly unset variable is a hard,
                        // clearly reported error here.
                        None => std::env::var(var_name)
                            .map_err(|_| ConfigError::MissingEnvVar(var_name.to_string()))?,
                    },
                };
                result.push_str(&value);
                rest = &after[end + 1..];
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p() -> PathBuf {
        PathBuf::from("test.yaml")
    }

    #[test]
    fn replaces_single_placeholder() {
        unsafe {
            std::env::set_var("SOCIAL_LINK_CFG_TEST_VAR_A", "hello");
        }
        let mut value: Value =
            serde_yaml::from_str("value: ${SOCIAL_LINK_CFG_TEST_VAR_A}").unwrap();
        interpolate(&mut value, &p()).unwrap();
        assert_eq!(value["value"].as_str(), Some("hello"));
        unsafe {
            std::env::remove_var("SOCIAL_LINK_CFG_TEST_VAR_A");
        }
    }

    #[test]
    fn replaces_multiple_placeholders() {
        unsafe {
            std::env::set_var("SOCIAL_LINK_CFG_TEST_VAR_B1", "foo");
            std::env::set_var("SOCIAL_LINK_CFG_TEST_VAR_B2", "bar");
        }
        let mut value: Value = serde_yaml::from_str(
            "value: \"${SOCIAL_LINK_CFG_TEST_VAR_B1}, ${SOCIAL_LINK_CFG_TEST_VAR_B2}\"",
        )
        .unwrap();
        interpolate(&mut value, &p()).unwrap();
        assert_eq!(value["value"].as_str(), Some("foo, bar"));
        unsafe {
            std::env::remove_var("SOCIAL_LINK_CFG_TEST_VAR_B1");
            std::env::remove_var("SOCIAL_LINK_CFG_TEST_VAR_B2");
        }
    }

    #[test]
    fn fails_on_missing_var() {
        unsafe {
            std::env::remove_var("SOCIAL_LINK_CFG_TEST_VAR_MISSING");
        }
        let mut value: Value =
            serde_yaml::from_str("value: ${SOCIAL_LINK_CFG_TEST_VAR_MISSING}").unwrap();
        let err = interpolate(&mut value, &p()).unwrap_err();
        assert!(
            matches!(err, ConfigError::MissingEnvVar(name) if name == "SOCIAL_LINK_CFG_TEST_VAR_MISSING")
        );
    }

    #[test]
    fn fails_on_unterminated_placeholder() {
        let mut value: Value = serde_yaml::from_str("value: ${OOPS").unwrap();
        let err = interpolate(&mut value, &p()).unwrap_err();
        assert!(matches!(err, ConfigError::Interpolation { .. }));
    }

    #[test]
    fn fails_on_empty_placeholder_name() {
        let mut value: Value = serde_yaml::from_str("value: ${}").unwrap();
        let err = interpolate(&mut value, &p()).unwrap_err();
        assert!(matches!(err, ConfigError::Interpolation { .. }));
    }

    #[test]
    fn leaves_plain_text_untouched() {
        let mut value = Value::String("no vars here".to_string());
        interpolate(&mut value, &p()).unwrap();
        assert_eq!(value.as_str(), Some("no vars here"));
    }

    #[test]
    fn preserves_yaml_syntax_inside_environment_values() {
        let secret = "a: b # still secret\nwith \"quotes\"";
        unsafe {
            std::env::set_var("SOCIAL_LINK_CFG_TEST_COMPLEX_SECRET", secret);
        }
        let mut value: Value =
            serde_yaml::from_str("value: ${SOCIAL_LINK_CFG_TEST_COMPLEX_SECRET}").unwrap();
        interpolate(&mut value, &p()).unwrap();
        assert_eq!(value["value"].as_str(), Some(secret));
        unsafe {
            std::env::remove_var("SOCIAL_LINK_CFG_TEST_COMPLEX_SECRET");
        }
    }

    #[test]
    fn uses_default_when_var_unset() {
        unsafe {
            std::env::remove_var("SOCIAL_LINK_CFG_TEST_DEFAULT_UNSET");
        }
        let mut value: Value =
            serde_yaml::from_str("value: \"${SOCIAL_LINK_CFG_TEST_DEFAULT_UNSET:-fallback}\"")
                .unwrap();
        interpolate(&mut value, &p()).unwrap();
        assert_eq!(value["value"].as_str(), Some("fallback"));
    }

    #[test]
    fn default_is_ignored_when_var_is_set() {
        unsafe {
            std::env::set_var("SOCIAL_LINK_CFG_TEST_DEFAULT_SET", "actual");
        }
        let mut value: Value =
            serde_yaml::from_str("value: \"${SOCIAL_LINK_CFG_TEST_DEFAULT_SET:-fallback}\"")
                .unwrap();
        interpolate(&mut value, &p()).unwrap();
        assert_eq!(value["value"].as_str(), Some("actual"));
        unsafe {
            std::env::remove_var("SOCIAL_LINK_CFG_TEST_DEFAULT_SET");
        }
    }

    #[test]
    fn default_is_used_when_var_is_empty() {
        unsafe {
            std::env::set_var("SOCIAL_LINK_CFG_TEST_DEFAULT_EMPTY", "");
        }
        let mut value: Value =
            serde_yaml::from_str("value: \"${SOCIAL_LINK_CFG_TEST_DEFAULT_EMPTY:-fallback}\"")
                .unwrap();
        interpolate(&mut value, &p()).unwrap();
        assert_eq!(value["value"].as_str(), Some("fallback"));
        unsafe {
            std::env::remove_var("SOCIAL_LINK_CFG_TEST_DEFAULT_EMPTY");
        }
    }
}
