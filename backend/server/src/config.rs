use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const DEFAULT_CONFIG_FILE: &str = "friendarazzi-server.json";

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config file `{path}` does not exist")]
    NotFound { path: String },
    #[error("failed to read config file `{path}`: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("config file `{path}` is malformed: {message}")]
    Malformed { path: String, message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ServerConfig {
    pub bind_address: String,
    pub bind_port: u16,
    pub database_url: String,
    pub object_store_dir: PathBuf,
    pub log_dir: Option<PathBuf>,
    /// PKCS#8 PEM private key that signs access tokens; absent means "generate one per process".
    pub signing_key_path: Option<PathBuf>,
    pub token_issuer: String,
    pub token_audience: String,
    /// Frontend origin the mailed links point at.
    pub frontend_basis_url: String,
    pub public_url: Option<String>,
    /// SMTP relay host. Absent means "no relay": every send fails and is logged.
    pub smtp_host: Option<String>,
    pub smtp_port: u16,
    pub smtp_verschlüsselung: SmtpVerschlüsselung,
    pub smtp_benutzername: Option<String>,
    pub smtp_passwort: Option<String>,
    /// Envelope sender of every transactional mail.
    pub smtp_absender: String,
    /// OTLP HTTP traces endpoint (e.g. `http://openobserve:5080/api/default/v1/traces`).
    pub otlp_endpoint: Option<String>,
    /// Optional Basic auth username for the OTLP endpoint.
    pub otlp_username: Option<String>,
    /// Optional Basic auth password for the OTLP endpoint.
    pub otlp_password: Option<String>,
    /// Target stream name header for OTLP traces (OpenObserve `stream-name`).
    pub otlp_stream_name: String,
    /// OpenTelemetry `service.name` resource attribute.
    pub otel_service_name: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1".into(),
            bind_port: 3000,
            database_url: "friendarazzi.db".into(),
            object_store_dir: PathBuf::from("storage/"),
            log_dir: None,
            signing_key_path: None,
            token_issuer: "friendarazzi.app".into(),
            token_audience: "friendarazzi-api".into(),
            frontend_basis_url: "http://localhost:3001".into(),
            public_url: None,
            smtp_host: None,
            smtp_port: 1025,
            smtp_verschlüsselung: SmtpVerschlüsselung::Klar,
            smtp_benutzername: None,
            smtp_passwort: None,
            smtp_absender: "noreply@friendarazzi.app".into(),
            otlp_endpoint: None,
            otlp_username: None,
            otlp_password: None,
            otlp_stream_name: "friendarazzi_traces".into(),
            otel_service_name: "friendarazzi-server".into(),
        }
    }
}

/// How the SMTP adapter talks to its relay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, clap::ValueEnum)]
#[serde(rename_all = "camelCase")]
pub enum SmtpVerschlüsselung {
    /// Plain SMTP: Mailpit on loopback in development and tests.
    #[default]
    Klar,
    /// `STARTTLS` upgrade, requiring a valid certificate.
    StartTls,
    /// TLS from the first byte (submission port 465).
    Implizit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Seed the database with sample development data and exit
    Seed,
}

