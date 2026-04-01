//! Autopilot pipeline state machine: Plan -> Execute -> Verify -> QA stages
//! with state transitions, retry on failure, configurable max iterations.

use serde::{Deserialize, Serialize};

/// Autopilot pipeline stage.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Idle,
    Planning,
    Executing,
    Verifying,
    Qa,
    Complete,
    Failed,
}

impl std::fmt::Display for Stage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Stage::Idle => write!(f, "IDLE"),
            Stage::Planning => write!(f, "PLANNING"),
            Stage::Executing => write!(f, "EXECUTING"),
            Stage::Verifying => write!(f, "VERIFYING"),
            Stage::Qa => write!(f, "QA"),
            Stage::Complete => write!(f, "COMPLETE"),
            Stage::Failed => write!(f, "FAILED"),
        }
    }
}

/// Transition result from a stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionResult {
    /// Successfully moved to next stage.
    Advanced(Stage),
    /// Stage failed, retrying.
    Retrying(Stage, u32),
    /// Pipeline completed successfully.
    Completed,
    /// Pipeline failed after max retries.
    FailedMaxRetries,
    /// Invalid transition attempted.
    InvalidTransition(String),
}

/// Stage completion signal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageResult {
    pub stage: Stage,
    pub success: bool,
    pub message: String,
    #[serde(default)]
    pub artifacts: Vec<String>,
}

/// Autopilot pipeline state machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutopilotPipeline {
    pub current_stage: Stage,
    pub iteration: u32,
    pub max_iterations: u32,
    pub max_retries: u32,
    pub retry_count: u32,
    pub history: Vec<StageResult>,
    pub task_description: String,
    #[serde(default)]
    pub plan: Option<String>,
    #[serde(default)]
    pub auto_verify: bool,
    #[serde(default)]
    pub auto_qa: bool,
}

impl AutopilotPipeline {
    /// Create a new pipeline for a task.
    pub fn new(task: &str, max_iterations: u32, max_retries: u32) -> Self {
        Self {
            current_stage: Stage::Idle,
            iteration: 0,
            max_iterations,
            max_retries,
            retry_count: 0,
            history: Vec::new(),
            task_description: task.to_string(),
            plan: None,
            auto_verify: true,
            auto_qa: true,
        }
    }

    /// Start the pipeline (transition from Idle to Planning).
    pub fn start(&mut self) -> TransitionResult {
        if self.current_stage != Stage::Idle {
            return TransitionResult::InvalidTransition(
                format!("Cannot start from stage {}", self.current_stage),
            );
        }
        self.current_stage = Stage::Planning;
        self.iteration = 1;
        TransitionResult::Advanced(Stage::Planning)
    }

    /// Complete the current stage and advance.
    pub fn complete_stage(&mut self, result: StageResult) -> TransitionResult {
        self.history.push(result.clone());

        if !result.success {
            return self.handle_failure();
        }

        match self.current_stage {
            Stage::Planning => {
                self.plan = Some(result.message.clone());
                self.current_stage = Stage::Executing;
                TransitionResult::Advanced(Stage::Executing)
            }
            Stage::Executing => {
                if self.auto_verify {
                    self.current_stage = Stage::Verifying;
                    TransitionResult::Advanced(Stage::Verifying)
                } else if self.auto_qa {
                    self.current_stage = Stage::Qa;
                    TransitionResult::Advanced(Stage::Qa)
                } else {
                    self.current_stage = Stage::Complete;
                    TransitionResult::Completed
                }
            }
            Stage::Verifying => {
                if self.auto_qa {
                    self.current_stage = Stage::Qa;
                    TransitionResult::Advanced(Stage::Qa)
                } else {
                    self.current_stage = Stage::Complete;
                    TransitionResult::Completed
                }
            }
            Stage::Qa => {
                self.current_stage = Stage::Complete;
                TransitionResult::Completed
            }
            _ => TransitionResult::InvalidTransition(
                format!("Cannot complete stage {}", self.current_stage),
            ),
        }
    }

