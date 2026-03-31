use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;

// Serializes tests that mutate shared, well-known environment variables
// (JWT_SECRET, IP_HASH_SALT, ADMIN_PASSWORD) so they don't race with each
// other when the test binary runs multi-threaded.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn unique_temp_root(label: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "social-link-config-test-{label}-{}-{}-{}",
        std::process::id(),
        nanos,
        n
    ));
    fs::create_dir_all(&dir).expect("create temp root");
    dir
}

const VALID_SERVER_YAML: &str = r#"
server:
  host: 0.0.0.0
  port: 3001
  cors_origins:
    - http://localhost:3000

logging:
  provider: local
  level: info
  directives:
    - "info"
    - "tower_http=info"
    - "axum=info"
  format: text
  config:
    file: /data/logs/social-link.log
    mirror_stdout: true
    rotation:
      max_size_mb: 100
      max_files: 5

auth:
  jwt_secret: ${JWT_SECRET}
  jwt_ttl_hours: 168
  cookie_secure: false
  ip_hash_salt: ${IP_HASH_SALT}

database:
  provider: mongo
  config:
    host: mongo
    port: 27017
    db: social-link

timeseries:
  provider: mongo
  config:
    host: mongo
    port: 27017
    db: social-link
    collection: events

storage:
  provider: local
  config:
    base_path: /data/uploads
    route_prefix: /uploads

cache:
  provider: none
  ttl_seconds: 300
"#;

const VALID_SOCIAL_LINK_YAML: &str = r#"
application:
  mode: single

admin:
  username: admin
  email: admin@example.com
  password: ${ADMIN_PASSWORD}
  display_name: Your Name

uploads:
  max_mb: 8

themes:
  seed_file: /app/theme.json
  max_preset_per_user: 3
  max_custom_per_user: 5
"#;

fn write_config(root: &Path, env: &str, server_yaml: &str, social_yaml: &str) {
    let dir = root.join("config").join(env);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("server.yaml"), server_yaml).unwrap();
    fs::write(dir.join("social-link.yaml"), social_yaml).unwrap();
}

fn set_secrets() {
    unsafe {
        std::env::set_var("JWT_SECRET", "test-jwt-secret");
        std::env::set_var("IP_HASH_SALT", "test-ip-hash-salt");
        std::env::set_var("ADMIN_PASSWORD", "test-admin-password");
    }
}

fn clear_secrets() {
    unsafe {
        std::env::remove_var("JWT_SECRET");
        std::env::remove_var("IP_HASH_SALT");
        std::env::remove_var("ADMIN_PASSWORD");
    }
}

#[test]
fn loads_valid_config_for_default_env_path() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("default-env");
    write_config(
        &root,
        DEFAULT_ENV,
        VALID_SERVER_YAML,
        VALID_SOCIAL_LINK_YAML,
    );
    set_secrets();

    let config = Config::load_from_root(&root, DEFAULT_ENV).expect("config should load");

    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.server.port, 3001);
    assert_eq!(
        config.server.cors_origins,
        vec!["http://localhost:3000".to_string()]
    );
    assert_eq!(config.auth.jwt_secret, "test-jwt-secret");
    assert_eq!(config.auth.jwt_ttl_hours, 168);
    assert_eq!(config.auth.ip_hash_salt, "test-ip-hash-salt");
    assert_eq!(config.admin.password, "test-admin-password");
    assert_eq!(config.database.provider, DbProvider::Mongo);
    assert_eq!(config.database.host.as_deref(), Some("mongo"));
    assert_eq!(config.timeseries.collection, "events");
    assert_eq!(config.storage.route_prefix, "/uploads");
    assert_eq!(config.cache.provider, CacheProviderConfig::None);
    assert_eq!(config.cache.ttl_seconds, 300);
    assert_eq!(
        config.logging.directives,
        vec![
            "info".to_string(),
            "tower_http=info".to_string(),
            "axum=info".to_string(),
        ]
    );
    assert_eq!(config.application.mode, AppMode::Single);
    assert!(!config.application.mode.is_multi());

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn interpolation_preserves_yaml_syntax_inside_secrets() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("complex-secrets");
    write_config(
        &root,
        DEFAULT_ENV,
        VALID_SERVER_YAML,
        VALID_SOCIAL_LINK_YAML,
    );
    let jwt_secret = "jwt: value # not a comment\nwith \"quotes\"";
    let ip_salt = "salt: value # literal";
    let admin_password = "password: value # literal";
    unsafe {
        std::env::set_var("JWT_SECRET", jwt_secret);
        std::env::set_var("IP_HASH_SALT", ip_salt);
        std::env::set_var("ADMIN_PASSWORD", admin_password);
    }

    let config = Config::load_from_root(&root, DEFAULT_ENV).expect("config should load");
    assert_eq!(config.auth.jwt_secret, jwt_secret);
    assert_eq!(config.auth.ip_hash_salt, ip_salt);
    assert_eq!(config.admin.password, admin_password);

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn fails_clearly_when_interpolated_var_missing() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("missing-var");
    write_config(&root, "local", VALID_SERVER_YAML, VALID_SOCIAL_LINK_YAML);
    // Deliberately do not set JWT_SECRET/IP_HASH_SALT/ADMIN_PASSWORD.
    clear_secrets();

    let err = Config::load_from_root(&root, "local").unwrap_err();
    assert!(matches!(err, ConfigError::MissingEnvVar(name) if name == "JWT_SECRET"));

    fs::remove_dir_all(&root).ok();
}