#[derive(Debug, Parser, Default, PartialEq, Eq)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Seed the database with sample development data before starting the server
    #[arg(long)]
    pub seed: bool,

    /// Path to the config file (`.json` or `.toml`)
    #[arg(long = "config-path", global = true, env = "FRIENDARAZZI_CONFIG_PATH")]
    pub config_path: Option<PathBuf>,

    /// IP address to bind to
    #[arg(long, global = true, env = "FRIENDARAZZI_BIND_ADDRESS")]
    pub bind_address: Option<String>,

    /// Port to bind to
    #[arg(long, global = true, env = "FRIENDARAZZI_BIND_PORT")]
    pub bind_port: Option<u16>,

    /// Subpath this service is hosted on
    #[arg(long, global = true, env = "SUBPATH")]
    pub subpath: Option<String>,

    /// Database URL
    #[arg(long, global = true, env = "FRIENDARAZZI_DATABASE_URL")]
    pub database_url: Option<String>,

    /// Directory for object store
    #[arg(long, global = true, env = "FRIENDARAZZI_OBJECT_STORE_DIR")]
    pub object_store_dir: Option<PathBuf>,

    /// Directory for rotated JSON log files (omit to disable logging)
    #[arg(long, global = true, env = "FRIENDARAZZI_LOG_DIR")]
    pub log_dir: Option<PathBuf>,

    /// PKCS#8 PEM private key that signs access tokens; omit to generate one per process
    #[arg(long, global = true, env = "FRIENDARAZZI_SIGNING_KEY_PATH")]
    pub signing_key_path: Option<PathBuf>,

    /// Issuer claim of every access token
    #[arg(long, global = true, env = "FRIENDARAZZI_TOKEN_ISSUER")]
    pub token_issuer: Option<String>,

    /// Audience claim of every access token
    #[arg(long, global = true, env = "FRIENDARAZZI_TOKEN_AUDIENCE")]
    pub token_audience: Option<String>,

    /// Frontend origin the mailed links point at
    #[arg(long, global = true, env = "FRIENDARAZZI_FRONTEND_BASIS_URL")]
    pub frontend_basis_url: Option<String>,

    /// Public URL of the server
    #[arg(long, global = true, env = "FRIENDARAZZI_PUBLIC_URL")]
    pub public_url: Option<String>,

    /// SMTP relay host; omit to run without mail delivery
    #[arg(long, global = true, env = "FRIENDARAZZI_SMTP_HOST")]
    pub smtp_host: Option<String>,

    /// SMTP relay port
    #[arg(long, global = true, env = "FRIENDARAZZI_SMTP_PORT")]
    pub smtp_port: Option<u16>,

    /// SMTP transport: klar, start-tls, or implizit
    #[arg(long, global = true, env = "FRIENDARAZZI_SMTP_VERSCHLUESSELUNG")]
    pub smtp_verschlüsselung: Option<SmtpVerschlüsselung>,

    /// SMTP username
    #[arg(long, global = true, env = "FRIENDARAZZI_SMTP_BENUTZERNAME")]
    pub smtp_benutzername: Option<String>,

    /// SMTP password
    #[arg(long, global = true, env = "FRIENDARAZZI_SMTP_PASSWORT")]
    pub smtp_passwort: Option<String>,

    /// Envelope sender of transactional mail
    #[arg(long, global = true, env = "FRIENDARAZZI_SMTP_ABSENDER")]
    pub smtp_absender: Option<String>,

    /// OTLP HTTP traces endpoint (omit to disable OTLP trace export)
    #[arg(long, global = true, env = "FRIENDARAZZI_OTLP_ENDPOINT")]
    pub otlp_endpoint: Option<String>,

    /// Optional Basic auth username for the OTLP endpoint
    #[arg(long, global = true, env = "FRIENDARAZZI_OTLP_USERNAME")]
    pub otlp_username: Option<String>,

    /// Optional Basic auth password for the OTLP endpoint
    #[arg(long, global = true, env = "FRIENDARAZZI_OTLP_PASSWORD")]
    pub otlp_password: Option<String>,

    /// OpenObserve `stream-name` header value for OTLP traces
    #[arg(long, global = true, env = "FRIENDARAZZI_OTLP_STREAM_NAME")]
    pub otlp_stream_name: Option<String>,

    /// OpenTelemetry `service.name` resource attribute
    #[arg(long, global = true, env = "FRIENDARAZZI_OTEL_SERVICE_NAME")]
    pub otel_service_name: Option<String>,
}

pub fn load() -> Result<(ServerConfig, Cli), ConfigError> {
    let cli = Cli::parse();
    let config = resolve_ref(&cli)?;
    Ok((config, cli))
}

