//! Configuration module: YAML-based config for agents, routing rules, autopilot settings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("config file not found: {0}")]
    NotFound(PathBuf),
    #[error("failed to read config: {0}")]
    IoError(#[from] std::io::Error),
    #[error("invalid YAML: {0}")]
    YamlError(#[from] serde_yaml::Error),
    #[error("invalid JSON: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// Model tier for routing decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelTier {
    Haiku,
    Sonnet,
    Opus,
}

impl std::fmt::Display for ModelTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelTier::Haiku => write!(f, "haiku"),
            ModelTier::Sonnet => write!(f, "sonnet"),
            ModelTier::Opus => write!(f, "opus"),
        }
    }
}

impl Default for ModelTier {
    fn default() -> Self {
        ModelTier::Sonnet
    }
}

/// Agent configuration entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub model_tier: ModelTier,
    #[serde(default)]
    pub tool_permissions: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Routing rule configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    pub name: String,
    #[serde(default = "default_threshold")]
    pub threshold: f64,
    #[serde(default)]
    pub target_tier: ModelTier,
    #[serde(default)]
    pub patterns: Vec<String>,
}

fn default_threshold() -> f64 {
    0.5
}

/// Autopilot configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutopilotConfig {
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    #[serde(default = "default_true")]
    pub auto_verify: bool,
    #[serde(default = "default_true")]
    pub auto_qa: bool,
}

fn default_max_retries() -> u32 {
    3
}
fn default_max_iterations() -> u32 {
    10
}
fn default_true() -> bool {
    true
}

impl Default for AutopilotConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            max_iterations: 10,
            auto_verify: true,
            auto_qa: true,
        }
    }
}

/// Top-level configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmcConfig {
    #[serde(default)]
    pub agents: Vec<AgentConfig>,
    #[serde(default)]
    pub routing_rules: Vec<RoutingRule>,
    #[serde(default)]
    pub autopilot: AutopilotConfig,
    #[serde(default)]
    pub keywords: HashMap<String, KeywordConfig>,
    #[serde(default)]
    pub state_dir: Option<String>,
}

/// Keyword configuration entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordConfig {
    pub triggers: Vec<String>,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub description: String,
}

impl Default for OmcConfig {
    fn default() -> Self {
        Self {
            agents: Vec::new(),
            routing_rules: Vec::new(),
            autopilot: AutopilotConfig::default(),
            keywords: HashMap::new(),
            state_dir: None,
        }
    }
}

impl OmcConfig {
    /// Load config from a YAML file path.
    pub fn from_yaml_file(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Err(ConfigError::NotFound(path.to_path_buf()));
        }
        let content = std::fs::read_to_string(path)?;
        let config: OmcConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    /// Load config from a YAML string.
    pub fn from_yaml_str(yaml: &str) -> Result<Self, ConfigError> {
        let config: OmcConfig = serde_yaml::from_str(yaml)?;
        Ok(config)
    }

    /// Load config from a JSON string.
    pub fn from_json_str(json: &str) -> Result<Self, ConfigError> {
        let config: OmcConfig = serde_json::from_str(json)?;
        Ok(config)
    }

    /// Resolve the state directory path.
    pub fn state_directory(&self) -> PathBuf {
        match &self.state_dir {
            Some(dir) => PathBuf::from(dir),
            None => PathBuf::from(".omc/state"),
        }
    }

    /// Get agent config by name.
    pub fn get_agent(&self, name: &str) -> Option<&AgentConfig> {
        self.agents.iter().find(|a| a.name == name)
    }

    /// Get routing rules for a specific tier.
    pub fn rules_for_tier(&self, tier: ModelTier) -> Vec<&RoutingRule> {
        self.routing_rules
            .iter()
            .filter(|r| r.target_tier == tier)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = OmcConfig::default();
        assert!(config.agents.is_empty());
        assert!(config.routing_rules.is_empty());
        assert_eq!(config.autopilot.max_retries, 3);
        assert_eq!(config.autopilot.max_iterations, 10);
        assert!(config.autopilot.auto_verify);
    }

    #[test]
    fn test_model_tier_display() {
        assert_eq!(ModelTier::Haiku.to_string(), "haiku");
        assert_eq!(ModelTier::Sonnet.to_string(), "sonnet");
        assert_eq!(ModelTier::Opus.to_string(), "opus");
    }

    #[test]
    fn test_model_tier_default() {
        assert_eq!(ModelTier::default(), ModelTier::Sonnet);
    }