#[test]
fn jwt_ttl_hours_uses_interpolated_default_when_unset() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("jwt-ttl-default");
    set_secrets();
    unsafe {
        std::env::remove_var("JWT_TTL_HOURS");
    }
    let server_yaml = VALID_SERVER_YAML.replace(
        "jwt_ttl_hours: 168",
        "jwt_ttl_hours: \"${JWT_TTL_HOURS:-12}\"",
    );
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let config = Config::load_from_root(&root, "local").expect("config should load");
    assert_eq!(config.auth.jwt_ttl_hours, 12);

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn jwt_ttl_hours_env_override_beats_default() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("jwt-ttl-override");
    set_secrets();
    unsafe {
        std::env::set_var("JWT_TTL_HOURS", "6");
    }
    let server_yaml = VALID_SERVER_YAML.replace(
        "jwt_ttl_hours: 168",
        "jwt_ttl_hours: \"${JWT_TTL_HOURS:-12}\"",
    );
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let config = Config::load_from_root(&root, "local").expect("config should load");
    assert_eq!(config.auth.jwt_ttl_hours, 6);

    unsafe {
        std::env::remove_var("JWT_TTL_HOURS");
    }
    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn fails_when_jwt_ttl_hours_is_not_an_integer() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("jwt-ttl-nan");
    set_secrets();
    let server_yaml =
        VALID_SERVER_YAML.replace("jwt_ttl_hours: 168", "jwt_ttl_hours: \"not-a-number\"");
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let err = Config::load_from_root(&root, "local").unwrap_err();
    assert!(matches!(err, ConfigError::Parse { .. }));

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn fails_when_jwt_ttl_hours_is_zero() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("jwt-ttl-zero");
    set_secrets();
    let server_yaml = VALID_SERVER_YAML.replace("jwt_ttl_hours: 168", "jwt_ttl_hours: 0");
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let err = Config::load_from_root(&root, "local").unwrap_err();
    assert!(matches!(err, ConfigError::Validation { .. }));

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn fails_on_malformed_yaml() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("malformed");
    set_secrets();
    write_config(
        &root,
        "local",
        "server: [this is not, valid: yaml",
        VALID_SOCIAL_LINK_YAML,
    );

    let err = Config::load_from_root(&root, "local").unwrap_err();
    assert!(matches!(err, ConfigError::Parse { .. }));

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn fails_on_unknown_field() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("unknown-field");
    set_secrets();
    let server_yaml = VALID_SERVER_YAML.replace(
        "cache:\n  provider: none\n  ttl_seconds: 300",
        "cache:\n  provider: none\n  ttl_seconds: 300\n  not_a_real_field: 1",
    );
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let err = Config::load_from_root(&root, "local").unwrap_err();
    assert!(matches!(err, ConfigError::Parse { .. }));

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn fails_validation_when_route_prefix_missing_leading_slash() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("route-prefix");
    set_secrets();
    let server_yaml = VALID_SERVER_YAML.replace("route_prefix: /uploads", "route_prefix: uploads");
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let err = Config::load_from_root(&root, "local").unwrap_err();
    assert!(matches!(err, ConfigError::Validation { .. }));

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn fails_validation_when_route_prefix_is_not_the_ui_proxy_route() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("route-prefix-mismatch");
    set_secrets();
    let server_yaml = VALID_SERVER_YAML.replace("route_prefix: /uploads", "route_prefix: /media");
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let err = Config::load_from_root(&root, "local").unwrap_err();
    assert!(matches!(err, ConfigError::Validation { .. }));

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn normalizes_trailing_slash_on_upload_route_prefix() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("route-prefix-trailing-slash");
    set_secrets();
    let server_yaml =
        VALID_SERVER_YAML.replace("route_prefix: /uploads", "route_prefix: /uploads/");
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let config = Config::load_from_root(&root, "local").expect("config should load");
    assert_eq!(config.storage.route_prefix, "/uploads");

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn fails_validation_when_port_is_zero() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("zero-port");
    set_secrets();
    let server_yaml = VALID_SERVER_YAML.replace("port: 3001", "port: 0");
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let err = Config::load_from_root(&root, "local").unwrap_err();
    assert!(matches!(err, ConfigError::Validation { .. }));

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn fails_when_provider_is_unsupported() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("bad-provider");
    set_secrets();
    let server_yaml = VALID_SERVER_YAML.replace("provider: mongo", "provider: postgres");
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let err = Config::load_from_root(&root, "local").unwrap_err();
    assert!(matches!(err, ConfigError::Parse { .. }));

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn redis_cache_provider_requires_connection_string_and_key_prefix() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("redis-cache-missing");
    set_secrets();
    let server_yaml = VALID_SERVER_YAML.replace(
        "cache:\n  provider: none\n  ttl_seconds: 300",
        "cache:\n  provider: redis\n  ttl_seconds: 300",
    );
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let err = Config::load_from_root(&root, "local").unwrap_err();
    // The `config` mapping is missing the required redis fields entirely,
    // so this is caught as a strict deserialization failure.
    assert!(matches!(err, ConfigError::Parse { .. }));

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn redis_cache_provider_rejects_blank_connection_string() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("redis-cache-blank");
    set_secrets();
    let server_yaml = VALID_SERVER_YAML.replace(
        "cache:\n  provider: none\n  ttl_seconds: 300",
        "cache:\n  provider: redis\n  ttl_seconds: 300\n  config:\n    connection_string: \"\"\n    key_prefix: \"sl:\"",
    );
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let err = Config::load_from_root(&root, "local").unwrap_err();
    assert!(matches!(err, ConfigError::Validation { .. }));

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn redis_cache_provider_loads_when_fully_configured() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("redis-cache-ok");
    set_secrets();
    let server_yaml = VALID_SERVER_YAML.replace(
        "cache:\n  provider: none\n  ttl_seconds: 300",
        "cache:\n  provider: redis\n  ttl_seconds: 300\n  config:\n    connection_string: redis://redis:6379\n    key_prefix: \"sl:\"",
    );
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let config = Config::load_from_root(&root, "local").expect("config should load");
    match config.cache.provider {
        CacheProviderConfig::Redis(redis) => {
            assert_eq!(redis.connection_string, "redis://redis:6379");
            assert_eq!(redis.key_prefix, "sl:");
        }
        other => panic!("expected redis cache provider, got {other:?}"),
    }

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn in_process_cache_provider_uses_default_max_entries() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("in-process-cache");
    set_secrets();
    let server_yaml = VALID_SERVER_YAML.replace(
        "cache:\n  provider: none\n  ttl_seconds: 300",
        "cache:\n  provider: in_process\n  ttl_seconds: 300",
    );
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let config = Config::load_from_root(&root, "local").expect("config should load");
    match config.cache.provider {
        CacheProviderConfig::InProcess(cfg) => assert_eq!(cfg.max_entries, 10_000),
        other => panic!("expected in-process cache provider, got {other:?}"),
    }

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn in_process_cache_provider_rejects_zero_max_entries() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("in-process-cache-zero");
    set_secrets();
    let server_yaml = VALID_SERVER_YAML.replace(
        "cache:\n  provider: none\n  ttl_seconds: 300",
        "cache:\n  provider: in_process\n  ttl_seconds: 300\n  config:\n    max_entries: 0",
    );
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let err = Config::load_from_root(&root, "local").unwrap_err();
    assert!(matches!(err, ConfigError::Validation { .. }));

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn none_cache_provider_rejects_unexpected_config_fields() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("none-cache-strict");
    set_secrets();
    let server_yaml = VALID_SERVER_YAML.replace(
        "cache:\n  provider: none\n  ttl_seconds: 300",
        "cache:\n  provider: none\n  ttl_seconds: 300\n  config:\n    max_entries: 10",
    );
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let err = Config::load_from_root(&root, "local").unwrap_err();
    // `none` only accepts an empty `{}` config; any field is unknown.
    assert!(matches!(err, ConfigError::Parse { .. }));

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn none_cache_provider_loads_with_empty_config() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("none-cache-empty");
    set_secrets();
    let server_yaml = VALID_SERVER_YAML.replace(
        "cache:\n  provider: none\n  ttl_seconds: 300",
        "cache:\n  provider: none\n  ttl_seconds: 300\n  config: {}",
    );
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let config = Config::load_from_root(&root, "local").expect("config should load");
    assert_eq!(config.cache.provider, CacheProviderConfig::None);

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn fails_when_logging_directives_is_not_a_list() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("directives-not-list");
    set_secrets();
    let server_yaml = VALID_SERVER_YAML.replace(
        "  directives:\n    - \"info\"\n    - \"tower_http=info\"\n    - \"axum=info\"\n",
        "  directives: \"info,tower_http=info,axum=info\"\n",
    );
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let err = Config::load_from_root(&root, "local").unwrap_err();
    assert!(matches!(err, ConfigError::Parse { .. }));

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn fails_on_unsupported_app_mode() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("bad-app-mode");
    set_secrets();
    let social_yaml = VALID_SOCIAL_LINK_YAML.replace("mode: single", "mode: turbo");
    write_config(&root, "local", VALID_SERVER_YAML, &social_yaml);

    let err = Config::load_from_root(&root, "local").unwrap_err();
    assert!(matches!(err, ConfigError::Parse { .. }));

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn fails_when_config_directory_missing() {
    let root = unique_temp_root("missing-dir");
    let err = Config::load_from_root(&root, "nonexistent-env").unwrap_err();
    assert!(matches!(err, ConfigError::Read { .. }));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn tls_defaults_when_section_absent() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("tls-absent");
    set_secrets();
    write_config(&root, "local", VALID_SERVER_YAML, VALID_SOCIAL_LINK_YAML);

    let config = Config::load_from_root(&root, "local").expect("config should load");
    assert!(!config.tls.enabled);
    assert!(config.tls.http2, "http2 should default to true");
    assert!(!config.tls.http3);
    assert_eq!(config.tls.cert_path, None);
    assert_eq!(config.tls.key_path, None);

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn tls_loads_when_enabled_with_paths() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("tls-enabled");
    set_secrets();
    let server_yaml = format!(
        "{VALID_SERVER_YAML}\ntls:\n  enabled: true\n  cert_path: /app/certs/fullchain.pem\n  key_path: /app/certs/privkey.pem\n"
    );
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let config = Config::load_from_root(&root, "local").expect("config should load");
    assert!(config.tls.enabled);
    assert!(config.tls.http2, "http2 should default to true");
    assert!(!config.tls.http3);
    assert_eq!(config.tls.cert_path.as_deref(), Some("/app/certs/fullchain.pem"));
    assert_eq!(config.tls.key_path.as_deref(), Some("/app/certs/privkey.pem"));

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn tls_enabled_requires_cert_and_key() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("tls-missing-paths");
    set_secrets();
    let server_yaml = format!("{VALID_SERVER_YAML}\ntls:\n  enabled: true\n");
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let err = Config::load_from_root(&root, "local").unwrap_err();
    assert!(matches!(err, ConfigError::Validation { .. }));

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn tls_accepts_string_booleans_from_interpolation() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("tls-string-bools");
    set_secrets();
    // Mirrors the shipped config, where every switch is an interpolated string
    // such as `${UI_TLS_ENABLED:-false}` that always resolves to a string.
    let server_yaml = format!(
        "{VALID_SERVER_YAML}\ntls:\n  enabled: \"true\"\n  cert_path: /app/certs/fullchain.pem\n  key_path: /app/certs/privkey.pem\n  http2: \"false\"\n  http3: \"1\"\n"
    );
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let config = Config::load_from_root(&root, "local").expect("config should load");
    assert!(config.tls.enabled);
    assert!(!config.tls.http2);
    assert!(config.tls.http3);

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn tls_disabled_ignores_missing_paths() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("tls-disabled");
    set_secrets();
    // The shipped default: disabled with the placeholder paths still present.
    let server_yaml = format!(
        "{VALID_SERVER_YAML}\ntls:\n  enabled: \"false\"\n  cert_path: /app/certs/fullchain.pem\n  key_path: /app/certs/privkey.pem\n"
    );
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let config = Config::load_from_root(&root, "local").expect("config should load");
    assert!(!config.tls.enabled);

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}

#[test]
fn tls_rejects_unknown_field() {
    let _guard = ENV_LOCK.lock().unwrap();
    let root = unique_temp_root("tls-unknown-field");
    set_secrets();
    let server_yaml = format!("{VALID_SERVER_YAML}\ntls:\n  enabled: false\n  bogus: 1\n");
    write_config(&root, "local", &server_yaml, VALID_SOCIAL_LINK_YAML);

    let err = Config::load_from_root(&root, "local").unwrap_err();
    assert!(matches!(err, ConfigError::Parse { .. }));

    clear_secrets();
    fs::remove_dir_all(&root).ok();
}
