use crate::capture::CapturePipelineState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureOrchestratorState {
    state: CapturePipelineState,
}

impl Default for CaptureOrchestratorState {
    fn default() -> Self {
        Self {
            state: CapturePipelineState::Idle,
        }
    }
}

impl CaptureOrchestratorState {
    pub fn current(&self) -> CapturePipelineState {
        self.state
    }

    pub fn transition(&mut self, next: CapturePipelineState) {
        self.state = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_start_in_idle_state() {
        let orchestrator = CaptureOrchestratorState::default();
        assert_eq!(orchestrator.current(), CapturePipelineState::Idle);
    }

    #[test]
    fn should_transition_between_states() {
        let mut orchestrator = CaptureOrchestratorState::default();
        orchestrator.transition(CapturePipelineState::Capturing);
        assert_eq!(orchestrator.current(), CapturePipelineState::Capturing);
    }
}