    #[test]
    fn test_config_from_yaml() {
        let yaml = r#"
agents:
  - name: architect
    description: "System design specialist"
    model_tier: opus
    tool_permissions:
      - read
      - write
    tags:
      - design
      - architecture
routing_rules:
  - name: complex_tasks
    threshold: 0.7
    target_tier: opus
    patterns:
      - "architect"
      - "design"
autopilot:
  max_retries: 5
  max_iterations: 20
  auto_verify: true
  auto_qa: false
"#;
        let config = OmcConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.agents.len(), 1);
        assert_eq!(config.agents[0].name, "architect");
        assert_eq!(config.agents[0].model_tier, ModelTier::Opus);
        assert_eq!(config.agents[0].tool_permissions.len(), 2);
        assert_eq!(config.routing_rules.len(), 1);
        assert_eq!(config.autopilot.max_retries, 5);
        assert!(!config.autopilot.auto_qa);
    }

    #[test]
    fn test_config_from_json() {
        let json = r#"{
            "agents": [{"name": "debugger", "description": "Bug hunter", "model_tier": "sonnet"}],
            "autopilot": {"max_retries": 2}
        }"#;
        let config = OmcConfig::from_json_str(json).unwrap();
        assert_eq!(config.agents[0].name, "debugger");
        assert_eq!(config.autopilot.max_retries, 2);
    }

    #[test]
    fn test_state_directory_default() {
        let config = OmcConfig::default();
        assert_eq!(config.state_directory(), PathBuf::from(".omc/state"));
    }

    #[test]
    fn test_state_directory_custom() {
        let config = OmcConfig {
            state_dir: Some("/tmp/omc".to_string()),
            ..Default::default()
        };
        assert_eq!(config.state_directory(), PathBuf::from("/tmp/omc"));
    }

    #[test]
    fn test_get_agent() {
        let config = OmcConfig {
            agents: vec![
                AgentConfig {
                    name: "analyst".to_string(),
                    description: "Analysis".to_string(),
                    system_prompt: String::new(),
                    model_tier: ModelTier::Sonnet,
                    tool_permissions: vec![],
                    tags: vec![],
                },
                AgentConfig {
                    name: "executor".to_string(),
                    description: "Execution".to_string(),
                    system_prompt: String::new(),
                    model_tier: ModelTier::Opus,
                    tool_permissions: vec![],
                    tags: vec![],
                },
            ],
            ..Default::default()
        };
        assert!(config.get_agent("analyst").is_some());
        assert!(config.get_agent("executor").is_some());
        assert!(config.get_agent("missing").is_none());
    }

    #[test]
    fn test_rules_for_tier() {
        let config = OmcConfig {
            routing_rules: vec![
                RoutingRule {
                    name: "r1".to_string(),
                    threshold: 0.3,
                    target_tier: ModelTier::Haiku,
                    patterns: vec![],
                },
                RoutingRule {
                    name: "r2".to_string(),
                    threshold: 0.7,
                    target_tier: ModelTier::Opus,
                    patterns: vec![],
                },
                RoutingRule {
                    name: "r3".to_string(),
                    threshold: 0.5,
                    target_tier: ModelTier::Haiku,
                    patterns: vec![],
                },
            ],
            ..Default::default()
        };
        let haiku_rules = config.rules_for_tier(ModelTier::Haiku);
        assert_eq!(haiku_rules.len(), 2);
        let opus_rules = config.rules_for_tier(ModelTier::Opus);
        assert_eq!(opus_rules.len(), 1);
    }

    #[test]
    fn test_config_file_not_found() {
        let result = OmcConfig::from_yaml_file(Path::new("/nonexistent/config.yaml"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::NotFound(_)));
    }

    #[test]
    fn test_invalid_yaml() {
        let result = OmcConfig::from_yaml_str("{{{{invalid yaml");
        assert!(result.is_err());
    }

    #[test]
    fn test_keyword_config() {
        let yaml = r#"
keywords:
  autopilot:
    triggers: ["autopilot", "auto"]
    mode: "autopilot"
    description: "Enable autopilot mode"
  ralph:
    triggers: ["ralph", "persistent"]
    mode: "ralph"
"#;
        let config = OmcConfig::from_yaml_str(yaml).unwrap();
        assert_eq!(config.keywords.len(), 2);
        assert_eq!(config.keywords["autopilot"].triggers.len(), 2);
    }

    #[test]
    fn test_agent_config_defaults() {
        let yaml = r#"
agents:
  - name: simple
    description: "Minimal agent"
"#;
        let config = OmcConfig::from_yaml_str(yaml).unwrap();
        let agent = &config.agents[0];
        assert_eq!(agent.model_tier, ModelTier::Sonnet);
        assert!(agent.tool_permissions.is_empty());
        assert!(agent.system_prompt.is_empty());
    }
}