fn resolve_ref(cli: &Cli) -> Result<ServerConfig, ConfigError> {
    let explicit = cli.config_path.is_some();
    let path = cli
        .config_path
        .clone()
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE));
    let file = load_file(&path, explicit)?;
    Ok(overlay(file, cli))
}

fn load_file(path: &Path, explicit: bool) -> Result<ServerConfig, ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(contents) => match parse_config_file::<ServerConfig>(path, &contents) {
            Ok(config) => Ok(config),
            Err(message) => {
                if explicit {
                    Err(ConfigError::Malformed {
                        path: path.display().to_string(),
                        message,
                    })
                } else {
                    tracing::warn!(
                        path = %path.display(),
                        error = %message,
                        "config file is malformed; using defaults"
                    );
                    Ok(ServerConfig::default())
                }
            }
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            if explicit {
                Err(ConfigError::NotFound {
                    path: path.display().to_string(),
                })
            } else {
                tracing::warn!(
                    path = %path.display(),
                    "config file does not exist; using defaults"
                );
                Ok(ServerConfig::default())
            }
        }
        Err(source) => Err(ConfigError::Io {
            path: path.display().to_string(),
            source,
        }),
    }
}

fn overlay(mut config: ServerConfig, cli: &Cli) -> ServerConfig {
    if let Some(bind_address) = &cli.bind_address {
        config.bind_address = bind_address.clone();
    }
    if let Some(bind_port) = cli.bind_port {
        config.bind_port = bind_port;
    }
    if let Some(database_url) = &cli.database_url {
        config.database_url = database_url.clone();
    }
    if let Some(object_store_dir) = &cli.object_store_dir {
        config.object_store_dir = object_store_dir.clone();
    }
    if let Some(log_dir) = &cli.log_dir {
        config.log_dir = Some(log_dir.clone());
    }
    if cli.signing_key_path.is_some() {
        config.signing_key_path = cli.signing_key_path.clone();
    }
    if let Some(token_issuer) = &cli.token_issuer {
        config.token_issuer = token_issuer.clone();
    }
    if let Some(token_audience) = &cli.token_audience {
        config.token_audience = token_audience.clone();
    }
    if let Some(frontend_basis_url) = &cli.frontend_basis_url {
        config.frontend_basis_url = frontend_basis_url.clone();
    }
    if let Some(public_url) = &cli.public_url {
        config.public_url = Some(public_url.clone());
    }
    if let Some(smtp_host) = &cli.smtp_host {
        config.smtp_host = Some(smtp_host.clone());
    }
    if let Some(smtp_port) = cli.smtp_port {
        config.smtp_port = smtp_port;
    }
    if let Some(smtp_verschlüsselung) = cli.smtp_verschlüsselung {
        config.smtp_verschlüsselung = smtp_verschlüsselung;
    }
    if let Some(smtp_benutzername) = &cli.smtp_benutzername {
        config.smtp_benutzername = Some(smtp_benutzername.clone());
    }
    if let Some(smtp_passwort) = &cli.smtp_passwort {
        config.smtp_passwort = Some(smtp_passwort.clone());
    }
    if let Some(smtp_absender) = &cli.smtp_absender {
        config.smtp_absender = smtp_absender.clone();
    }
    if let Some(otlp_endpoint) = &cli.otlp_endpoint {
        config.otlp_endpoint = Some(otlp_endpoint.clone());
    }
    if let Some(otlp_username) = &cli.otlp_username {
        config.otlp_username = Some(otlp_username.clone());
    }
    if let Some(otlp_password) = &cli.otlp_password {
        config.otlp_password = Some(otlp_password.clone());
    }
    if let Some(otlp_stream_name) = &cli.otlp_stream_name {
        config.otlp_stream_name = otlp_stream_name.clone();
    }
    if let Some(otel_service_name) = &cli.otel_service_name {
        config.otel_service_name = otel_service_name.clone();
    }
    config
}

