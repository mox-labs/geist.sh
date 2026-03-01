//! geist — composition root for the governed runtime.
//!
//! Links extension crates so `inventory` collects their self-registrations.
//! Loads pipeline config, builds the processor registry, and starts the
//! governed reverse proxy.

use geist_edge::prelude::*;

// Link extension crates so inventory collects their self-registrations.
use geist_acl as _;
use geist_log as _;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Build registry — collect_extensions() picks up everything linked.
    let registry = ProcessorRegistryBuilder::new()
        .collect_extensions()
        .build();

    tracing::info!(
        extensions = registry.type_urls().len(),
        "processor registry built"
    );

    for url in registry.type_urls() {
        tracing::info!(type_url = url, "registered");
    }

    // Load proxy config.
    let config_path = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("GEIST_CONFIG").ok())
        .unwrap_or_else(|| {
            eprintln!("usage: geist-sh <config.json>");
            eprintln!("   or: GEIST_CONFIG=config.json geist-sh");
            std::process::exit(1);
        });

    let config_str = std::fs::read_to_string(&config_path).unwrap_or_else(|e| {
        eprintln!("failed to read config '{}': {e}", config_path);
        std::process::exit(1);
    });

    let config: geist_edge::adapter::axum::ProxyConfig =
        serde_json::from_str(&config_str).unwrap_or_else(|e| {
            eprintln!("invalid config '{}': {e}", config_path);
            std::process::exit(1);
        });

    // Start the governed proxy.
    if let Err(e) = geist_edge::adapter::axum::serve(config, &registry).await {
        eprintln!("server error: {e}");
        std::process::exit(1);
    }
}
