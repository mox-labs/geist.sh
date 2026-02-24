//! geist — composition root for the governed runtime.
//!
//! Wires extension crates into the processor registry, builds the pipeline
//! from config, and (future) starts the adapter.

use geist_edge::prelude::*;

// Link extension crates so inventory collects their registrations.
// The `use` ensures the linker doesn't dead-strip the crate.
use geist_access_control as _;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Build registry from all self-registered extensions.
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

    // TODO(P2): load pipeline config from file/env, create pipeline, start adapter.
    tracing::info!("geist: registry ready, adapter not yet implemented");
}
