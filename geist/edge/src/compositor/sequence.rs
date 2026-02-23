//! Sequence compositor — linear processor pipeline.
//!
//! Runs processors in registration order for each phase. Short-circuits
//! on `ImmediateResponse` or `ProcessorError`. Cross-phase termination:
//! if any phase returns `Respond`, all subsequent phases are skipped.

use std::sync::Arc;

use envoy_grpc_ext_proc::envoy::service::ext_proc::v3::{HeaderMutation, ImmediateResponse};
use rumi_http::HttpMessage;

use crate::phase::{PhaseResult, ProcessingMode};
use crate::processor::{Processor, ProcessorError};

/// How the pipeline handles processor errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureMode {
    /// On error, generate an error response and stop (deny by default).
    FailClosed,
    /// On error, log and continue to the next processor (allow by default).
    FailOpen,
}

impl Default for FailureMode {
    fn default() -> Self {
        FailureMode::FailClosed
    }
}

/// Outcome of running a phase through the pipeline.
#[derive(Debug)]
pub enum PhaseOutcome {
    /// All processors returned Continue (possibly with accumulated mutations).
    Continue(Vec<HeaderMutation>),
    /// A processor returned ImmediateResponse — pipeline terminated.
    Respond(ImmediateResponse),
    /// A processor failed and failure_mode is FailClosed.
    Error(ProcessorError),
}

impl PhaseOutcome {
    /// Returns `true` if the pipeline should stop all subsequent phases.
    pub fn is_terminal(&self) -> bool {
        matches!(self, PhaseOutcome::Respond(_) | PhaseOutcome::Error(_))
    }
}

/// Linear processor pipeline.
///
/// Runs processors in registration order. For each phase:
/// 1. Iterate processors that participate in this phase (per `mode()`)
/// 2. Call the phase method
/// 3. On `Continue` — accumulate, move to next processor
/// 4. On `Mutate` — accumulate mutation, move to next processor
/// 5. On `Respond` — short-circuit, skip remaining processors AND phases
/// 6. On `Err` — handle per `failure_mode`
///
/// # Cross-Phase Termination (Dijkstra I2)
///
/// If any phase returns `Respond` or `Error` (in FailClosed mode),
/// ALL subsequent phases are skipped. The adapter constructs the response
/// directly from the `ImmediateResponse` or generates a 500.
pub struct Sequence {
    processors: Vec<Arc<dyn Processor>>,
    aggregate_mode: ProcessingMode,
    failure_mode: FailureMode,
}

impl Sequence {
    /// Create a new builder.
    pub fn builder() -> SequenceBuilder {
        SequenceBuilder::new()
    }

    /// The aggregate processing mode (union of all processor modes).
    pub fn mode(&self) -> ProcessingMode {
        self.aggregate_mode
    }

    /// Number of processors in the pipeline.
    pub fn len(&self) -> usize {
        self.processors.len()
    }

