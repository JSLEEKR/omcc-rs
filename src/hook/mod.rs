//! Hook bridge: read JSON from stdin, route to handlers, write JSON to stdout.
//! Supports 14+ hook types.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HookError {
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("unknown hook type: {0}")]
    UnknownHook(String),
    #[error("handler error: {0}")]
    HandlerError(String),
}

/// All supported hook types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookType {
    SessionStart,
    SessionEnd,
    PreToolUse,
    PostToolUse,
    KeywordDetector,
    Notification,
    PreCompact,
    PostCompact,
    ModelSelection,
    TaskDecomposition,
    AgentDelegation,
    PermissionCheck,
    ContextInjection,
    Recovery,
}

impl HookType {
    /// Parse a hook type from a string.
    pub fn from_str_name(s: &str) -> Option<Self> {
        match s {
            "session_start" | "SessionStart" => Some(HookType::SessionStart),
            "session_end" | "SessionEnd" => Some(HookType::SessionEnd),
            "pre_tool_use" | "PreToolUse" => Some(HookType::PreToolUse),
            "post_tool_use" | "PostToolUse" => Some(HookType::PostToolUse),
            "keyword_detector" | "KeywordDetector" => Some(HookType::KeywordDetector),
            "notification" | "Notification" => Some(HookType::Notification),
            "pre_compact" | "PreCompact" => Some(HookType::PreCompact),
            "post_compact" | "PostCompact" => Some(HookType::PostCompact),
            "model_selection" | "ModelSelection" => Some(HookType::ModelSelection),
            "task_decomposition" | "TaskDecomposition" => Some(HookType::TaskDecomposition),
            "agent_delegation" | "AgentDelegation" => Some(HookType::AgentDelegation),
            "permission_check" | "PermissionCheck" => Some(HookType::PermissionCheck),
            "context_injection" | "ContextInjection" => Some(HookType::ContextInjection),
            "recovery" | "Recovery" => Some(HookType::Recovery),
            _ => None,
        }
    }

    /// Get all hook type names.
    pub fn all_names() -> Vec<&'static str> {
        vec![
            "session_start", "session_end", "pre_tool_use", "post_tool_use",
            "keyword_detector", "notification", "pre_compact", "post_compact",
            "model_selection", "task_decomposition", "agent_delegation",
            "permission_check", "context_injection", "recovery",
        ]
    }
}

/// Input message from Claude Code hook system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookInput {
    pub hook_type: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
}

/// Output response to Claude Code hook system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookOutput {
    #[serde(default)]
    pub result: Value,
    #[serde(default = "default_true")]
    pub continue_execution: bool,
    #[serde(default)]
    pub inject_message: Option<String>,
    #[serde(default)]
    pub override_model: Option<String>,
    #[serde(default)]
    pub errors: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl HookOutput {
    /// Create a pass-through output (no modifications).
    pub fn passthrough() -> Self {
        Self {
            result: Value::Null,
            continue_execution: true,
            inject_message: None,
            override_model: None,
            errors: vec![],
        }
    }

    /// Create a blocking output (stop execution).
    pub fn block(reason: &str) -> Self {
        Self {
            result: Value::Null,
            continue_execution: false,
            inject_message: Some(reason.to_string()),
            override_model: None,
            errors: vec![],
        }
    }

    /// Create an output with a model override.
    pub fn with_model(model: &str) -> Self {
        Self {
            result: Value::Null,
            continue_execution: true,
            inject_message: None,
            override_model: Some(model.to_string()),
            errors: vec![],
        }
    }

    /// Create an output with an injected message.
    pub fn with_injection(message: &str) -> Self {
        Self {
            result: Value::Null,
            continue_execution: true,
            inject_message: Some(message.to_string()),
            override_model: None,
            errors: vec![],
        }
    }

    /// Create an error output.
    pub fn error(message: &str) -> Self {
        Self {
            result: Value::Null,
            continue_execution: true,
            inject_message: None,
            override_model: None,
            errors: vec![message.to_string()],
        }
    }
}