fn parse_config_file<T: serde::de::DeserializeOwned>(
    path: &Path,
    contents: &str,
) -> Result<T, String> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("toml") => toml::from_str(contents).map_err(|err| err.to_string()),
        Some("json") => serde_json::from_str(contents).map_err(|err| err.to_string()),
        Some(ext) => Err(format!(
            "unsupported config format `.{ext}`; use `.json` or `.toml`"
        )),
        None => serde_json::from_str(contents).map_err(|err| err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cli_overlay(cli: Cli) -> ServerConfig {
        overlay(ServerConfig::default(), &cli)
    }

    #[test_log::test]
    fn defaults_when_no_overlay() {
        assert_eq!(cli_overlay(Cli::default()), ServerConfig::default());
    }

    #[test_log::test]
    fn file_values_used_when_no_overlay() {
        let file = ServerConfig {
            database_url: "from-file.db".into(),
            bind_port: 4000,
            ..ServerConfig::default()
        };
        assert_eq!(overlay(file.clone(), &Cli::default()), file);
    }

    #[test_log::test]
    fn env_or_cli_overlay_beats_file() {
        let file = ServerConfig {
            database_url: "from-file.db".into(),
            bind_port: 4000,
            bind_address: "0.0.0.0".into(),
            ..ServerConfig::default()
        };
        let cli = Cli {
            database_url: Some("from-env.db".into()),
            bind_port: Some(5000),
            ..Cli::default()
        };
        let resolved = overlay(file, &cli);
        assert_eq!(resolved.database_url, "from-env.db");
        assert_eq!(resolved.bind_port, 5000);
        assert_eq!(resolved.bind_address, "0.0.0.0");
    }

    /// The deployment sets this through Compose, where a rejected spelling stops the
    /// container at startup. `clap` spells a `ValueEnum` kebab-case, so the environment
    /// value must be `klar`, not the serialised `camelCase` name.
    #[test_log::test]
    fn smtp_verschlüsselung_wird_in_kebab_schreibweise_gelesen() {
        let klar = Cli::try_parse_from(["friendarazzi-server"]).unwrap();
        assert_eq!(klar.smtp_verschlüsselung, None);

        let start =
            Cli::try_parse_from(["friendarazzi-server", "--smtp-verschlüsselung", "start-tls"])
                .expect("start-tls ist der von Compose erwartete Wert");
        assert_eq!(
            start.smtp_verschlüsselung,
            Some(SmtpVerschlüsselung::StartTls)
        );

        // The serialised spelling is not an accepted CLI value, which is why compose must
        // never carry it. Guarding it here keeps the two spellings from drifting back.
        assert!(
            Cli::try_parse_from(["friendarazzi-server", "--smtp-verschlüsselung", "startTls"])
                .is_err()
        );
    }

    #[test_log::test]
    fn overlays_log_dir() {
        let cli = Cli {
            log_dir: Some(PathBuf::from("/var/log/friendarazzi")),
            ..Cli::default()
        };
        let resolved = cli_overlay(cli);
        assert_eq!(
            resolved.log_dir,
            Some(PathBuf::from("/var/log/friendarazzi"))
        );
    }

    #[test_log::test]
    fn loads_log_dir_from_json_file() {
        let dir = std::env::temp_dir().join(format!(
            "friendarazzi-server-config-log-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{
                "logDir": "server-logs/"
            }"#,
        )
        .unwrap();

        let loaded = load_file(&path, true).unwrap();
        assert_eq!(loaded.log_dir, Some(PathBuf::from("server-logs/")));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test_log::test]
    fn loads_log_dir_from_toml_file() {
        let dir = std::env::temp_dir().join(format!(
            "friendarazzi-server-config-log-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(
            &path,
            r#"
logDir = "server-logs/"
"#,
        )
        .unwrap();

        let loaded = load_file(&path, true).unwrap();
        assert_eq!(loaded.log_dir, Some(PathBuf::from("server-logs/")));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test_log::test]
    fn missing_default_file_uses_defaults() {
        let path = PathBuf::from("definitely-missing-friendarazzi-server.json");
        let loaded = load_file(&path, false).unwrap();
        assert_eq!(loaded, ServerConfig::default());
    }

    #[test_log::test]
    fn missing_explicit_file_errors() {
        let path = PathBuf::from("definitely-missing-friendarazzi-server.json");
        let err = load_file(&path, true).unwrap_err();
        assert!(matches!(err, ConfigError::NotFound { .. }));
    }

    #[test_log::test]
    fn loads_json_file() {
        let dir = std::env::temp_dir().join(format!(
            "friendarazzi-server-config-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            r#"{
                "databaseUrl": "file.db",
                "objectStoreDir": "objects/",
                "bindAddress": "10.0.0.1",
                "bindPort": 1234,
                "publicUrl": "http://test.friendarazzi.app/"
            }"#,
        )
        .unwrap();

        let loaded = load_file(&path, true).unwrap();
        assert_eq!(loaded.database_url, "file.db");
        assert_eq!(loaded.object_store_dir, PathBuf::from("objects/"));
        assert_eq!(loaded.bind_address, "10.0.0.1");
        assert_eq!(loaded.bind_port, 1234);
        assert_eq!(
            loaded.public_url,
            Some("http://test.friendarazzi.app/".into())
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test_log::test]
    fn malformed_explicit_file_errors() {
        let dir = std::env::temp_dir().join(format!(
            "friendarazzi-server-config-bad-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.json");
        std::fs::write(&path, "{ not json").unwrap();

        let err = load_file(&path, true).unwrap_err();
        assert!(matches!(err, ConfigError::Malformed { .. }));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test_log::test]
    fn clap_cli_wins_over_env_for_config_path() {
        let cli =
            Cli::try_parse_from(["friendarazzi-server", "--config-path", "/tmp/cli.json"]).unwrap();
        assert_eq!(cli.config_path, Some(PathBuf::from("/tmp/cli.json")));
    }

    #[test_log::test]
    fn overlays_otlp_settings() {
        let cli = Cli {
            otlp_endpoint: Some("http://openobserve:5080/api/default/v1/traces".into()),
            otlp_username: Some("ops@example.invalid".into()),
            otlp_password: Some("secret".into()),
            otlp_stream_name: Some("custom_traces".into()),
            otel_service_name: Some("friendarazzi-test".into()),
            ..Cli::default()
        };
        let resolved = cli_overlay(cli);
        assert_eq!(
            resolved.otlp_endpoint.as_deref(),
            Some("http://openobserve:5080/api/default/v1/traces")
        );
        assert_eq!(
            resolved.otlp_username.as_deref(),
            Some("ops@example.invalid")
        );
        assert_eq!(resolved.otlp_password.as_deref(), Some("secret"));
        assert_eq!(resolved.otlp_stream_name, "custom_traces");
        assert_eq!(resolved.otel_service_name, "friendarazzi-test");
    }

    #[test_log::test]
    fn parses_seed_subcommand_with_global_flags_before_or_after() {
        let after = Cli::try_parse_from([
            "friendarazzi-server",
            "seed",
            "--database-url",
            "/tmp/seed.db",
        ])
        .unwrap();
        assert_eq!(after.command, Some(Command::Seed));
        assert!(!after.seed);
        assert_eq!(after.database_url.as_deref(), Some("/tmp/seed.db"));

        let before = Cli::try_parse_from([
            "friendarazzi-server",
            "--database-url",
            "/tmp/seed.db",
            "seed",
        ])
        .unwrap();
        assert_eq!(before.command, Some(Command::Seed));
        assert_eq!(before.database_url.as_deref(), Some("/tmp/seed.db"));
    }

    #[test_log::test]
    fn parses_seed_flag_for_server_startup() {
        let default = Cli::try_parse_from(["friendarazzi-server"]).unwrap();
        assert_eq!(default.command, None);
        assert!(!default.seed);

        let with_flag = Cli::try_parse_from(["friendarazzi-server", "--seed"]).unwrap();
        assert_eq!(with_flag.command, None);
        assert!(with_flag.seed);
    }
}