    /// Returns `true` if the pipeline has no processors.
    pub fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }

    /// Run the request headers phase.
    pub async fn process_request_headers(&self, msg: &HttpMessage) -> PhaseOutcome {
        let mut mutations = Vec::new();

        for proc in &self.processors {
            if !proc.mode().request_headers {
                continue;
            }

            match proc.process_request_headers(msg).await {
                Ok(PhaseResult::Continue) => {}
                Ok(PhaseResult::Mutate(mutation)) => {
                    mutations.push(mutation);
                }
                Ok(PhaseResult::Respond(response)) => {
                    return PhaseOutcome::Respond(response);
                }
                Err(err) => match self.failure_mode {
                    FailureMode::FailClosed => return PhaseOutcome::Error(err),
                    FailureMode::FailOpen => {
                        tracing::warn!(
                            processor = proc.name(),
                            error = %err,
                            "processor failed, continuing (fail-open)"
                        );
                    }
                },
            }
        }

        PhaseOutcome::Continue(mutations)
    }

    /// Run the request body phase.
    pub async fn process_request_body(&self, body: &[u8]) -> PhaseOutcome {
        if !self.aggregate_mode.request_body {
            return PhaseOutcome::Continue(vec![]);
        }

        let mut mutations = Vec::new();

        for proc in &self.processors {
            if !proc.mode().request_body {
                continue;
            }

            match proc.process_request_body(body).await {
                Ok(PhaseResult::Continue) => {}
                Ok(PhaseResult::Mutate(mutation)) => {
                    mutations.push(mutation);
                }
                Ok(PhaseResult::Respond(response)) => {
                    return PhaseOutcome::Respond(response);
                }
                Err(err) => match self.failure_mode {
                    FailureMode::FailClosed => return PhaseOutcome::Error(err),
                    FailureMode::FailOpen => {
                        tracing::warn!(
                            processor = proc.name(),
                            error = %err,
                            "processor failed, continuing (fail-open)"
                        );
                    }
                },
            }
        }

        PhaseOutcome::Continue(mutations)
    }

    /// Run the response headers phase.
    pub async fn process_response_headers(&self, msg: &HttpMessage) -> PhaseOutcome {
        let mut mutations = Vec::new();

        for proc in &self.processors {
            if !proc.mode().response_headers {
                continue;
            }

            match proc.process_response_headers(msg).await {
                Ok(PhaseResult::Continue) => {}
                Ok(PhaseResult::Mutate(mutation)) => {
                    mutations.push(mutation);
                }
                Ok(PhaseResult::Respond(response)) => {
                    return PhaseOutcome::Respond(response);
                }
                Err(err) => match self.failure_mode {
                    FailureMode::FailClosed => return PhaseOutcome::Error(err),
                    FailureMode::FailOpen => {
                        tracing::warn!(
                            processor = proc.name(),
                            error = %err,
                            "processor failed, continuing (fail-open)"
                        );
                    }
                },
            }
        }

        PhaseOutcome::Continue(mutations)
    }

    /// Run the response body phase.
    pub async fn process_response_body(&self, body: &[u8]) -> PhaseOutcome {
        if !self.aggregate_mode.response_body {
            return PhaseOutcome::Continue(vec![]);
        }

        let mut mutations = Vec::new();

        for proc in &self.processors {
            if !proc.mode().response_body {
                continue;
            }

            match proc.process_response_body(body).await {
                Ok(PhaseResult::Continue) => {}
                Ok(PhaseResult::Mutate(mutation)) => {
                    mutations.push(mutation);
                }
                Ok(PhaseResult::Respond(response)) => {
                    return PhaseOutcome::Respond(response);
                }
                Err(err) => match self.failure_mode {
                    FailureMode::FailClosed => return PhaseOutcome::Error(err),
                    FailureMode::FailOpen => {
                        tracing::warn!(
                            processor = proc.name(),
                            error = %err,
                            "processor failed, continuing (fail-open)"
                        );
                    }
                },
            }
        }

        PhaseOutcome::Continue(mutations)
    }
}

/// Builder for [`Sequence`] pipelines.
///
/// Validates mode consistency and computes aggregate mode at build time.
pub struct SequenceBuilder {
    processors: Vec<Arc<dyn Processor>>,
    failure_mode: FailureMode,
}

impl SequenceBuilder {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
            failure_mode: FailureMode::default(),
        }
    }

    /// Add a processor to the pipeline.
    ///
    /// Processors are executed in the order they are added.
    pub fn processor(mut self, proc: Arc<dyn Processor>) -> Self {
        self.processors.push(proc);
        self
    }

    /// Set the failure mode for the pipeline.
    pub fn failure_mode(mut self, mode: FailureMode) -> Self {
        self.failure_mode = mode;
        self
    }

    /// Build the pipeline.
    ///
    /// Computes the aggregate processing mode (union of all processor modes).
    pub fn build(self) -> Sequence {
        let aggregate_mode = self
            .processors
            .iter()
            .map(|p| p.mode())
            .fold(ProcessingMode::default(), |acc, m| acc.union(m));

        Sequence {
            processors: self.processors,
            aggregate_mode,
            failure_mode: self.failure_mode,
        }
    }
}

