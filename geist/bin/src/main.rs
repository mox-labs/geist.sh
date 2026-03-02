//! geist — composition root for the governed runtime.
//!
//! Processors are built into geist-edge. Extensions register themselves
//! via `inventory::submit!` — core never knows they exist.

use geist_edge::prelude::*;

fn main() {
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

    // TODO(P2): load pipeline config, create pipeline, start adapter.
    tracing::info!("geist: registry ready, adapter not yet implemented");
}
