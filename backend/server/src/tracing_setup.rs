use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use friendarazzi_api::FriendarazziAppApi;
use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::{SpanExporter, WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing::Level;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::ServerConfig;

static LOG_GUARD: OnceLock<tracing_appender::non_blocking::WorkerGuard> = OnceLock::new();

pub struct TracingGuard {
    tracer_provider: Option<SdkTracerProvider>,
}

impl TracingGuard {
    pub fn shutdown(self) {
        if let Some(provider) = self.tracer_provider
            && let Err(err) = provider.shutdown()
        {
            eprintln!("failed to shut down OpenTelemetry tracer provider: {err:?}");
        }
    }
}

pub fn init_tracing(config: &ServerConfig) -> TracingGuard {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .with_env_var("FRIENDARAZZI_LOG_LEVEL")
        .from_env_lossy();

    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_filter(filter.clone());

    let (tracer_provider, otel_layer) = match build_otel_tracer(config, filter.clone()) {
        Some((provider, layer)) => (Some(provider), Some(layer)),
        None => (None, None),
    };

    let json_layer = config
        .log_dir
        .as_deref()
        .map(|log_dir| build_json_layer(log_dir, filter.clone()));

    tracing_subscriber::registry()
        .with(stdout_layer)
        .with(json_layer)
        .with(otel_layer)
        .init();

    if tracer_provider.is_some() {
        tracing::info!(
            stream = %config.otlp_stream_name,
            service = %config.otel_service_name,
            "OpenTelemetry OTLP trace export enabled"
        );
    }

    TracingGuard { tracer_provider }
}

fn build_json_layer<S>(log_dir: &Path, filter: EnvFilter) -> impl Layer<S>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    std::fs::create_dir_all(log_dir).expect("failed to create log directory");
    let file_appender = tracing_appender::rolling::daily(log_dir, "friendarazzi-server.json");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    LOG_GUARD.set(guard).ok();

    fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_filter(filter.add_directive(Level::DEBUG.into()))
}

fn build_otel_tracer<S>(
    config: &ServerConfig,
    filter: EnvFilter,
) -> Option<(SdkTracerProvider, impl Layer<S>)>
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    let endpoint = resolve_otlp_endpoint(config)?;

    let mut headers = HashMap::new();
    if !config.otlp_stream_name.trim().is_empty() {
        headers.insert("stream-name".to_owned(), config.otlp_stream_name.clone());
    }
    if let Some(auth_header) = build_basic_auth_header(config) {
        headers.insert("Authorization".to_owned(), auth_header);
    }

    let exporter = match SpanExporter::builder()
        .with_http()
        .with_endpoint(endpoint)
        .with_headers(headers)
        .build()
    {
        Ok(exporter) => exporter,
        Err(err) => {
            eprintln!("failed to build OTLP span exporter; continuing without OTLP: {err}");
            return None;
        }
    };

    let build = FriendarazziAppApi::build_version();
    let mut attributes = vec![KeyValue::new("service.version", build.version)];
    if let Some(commit) = build.commit {
        attributes.push(KeyValue::new("service.commit", commit));
    }
    let resource = Resource::builder()
        .with_service_name(config.otel_service_name.clone())
        .with_attributes(attributes)
        .build();

    let provider = SdkTracerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build();

    let tracer = provider.tracer(config.otel_service_name.clone());
    let layer = tracing_opentelemetry::layer()
        .with_tracer(tracer)
        .with_filter(filter.add_directive(Level::TRACE.into()));

    Some((provider, layer))
}

fn resolve_otlp_endpoint(config: &ServerConfig) -> Option<String> {
    if let Some(endpoint) = &config.otlp_endpoint {
        let trimmed = endpoint.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_owned());
        }
    }
    for env_key in [
        "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
        "OTEL_EXPORTER_OTLP_ENDPOINT",
    ] {
        if let Ok(val) = std::env::var(env_key) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                if env_key == "OTEL_EXPORTER_OTLP_ENDPOINT" && !trimmed.ends_with("/v1/traces") {
                    return Some(format!("{}/v1/traces", trimmed.trim_end_matches('/')));
                }
                return Some(trimmed.to_owned());
            }
        }
    }
    None
}

fn build_basic_auth_header(config: &ServerConfig) -> Option<String> {
    let username = config
        .otlp_username
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .or_else(|| non_empty_env("ZO_ROOT_USER_EMAIL"))?;

    let password = config
        .otlp_password
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .or_else(|| non_empty_env("ZO_ROOT_USER_PASSWORD"))?;

    let token = BASE64.encode(format!("{username}:{password}"));
    Some(format!("Basic {token}"))
}

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_log::test]
    fn basic_auth_header_encodes_explicit_credentials() {
        let config = ServerConfig {
            otlp_username: Some("ops@example.invalid".into()),
            otlp_password: Some("s3cr3t".into()),
            ..ServerConfig::default()
        };
        assert_eq!(
            build_basic_auth_header(&config).as_deref(),
            Some("Basic b3BzQGV4YW1wbGUuaW52YWxpZDpzM2NyM3Q=")
        );
    }

    #[test_log::test]
    fn otlp_disabled_when_endpoint_absent() {
        let config = ServerConfig::default();
        assert_eq!(resolve_otlp_endpoint(&config), None);
    }
}