impl Default for SequenceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::processor::BoxFuture;
    use envoy_grpc_ext_proc::envoy::{
        config::core::v3::HeaderValueOption,
        r#type::v3::HttpStatus,
        service::ext_proc::v3::HeaderMutation,
    };

    // -- Test processors --

    /// Always continues.
    struct PassthroughProcessor;
    impl Processor for PassthroughProcessor {
        fn name(&self) -> &str {
            "passthrough"
        }
    }

    /// Denies with 403 on request headers.
    struct DenyProcessor;
    impl Processor for DenyProcessor {
        fn name(&self) -> &str {
            "deny"
        }
        fn process_request_headers(
            &self,
            _msg: &HttpMessage,
        ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
            Box::pin(async {
                Ok(PhaseResult::Respond(ImmediateResponse {
                    status: Some(HttpStatus { code: 403 }),
                    body: b"denied".to_vec(),
                    ..Default::default()
                }))
            })
        }
    }

    /// Adds a header mutation.
    struct MutatingProcessor {
        header_name: String,
        header_value: String,
    }
    impl Processor for MutatingProcessor {
        fn name(&self) -> &str {
            "mutating"
        }
        fn process_request_headers(
            &self,
            _msg: &HttpMessage,
        ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
            let name = self.header_name.clone();
            let value = self.header_value.clone();
            Box::pin(async move {
                Ok(PhaseResult::Mutate(HeaderMutation {
                    set_headers: vec![HeaderValueOption {
                        header: Some(
                            envoy_grpc_ext_proc::envoy::config::core::v3::HeaderValue {
                                key: name,
                                value,
                                raw_value: vec![],
                            },
                        ),
                        ..Default::default()
                    }],
                    remove_headers: vec![],
                }))
            })
        }
    }

    /// Always fails.
    struct FailingProcessor;
    impl Processor for FailingProcessor {
        fn name(&self) -> &str {
            "failing"
        }
        fn process_request_headers(
            &self,
            _msg: &HttpMessage,
        ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
            Box::pin(async { Err(ProcessorError::new("failing", "intentional failure")) })
        }
    }

    /// Processor that opts into body processing.
    struct BodyProcessor;
    impl Processor for BodyProcessor {
        fn name(&self) -> &str {
            "body"
        }
        fn mode(&self) -> ProcessingMode {
            ProcessingMode::FULL
        }
    }

    /// Records which phases were called (for verifying cross-phase termination).
    struct RecordingProcessor {
        request_headers_called: std::sync::atomic::AtomicBool,
        response_headers_called: std::sync::atomic::AtomicBool,
    }
    impl RecordingProcessor {
        fn new() -> Self {
            Self {
                request_headers_called: std::sync::atomic::AtomicBool::new(false),
                response_headers_called: std::sync::atomic::AtomicBool::new(false),
            }
        }
    }
    impl Processor for RecordingProcessor {
        fn name(&self) -> &str {
            "recording"
        }
        fn process_request_headers(
            &self,
            _msg: &HttpMessage,
        ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
            self.request_headers_called
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Box::pin(async { Ok(PhaseResult::Continue) })
        }
        fn process_response_headers(
            &self,
            _msg: &HttpMessage,
        ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
            self.response_headers_called
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Box::pin(async { Ok(PhaseResult::Continue) })
        }
    }

    fn empty_message() -> HttpMessage {
        use envoy_grpc_ext_proc::envoy::service::ext_proc::v3::{
            processing_request::Request, HttpHeaders, ProcessingRequest,
        };
        let req = ProcessingRequest {
            request: Some(Request::RequestHeaders(HttpHeaders::default())),
            ..Default::default()
        };
        HttpMessage::from(&req)
    }

    // -- Tests --

    #[tokio::test]
    async fn empty_pipeline_continues() {
        let seq = Sequence::builder().build();
        let msg = empty_message();
        let outcome = seq.process_request_headers(&msg).await;
        assert!(matches!(outcome, PhaseOutcome::Continue(m) if m.is_empty()));
    }

    #[tokio::test]
    async fn passthrough_continues() {
        let seq = Sequence::builder()
            .processor(Arc::new(PassthroughProcessor))
            .build();
        let msg = empty_message();
        let outcome = seq.process_request_headers(&msg).await;
        assert!(matches!(outcome, PhaseOutcome::Continue(m) if m.is_empty()));
    }

    #[tokio::test]
    async fn deny_short_circuits() {
        let seq = Sequence::builder()
            .processor(Arc::new(DenyProcessor))
            .processor(Arc::new(PassthroughProcessor))
            .build();
        let msg = empty_message();
        let outcome = seq.process_request_headers(&msg).await;
        assert!(matches!(outcome, PhaseOutcome::Respond(_)));
    }

    #[tokio::test]
    async fn mutations_accumulate() {
        let seq = Sequence::builder()
            .processor(Arc::new(MutatingProcessor {
                header_name: "x-first".into(),
                header_value: "1".into(),
            }))
            .processor(Arc::new(MutatingProcessor {
                header_name: "x-second".into(),
                header_value: "2".into(),
            }))
            .build();
        let msg = empty_message();
        let outcome = seq.process_request_headers(&msg).await;
        match outcome {
            PhaseOutcome::Continue(mutations) => {
                assert_eq!(mutations.len(), 2);
            }
            _ => panic!("expected Continue with mutations"),
        }
    }

    #[tokio::test]
    async fn deny_before_mutation_skips_mutation() {
        let seq = Sequence::builder()
            .processor(Arc::new(DenyProcessor))
            .processor(Arc::new(MutatingProcessor {
                header_name: "x-should-not-appear".into(),
                header_value: "nope".into(),
            }))
            .build();
        let msg = empty_message();
        let outcome = seq.process_request_headers(&msg).await;
        assert!(matches!(outcome, PhaseOutcome::Respond(_)));
    }

    #[tokio::test]
    async fn error_fail_closed() {
        let seq = Sequence::builder()
            .failure_mode(FailureMode::FailClosed)
            .processor(Arc::new(FailingProcessor))
            .processor(Arc::new(PassthroughProcessor))
            .build();
        let msg = empty_message();
        let outcome = seq.process_request_headers(&msg).await;
        assert!(matches!(outcome, PhaseOutcome::Error(_)));
    }

    #[tokio::test]
    async fn error_fail_open() {
        let seq = Sequence::builder()
            .failure_mode(FailureMode::FailOpen)
            .processor(Arc::new(FailingProcessor))
            .processor(Arc::new(PassthroughProcessor))
            .build();
        let msg = empty_message();
        let outcome = seq.process_request_headers(&msg).await;
        assert!(matches!(outcome, PhaseOutcome::Continue(_)));
    }

    #[tokio::test]
    async fn aggregate_mode_computed_from_processors() {
        let seq = Sequence::builder()
            .processor(Arc::new(PassthroughProcessor))
            .processor(Arc::new(BodyProcessor))
            .build();
        // BodyProcessor declares FULL mode, so aggregate should include body
        assert!(seq.mode().request_body);
        assert!(seq.mode().response_body);
    }

    #[tokio::test]
    async fn body_phase_skipped_when_no_processor_opts_in() {
        let seq = Sequence::builder()
            .processor(Arc::new(PassthroughProcessor))
            .build();
        assert!(!seq.mode().request_body);
        let outcome = seq.process_request_body(b"hello").await;
        assert!(matches!(outcome, PhaseOutcome::Continue(m) if m.is_empty()));
    }

    #[tokio::test]
    async fn cross_phase_termination_on_immediate_response() {
        // Deny on request headers → response headers should NOT be called
        let recorder = Arc::new(RecordingProcessor::new());
        let seq = Sequence::builder()
            .processor(Arc::new(DenyProcessor))
            .processor(recorder.clone())
            .build();

        let msg = empty_message();
        let outcome = seq.process_request_headers(&msg).await;

        // DenyProcessor fires first, recorder never gets request headers
        assert!(matches!(outcome, PhaseOutcome::Respond(_)));
        assert!(!recorder
            .request_headers_called
            .load(std::sync::atomic::Ordering::SeqCst));

        // The adapter is responsible for NOT calling response phases after
        // ImmediateResponse. Verify the Sequence correctly reports terminal.
        assert!(outcome.is_terminal());
    }
}