    /// Handle a stage failure.
    fn handle_failure(&mut self) -> TransitionResult {
        self.retry_count += 1;

        if self.retry_count > self.max_retries {
            self.current_stage = Stage::Failed;
            return TransitionResult::FailedMaxRetries;
        }

        // Retry from Executing stage (verify/QA failures retry execution)
        // Terminal states should never reach here, but guard against it
        let retry_stage = match self.current_stage {
            Stage::Verifying | Stage::Qa => Stage::Executing,
            Stage::Idle | Stage::Complete | Stage::Failed => {
                self.current_stage = Stage::Failed;
                return TransitionResult::InvalidTransition(
                    format!("Cannot retry from terminal stage {}", self.current_stage),
                );
            }
            _ => self.current_stage.clone(),
        };

        self.current_stage = retry_stage.clone();
        self.iteration += 1;

        if self.iteration > self.max_iterations {
            self.current_stage = Stage::Failed;
            return TransitionResult::FailedMaxRetries;
        }

        TransitionResult::Retrying(retry_stage, self.retry_count)
    }

    /// Check if the pipeline is active (not idle, complete, or failed).
    pub fn is_active(&self) -> bool {
        !matches!(self.current_stage, Stage::Idle | Stage::Complete | Stage::Failed)
    }

    /// Check if the pipeline is complete.
    pub fn is_complete(&self) -> bool {
        self.current_stage == Stage::Complete
    }

    /// Check if the pipeline failed.
    pub fn is_failed(&self) -> bool {
        self.current_stage == Stage::Failed
    }

    /// Get progress as a percentage (0-100).
    pub fn progress_percent(&self) -> u32 {
        match self.current_stage {
            Stage::Idle => 0,
            Stage::Planning => 10,
            Stage::Executing => 40,
            Stage::Verifying => 70,
            Stage::Qa => 90,
            Stage::Complete => 100,
            Stage::Failed => 0,
        }
    }

    /// Get a summary of the pipeline state.
    pub fn summary(&self) -> String {
        format!(
            "[{}] iter={}/{} retries={}/{} progress={}%",
            self.current_stage,
            self.iteration,
            self.max_iterations,
            self.retry_count,
            self.max_retries,
            self.progress_percent(),
        )
    }

    /// Force-fail the pipeline.
    pub fn abort(&mut self) {
        self.current_stage = Stage::Failed;
        self.history.push(StageResult {
            stage: Stage::Failed,
            success: false,
            message: "Pipeline aborted".to_string(),
            artifacts: vec![],
        });
    }

    /// Reset the pipeline to idle.
    pub fn reset(&mut self) {
        self.current_stage = Stage::Idle;
        self.iteration = 0;
        self.retry_count = 0;
        self.history.clear();
        self.plan = None;
    }