/// A hook handler function type.
pub type HandlerFn = Box<dyn Fn(&HookInput) -> Result<HookOutput, HookError> + Send + Sync>;

/// The hook bridge router.
pub struct HookBridge {
    handlers: HashMap<String, HandlerFn>,
}

impl HookBridge {
    /// Create a new hook bridge.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler for a hook type.
    pub fn register<F>(&mut self, hook_type: &str, handler: F)
    where
        F: Fn(&HookInput) -> Result<HookOutput, HookError> + Send + Sync + 'static,
    {
        self.handlers.insert(hook_type.to_string(), Box::new(handler));
    }

    /// Route a hook input to the appropriate handler.
    pub fn route(&self, input: &HookInput) -> Result<HookOutput, HookError> {
        match self.handlers.get(&input.hook_type) {
            Some(handler) => handler(input),
            None => {
                // Unknown hooks pass through
                Ok(HookOutput::passthrough())
            }
        }
    }

    /// Process a single JSON line from stdin and return the output.
    pub fn process_json(&self, json_str: &str) -> Result<HookOutput, HookError> {
        let input: HookInput = serde_json::from_str(json_str)?;
        self.route(&input)
    }

    /// Maximum allowed input size (1 MB) to prevent memory exhaustion.
    const MAX_INPUT_SIZE: usize = 1024 * 1024;

    /// Run the bridge in stdin/stdout mode (one-shot).
    pub fn run_oneshot(&self) -> Result<(), HookError> {
        let stdin = io::stdin();
        let mut input_str = String::new();
        let mut reader = stdin.lock();
        let bytes_read = reader.read_line(&mut input_str)?;

        if bytes_read > Self::MAX_INPUT_SIZE {
            return Err(HookError::HandlerError(
                format!("Input too large: {} bytes (max {})", bytes_read, Self::MAX_INPUT_SIZE),
            ));
        }

        if input_str.trim().is_empty() {
            return Ok(());
        }

        let output = self.process_json(input_str.trim())?;
        let output_json = serde_json::to_string(&output)?;

        let stdout = io::stdout();
        let mut handle = stdout.lock();
        writeln!(handle, "{}", output_json)?;
        handle.flush()?;

        Ok(())
    }

    /// Check if a handler is registered for a hook type.
    pub fn has_handler(&self, hook_type: &str) -> bool {
        self.handlers.contains_key(hook_type)
    }

    /// List all registered hook types.
    pub fn registered_hooks(&self) -> Vec<String> {
        let mut hooks: Vec<String> = self.handlers.keys().cloned().collect();
        hooks.sort();
        hooks
    }
}

