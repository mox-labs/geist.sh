//! Sequence compositor — linear processor pipeline.
//!
//! Runs processors in registration order for each phase. Short-circuits
//! on `ImmediateResponse` or `ProcessorError`. Cross-phase termination:
//! if any phase returns `Respond`, all subsequent phases are skipped.

use std::sync::Arc;

use tracing::Instrument;

use crate::phase::{HeaderMutations, ImmediateResponse, PhaseResult, ProcessingMode};
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
pub enum SequenceOutcome {
    /// All processors returned Continue (possibly with accumulated mutations).
    Continue(Vec<HeaderMutations>),
    /// A processor returned ImmediateResponse — pipeline terminated.
    Respond(ImmediateResponse),
    /// A processor failed and failure_mode is FailClosed.
    Error(ProcessorError),
}

impl SequenceOutcome {
    /// Returns `true` if the pipeline should stop all subsequent phases.
    pub fn is_terminal(&self) -> bool {
        matches!(self, SequenceOutcome::Respond(_) | SequenceOutcome::Error(_))
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
    ///
    /// All processors participate in header phases — there is no opt-out.
    pub async fn process_request_headers(
        &self,
        parts: &http::request::Parts,
    ) -> SequenceOutcome {
        let mut mutations = Vec::new();

        for proc in &self.processors {
            let span = tracing::info_span!("processor", name = proc.name(), phase = "request_headers");
            match proc.process_request_headers(parts).instrument(span).await {
                Ok(PhaseResult::Continue) => {}
                Ok(PhaseResult::Mutate(mutation)) => {
                    mutations.push(mutation);
                }
                Ok(PhaseResult::Respond(response)) => {
                    return SequenceOutcome::Respond(response);
                }
                Err(err) => match self.failure_mode {
                    FailureMode::FailClosed => return SequenceOutcome::Error(err),
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

        SequenceOutcome::Continue(mutations)
    }

    /// Run the request body phase.
    pub async fn process_request_body(&self, body: &[u8]) -> SequenceOutcome {
        if !self.aggregate_mode.request_body {
            return SequenceOutcome::Continue(vec![]);
        }

        let mut mutations = Vec::new();

        for proc in &self.processors {
            if !proc.mode().request_body {
                continue;
            }

            let span = tracing::info_span!("processor", name = proc.name(), phase = "request_body");
            match proc.process_request_body(body).instrument(span).await {
                Ok(PhaseResult::Continue) => {}
                Ok(PhaseResult::Mutate(mutation)) => {
                    mutations.push(mutation);
                }
                Ok(PhaseResult::Respond(response)) => {
                    return SequenceOutcome::Respond(response);
                }
                Err(err) => match self.failure_mode {
                    FailureMode::FailClosed => return SequenceOutcome::Error(err),
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

        SequenceOutcome::Continue(mutations)
    }

    /// Run the response headers phase.
    ///
    /// All processors participate in header phases — there is no opt-out.
    pub async fn process_response_headers(
        &self,
        parts: &http::response::Parts,
    ) -> SequenceOutcome {
        let mut mutations = Vec::new();

        for proc in &self.processors {
            let span = tracing::info_span!("processor", name = proc.name(), phase = "response_headers");
            match proc.process_response_headers(parts).instrument(span).await {
                Ok(PhaseResult::Continue) => {}
                Ok(PhaseResult::Mutate(mutation)) => {
                    mutations.push(mutation);
                }
                Ok(PhaseResult::Respond(response)) => {
                    return SequenceOutcome::Respond(response);
                }
                Err(err) => match self.failure_mode {
                    FailureMode::FailClosed => return SequenceOutcome::Error(err),
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

        SequenceOutcome::Continue(mutations)
    }

    /// Run the response body phase.
    pub async fn process_response_body(&self, body: &[u8]) -> SequenceOutcome {
        if !self.aggregate_mode.response_body {
            return SequenceOutcome::Continue(vec![]);
        }

        let mut mutations = Vec::new();

        for proc in &self.processors {
            if !proc.mode().response_body {
                continue;
            }

            let span = tracing::info_span!("processor", name = proc.name(), phase = "response_body");
            match proc.process_response_body(body).instrument(span).await {
                Ok(PhaseResult::Continue) => {}
                Ok(PhaseResult::Mutate(mutation)) => {
                    mutations.push(mutation);
                }
                Ok(PhaseResult::Respond(response)) => {
                    return SequenceOutcome::Respond(response);
                }
                Err(err) => match self.failure_mode {
                    FailureMode::FailClosed => return SequenceOutcome::Error(err),
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

        SequenceOutcome::Continue(mutations)
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
    use crate::phase::HeaderMutations;
    use crate::processor::BoxFuture;

    // -- Test helpers --

    fn empty_request_parts() -> http::request::Parts {
        http::Request::builder().body(()).unwrap().into_parts().0
    }

    fn empty_response_parts() -> http::response::Parts {
        http::Response::builder().body(()).unwrap().into_parts().0
    }

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
            _parts: &http::request::Parts,
        ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
            Box::pin(async {
                Ok(PhaseResult::Respond(
                    ImmediateResponse::with_status(http::StatusCode::FORBIDDEN)
                        .body("denied"),
                ))
            })
        }
    }

    /// Adds a header mutation.
    struct MutatingProcessor {
        header_name: &'static str,
        header_value: &'static str,
    }
    impl Processor for MutatingProcessor {
        fn name(&self) -> &str {
            "mutating"
        }
        fn process_request_headers(
            &self,
            _parts: &http::request::Parts,
        ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
            let name = http::HeaderName::from_static(self.header_name);
            let value = http::HeaderValue::from_static(self.header_value);
            Box::pin(async move {
                Ok(PhaseResult::Mutate(
                    HeaderMutations::new().set_header(name, value),
                ))
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
            _parts: &http::request::Parts,
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
            _parts: &http::request::Parts,
        ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
            self.request_headers_called
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Box::pin(async { Ok(PhaseResult::Continue) })
        }
        fn process_response_headers(
            &self,
            _parts: &http::response::Parts,
        ) -> BoxFuture<'_, Result<PhaseResult, ProcessorError>> {
            self.response_headers_called
                .store(true, std::sync::atomic::Ordering::SeqCst);
            Box::pin(async { Ok(PhaseResult::Continue) })
        }
    }

    // -- Tests --

    #[tokio::test]
    async fn empty_pipeline_continues() {
        let seq = Sequence::builder().build();
        let parts = empty_request_parts();
        let outcome = seq.process_request_headers(&parts).await;
        assert!(matches!(outcome, SequenceOutcome::Continue(m) if m.is_empty()));
    }

    #[tokio::test]
    async fn passthrough_continues() {
        let seq = Sequence::builder()
            .processor(Arc::new(PassthroughProcessor))
            .build();
        let parts = empty_request_parts();
        let outcome = seq.process_request_headers(&parts).await;
        assert!(matches!(outcome, SequenceOutcome::Continue(m) if m.is_empty()));
    }

    #[tokio::test]
    async fn deny_short_circuits() {
        let seq = Sequence::builder()
            .processor(Arc::new(DenyProcessor))
            .processor(Arc::new(PassthroughProcessor))
            .build();
        let parts = empty_request_parts();
        let outcome = seq.process_request_headers(&parts).await;
        assert!(matches!(outcome, SequenceOutcome::Respond(_)));
    }

    #[tokio::test]
    async fn mutations_accumulate() {
        let seq = Sequence::builder()
            .processor(Arc::new(MutatingProcessor {
                header_name: "x-first",
                header_value: "1",
            }))
            .processor(Arc::new(MutatingProcessor {
                header_name: "x-second",
                header_value: "2",
            }))
            .build();
        let parts = empty_request_parts();
        let outcome = seq.process_request_headers(&parts).await;
        match outcome {
            SequenceOutcome::Continue(mutations) => {
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
                header_name: "x-should-not-appear",
                header_value: "nope",
            }))
            .build();
        let parts = empty_request_parts();
        let outcome = seq.process_request_headers(&parts).await;
        assert!(matches!(outcome, SequenceOutcome::Respond(_)));
    }

    #[tokio::test]
    async fn error_fail_closed() {
        let seq = Sequence::builder()
            .failure_mode(FailureMode::FailClosed)
            .processor(Arc::new(FailingProcessor))
            .processor(Arc::new(PassthroughProcessor))
            .build();
        let parts = empty_request_parts();
        let outcome = seq.process_request_headers(&parts).await;
        assert!(matches!(outcome, SequenceOutcome::Error(_)));
    }

    #[tokio::test]
    async fn error_fail_open() {
        let seq = Sequence::builder()
            .failure_mode(FailureMode::FailOpen)
            .processor(Arc::new(FailingProcessor))
            .processor(Arc::new(PassthroughProcessor))
            .build();
        let parts = empty_request_parts();
        let outcome = seq.process_request_headers(&parts).await;
        assert!(matches!(outcome, SequenceOutcome::Continue(_)));
    }

    #[tokio::test]
    async fn aggregate_mode_computed_from_processors() {
        let seq = Sequence::builder()
            .processor(Arc::new(PassthroughProcessor))
            .processor(Arc::new(BodyProcessor))
            .build();
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
        assert!(matches!(outcome, SequenceOutcome::Continue(m) if m.is_empty()));
    }

    #[tokio::test]
    async fn cross_phase_termination_on_immediate_response() {
        let recorder = Arc::new(RecordingProcessor::new());
        let seq = Sequence::builder()
            .processor(Arc::new(DenyProcessor))
            .processor(recorder.clone())
            .build();

        let parts = empty_request_parts();
        let outcome = seq.process_request_headers(&parts).await;

        assert!(matches!(outcome, SequenceOutcome::Respond(_)));
        assert!(!recorder
            .request_headers_called
            .load(std::sync::atomic::Ordering::SeqCst));
        assert!(outcome.is_terminal());
    }

    #[tokio::test]
    async fn response_headers_phase() {
        let seq = Sequence::builder()
            .processor(Arc::new(PassthroughProcessor))
            .build();
        let parts = empty_response_parts();
        let outcome = seq.process_response_headers(&parts).await;
        assert!(matches!(outcome, SequenceOutcome::Continue(m) if m.is_empty()));
    }
}
