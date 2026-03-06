//! geist — composition root for the governed runtime.
//!
//! Processors are built into geist-edge. Extensions register themselves
//! via `inventory::submit!` — core never knows they exist.

use std::sync::Arc;

use geist_edge::prelude::*;
use opentelemetry::trace::TracerProvider;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() {
    // OTel tracer provider — exports to OTLP (Jaeger/Tempo/collector).
    // Silently skips if no collector is reachable.
    let tracer_provider = init_tracer_provider();

    let telemetry = tracing_opentelemetry::layer()
        .with_tracer(tracer_provider.tracer("geist-edge"));

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(telemetry)
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Build registry — collects all inventory::submit! registrations.
    let registry = collect_processor_extensions().build();

    tracing::info!(
        extensions = registry.type_urls().len(),
        "processor registry built"
    );

    for url in registry.type_urls() {
        tracing::info!(type_url = url, "registered");
    }

    // Demo pipeline: ACL with deny-first policy.
    let acl_config = serde_json::json!({
        "deny": [
            {
                "reason": "Bash is not allowed",
                "matches": [{ "tool_name": { "Exact": "Bash" } }]
            }
        ],
        "allow": [
            {
                "matches": [{ "agent_id": { "Exact": "claude-main" } }]
            }
        ]
    });

    let pipeline_configs = vec![TypedConfig {
        type_url: "mox.geist.processors.v1.AccessControl".into(),
        config: acl_config,
    }];

    let processors = registry
        .create_pipeline(&pipeline_configs)
        .expect("failed to create pipeline");

    let mut builder = Sequence::builder();
    for proc in processors {
        builder = builder.processor(proc);
    }
    let pipeline = Arc::new(builder.build());

    tracing::info!(
        processors = pipeline.len(),
        "pipeline built"
    );

    // Start axum server.
    let app = geist_edge::adapter::axum::router(pipeline);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind :3000");

    tracing::info!("geist-edge listening on :3000");
    axum::serve(listener, app).await.expect("server error");
}

fn init_tracer_provider() -> opentelemetry_sdk::trace::SdkTracerProvider {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("failed to create OTLP exporter");

    opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .build()
}