impl Default for HookBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_type_from_str() {
        assert_eq!(HookType::from_str_name("session_start"), Some(HookType::SessionStart));
        assert_eq!(HookType::from_str_name("pre_tool_use"), Some(HookType::PreToolUse));
        assert_eq!(HookType::from_str_name("unknown"), None);
    }

    #[test]
    fn test_all_hook_names() {
        let names = HookType::all_names();
        assert_eq!(names.len(), 14);
        assert!(names.contains(&"session_start"));
        assert!(names.contains(&"recovery"));
    }

    #[test]
    fn test_hook_input_parse() {
        let json = r#"{"hook_type": "session_start", "payload": {"user": "test"}}"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.hook_type, "session_start");
        assert_eq!(input.payload["user"], "test");
    }

    #[test]
    fn test_hook_input_minimal() {
        let json = r#"{"hook_type": "notification"}"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.hook_type, "notification");
        assert!(input.session_id.is_none());
    }

    #[test]
    fn test_passthrough_output() {
        let output = HookOutput::passthrough();
        assert!(output.continue_execution);
        assert!(output.inject_message.is_none());
        assert!(output.errors.is_empty());
    }

    #[test]
    fn test_block_output() {
        let output = HookOutput::block("dangerous operation");
        assert!(!output.continue_execution);
        assert_eq!(output.inject_message, Some("dangerous operation".to_string()));
    }

    #[test]
    fn test_model_override_output() {
        let output = HookOutput::with_model("claude-3-opus");
        assert!(output.continue_execution);
        assert_eq!(output.override_model, Some("claude-3-opus".to_string()));
    }

    #[test]
    fn test_injection_output() {
        let output = HookOutput::with_injection("Remember to test!");
        assert_eq!(output.inject_message, Some("Remember to test!".to_string()));
    }

    #[test]
    fn test_error_output() {
        let output = HookOutput::error("something went wrong");
        assert_eq!(output.errors.len(), 1);
        assert!(output.continue_execution);
    }

    #[test]
    fn test_bridge_register_and_route() {
        let mut bridge = HookBridge::new();
        bridge.register("test_hook", |_input| {
            Ok(HookOutput::with_injection("handled!"))
        });

        let input = HookInput {
            hook_type: "test_hook".to_string(),
            session_id: None,
            payload: Value::Null,
            metadata: HashMap::new(),
        };

        let output = bridge.route(&input).unwrap();
        assert_eq!(output.inject_message, Some("handled!".to_string()));
    }

    #[test]
    fn test_bridge_unknown_hook_passthrough() {
        let bridge = HookBridge::new();
        let input = HookInput {
            hook_type: "unknown_hook".to_string(),
            session_id: None,
            payload: Value::Null,
            metadata: HashMap::new(),
        };
        let output = bridge.route(&input).unwrap();
        assert!(output.continue_execution);
    }

    #[test]
    fn test_bridge_process_json() {
        let mut bridge = HookBridge::new();
        bridge.register("ping", |_| Ok(HookOutput::with_injection("pong")));

        let json = r#"{"hook_type": "ping"}"#;
        let output = bridge.process_json(json).unwrap();
        assert_eq!(output.inject_message, Some("pong".to_string()));
    }

    #[test]
    fn test_bridge_process_invalid_json() {
        let bridge = HookBridge::new();
        let result = bridge.process_json("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_has_handler() {
        let mut bridge = HookBridge::new();
        bridge.register("my_hook", |_| Ok(HookOutput::passthrough()));
        assert!(bridge.has_handler("my_hook"));
        assert!(!bridge.has_handler("other_hook"));
    }

    #[test]
    fn test_registered_hooks() {
        let mut bridge = HookBridge::new();
        bridge.register("b_hook", |_| Ok(HookOutput::passthrough()));
        bridge.register("a_hook", |_| Ok(HookOutput::passthrough()));
        let hooks = bridge.registered_hooks();
        assert_eq!(hooks, vec!["a_hook", "b_hook"]);
    }

    #[test]
    fn test_handler_with_payload() {
        let mut bridge = HookBridge::new();
        bridge.register("echo", |input| {
            Ok(HookOutput {
                result: input.payload.clone(),
                continue_execution: true,
                inject_message: None,
                override_model: None,
                errors: vec![],
            })
        });

        let input = HookInput {
            hook_type: "echo".to_string(),
            session_id: None,
            payload: serde_json::json!({"key": "value"}),
            metadata: HashMap::new(),
        };
        let output = bridge.route(&input).unwrap();
        assert_eq!(output.result["key"], "value");
    }

    #[test]
    fn test_handler_error() {
        let mut bridge = HookBridge::new();
        bridge.register("fail", |_| {
            Err(HookError::HandlerError("intentional failure".to_string()))
        });

        let input = HookInput {
            hook_type: "fail".to_string(),
            session_id: None,
            payload: Value::Null,
            metadata: HashMap::new(),
        };
        let result = bridge.route(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_hook_output_serialization() {
        let output = HookOutput::with_model("opus");
        let json = serde_json::to_string(&output).unwrap();
        let restored: HookOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.override_model, Some("opus".to_string()));
    }

    #[test]
    fn test_hook_input_with_metadata() {
        let json = r#"{"hook_type": "pre_tool_use", "metadata": {"tool": "bash", "args": "ls"}}"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.metadata["tool"], "bash");
    }

    #[test]
    fn test_hook_input_with_session() {
        let json = r#"{"hook_type": "session_start", "session_id": "abc-123"}"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.session_id, Some("abc-123".to_string()));
    }
}