    /// Get history for a specific stage.
    pub fn stage_history(&self, stage: &Stage) -> Vec<&StageResult> {
        self.history.iter().filter(|r| &r.stage == stage).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(stage: Stage, success: bool) -> StageResult {
        StageResult {
            stage,
            success,
            message: "test".to_string(),
            artifacts: vec![],
        }
    }

    #[test]
    fn test_new_pipeline() {
        let p = AutopilotPipeline::new("test task", 10, 3);
        assert_eq!(p.current_stage, Stage::Idle);
        assert_eq!(p.iteration, 0);
        assert!(!p.is_active());
    }

    #[test]
    fn test_start() {
        let mut p = AutopilotPipeline::new("task", 10, 3);
        let result = p.start();
        assert_eq!(result, TransitionResult::Advanced(Stage::Planning));
        assert!(p.is_active());
    }

    #[test]
    fn test_double_start() {
        let mut p = AutopilotPipeline::new("task", 10, 3);
        p.start();
        let result = p.start();
        assert!(matches!(result, TransitionResult::InvalidTransition(_)));
    }

    #[test]
    fn test_full_success_pipeline() {
        let mut p = AutopilotPipeline::new("task", 10, 3);
        p.start();

        // Planning success
        let r = p.complete_stage(make_result(Stage::Planning, true));
        assert_eq!(r, TransitionResult::Advanced(Stage::Executing));

        // Executing success
        let r = p.complete_stage(make_result(Stage::Executing, true));
        assert_eq!(r, TransitionResult::Advanced(Stage::Verifying));

        // Verifying success
        let r = p.complete_stage(make_result(Stage::Verifying, true));
        assert_eq!(r, TransitionResult::Advanced(Stage::Qa));

        // QA success
        let r = p.complete_stage(make_result(Stage::Qa, true));
        assert_eq!(r, TransitionResult::Completed);
        assert!(p.is_complete());
        assert_eq!(p.progress_percent(), 100);
    }

    #[test]
    fn test_failure_and_retry() {
        let mut p = AutopilotPipeline::new("task", 10, 3);
        p.start();
        p.complete_stage(make_result(Stage::Planning, true));

        // Executing fails
        let r = p.complete_stage(make_result(Stage::Executing, false));
        assert!(matches!(r, TransitionResult::Retrying(Stage::Executing, 1)));
        assert_eq!(p.retry_count, 1);
    }

    #[test]
    fn test_max_retries_exceeded() {
        let mut p = AutopilotPipeline::new("task", 10, 2);
        p.start();
        p.complete_stage(make_result(Stage::Planning, true));

        // Fail 3 times (max_retries = 2)
        p.complete_stage(make_result(Stage::Executing, false));
        p.complete_stage(make_result(Stage::Executing, false));
        let r = p.complete_stage(make_result(Stage::Executing, false));
        assert_eq!(r, TransitionResult::FailedMaxRetries);
        assert!(p.is_failed());
    }

    #[test]
    fn test_verification_failure_retries_executing() {
        let mut p = AutopilotPipeline::new("task", 10, 3);
        p.start();
        p.complete_stage(make_result(Stage::Planning, true));
        p.complete_stage(make_result(Stage::Executing, true));

        // Verification fails -> retry from executing
        let r = p.complete_stage(make_result(Stage::Verifying, false));
        assert!(matches!(r, TransitionResult::Retrying(Stage::Executing, _)));
    }

    #[test]
    fn test_no_verify_pipeline() {
        let mut p = AutopilotPipeline::new("task", 10, 3);
        p.auto_verify = false;
        p.auto_qa = false;
        p.start();
        p.complete_stage(make_result(Stage::Planning, true));
        let r = p.complete_stage(make_result(Stage::Executing, true));
        assert_eq!(r, TransitionResult::Completed);
    }

    #[test]
    fn test_no_verify_but_qa() {
        let mut p = AutopilotPipeline::new("task", 10, 3);
        p.auto_verify = false;
        p.auto_qa = true;
        p.start();
        p.complete_stage(make_result(Stage::Planning, true));
        let r = p.complete_stage(make_result(Stage::Executing, true));
        assert_eq!(r, TransitionResult::Advanced(Stage::Qa));
    }

    #[test]
    fn test_abort() {
        let mut p = AutopilotPipeline::new("task", 10, 3);
        p.start();
        p.abort();
        assert!(p.is_failed());
        assert!(!p.is_active());
    }

    #[test]
    fn test_reset() {
        let mut p = AutopilotPipeline::new("task", 10, 3);
        p.start();
        p.complete_stage(make_result(Stage::Planning, true));
        p.reset();
        assert_eq!(p.current_stage, Stage::Idle);
        assert_eq!(p.iteration, 0);
        assert!(p.history.is_empty());
    }

    #[test]
    fn test_summary() {
        let mut p = AutopilotPipeline::new("task", 10, 3);
        p.start();
        let summary = p.summary();
        assert!(summary.contains("PLANNING"));
        assert!(summary.contains("iter=1/10"));
    }

    #[test]
    fn test_stage_display() {
        assert_eq!(Stage::Idle.to_string(), "IDLE");
        assert_eq!(Stage::Planning.to_string(), "PLANNING");
        assert_eq!(Stage::Complete.to_string(), "COMPLETE");
    }

    #[test]
    fn test_progress_percent() {
        let mut p = AutopilotPipeline::new("task", 10, 3);
        assert_eq!(p.progress_percent(), 0);
        p.start();
        assert_eq!(p.progress_percent(), 10);
    }

    #[test]
    fn test_stage_history() {
        let mut p = AutopilotPipeline::new("task", 10, 3);
        p.start();
        p.complete_stage(make_result(Stage::Planning, true));
        p.complete_stage(make_result(Stage::Executing, true));
        let plan_history = p.stage_history(&Stage::Planning);
        assert_eq!(plan_history.len(), 1);
    }

    #[test]
    fn test_plan_saved() {
        let mut p = AutopilotPipeline::new("task", 10, 3);
        p.start();
        p.complete_stage(StageResult {
            stage: Stage::Planning,
            success: true,
            message: "My plan details".to_string(),
            artifacts: vec![],
        });
        assert_eq!(p.plan, Some("My plan details".to_string()));
    }

    #[test]
    fn test_serialization() {
        let mut p = AutopilotPipeline::new("task", 10, 3);
        p.start();
        let json = serde_json::to_string(&p).unwrap();
        let restored: AutopilotPipeline = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.current_stage, Stage::Planning);
        assert_eq!(restored.task_description, "task");
    }
}
