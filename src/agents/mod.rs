//! Agent registry: agent definitions with name, description, system prompt,
//! model tier, tool permissions. Support loading from config.

use crate::config::ModelTier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An agent definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDef {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub model_tier: ModelTier,
    pub tool_permissions: Vec<String>,
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
}

impl AgentDef {
    /// Build a system prompt including context.
    pub fn build_prompt(&self, context: &str) -> String {
        if self.system_prompt.is_empty() {
            format!(
                "You are the {} agent. {}.\n\nContext:\n{}",
                self.name, self.description, context
            )
        } else {
            format!("{}\n\nContext:\n{}", self.system_prompt, context)
        }
    }

    /// Check if the agent has a specific capability.
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }

    /// Check if a tool is permitted.
    pub fn is_tool_permitted(&self, tool: &str) -> bool {
        self.tool_permissions.is_empty() || self.tool_permissions.iter().any(|t| t == tool)
    }
}

/// Agent registry holding all agent definitions.
pub struct AgentRegistry {
    agents: HashMap<String, AgentDef>,
}

impl AgentRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    /// Create a registry with the 19 built-in agents.
    pub fn with_builtins() -> Self {
        let mut registry = Self::new();
        for agent in builtin_agents() {
            registry.register(agent);
        }
        registry
    }

    /// Register an agent.
    pub fn register(&mut self, agent: AgentDef) {
        self.agents.insert(agent.name.clone(), agent);
    }

    /// Get an agent by name.
    pub fn get(&self, name: &str) -> Option<&AgentDef> {
        self.agents.get(name)
    }

    /// Get an agent by name, case-insensitive.
    pub fn get_normalized(&self, name: &str) -> Option<&AgentDef> {
        let lower = name.to_lowercase();
        self.agents.values().find(|a| a.name.to_lowercase() == lower)
    }

    /// List all agent names.
    pub fn list_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.agents.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// List agents by model tier.
    pub fn agents_for_tier(&self, tier: ModelTier) -> Vec<&AgentDef> {
        self.agents.values().filter(|a| a.model_tier == tier).collect()
    }

    /// Count total agents.
    pub fn count(&self) -> usize {
        self.agents.len()
    }

    /// Find agents matching a tag.
    pub fn find_by_tag(&self, tag: &str) -> Vec<&AgentDef> {
        self.agents
            .values()
            .filter(|a| a.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// Find agents matching a capability.
    pub fn find_by_capability(&self, capability: &str) -> Vec<&AgentDef> {
        self.agents
            .values()
            .filter(|a| a.has_capability(capability))
            .collect()
    }

    /// Remove an agent by name.
    pub fn unregister(&mut self, name: &str) -> Option<AgentDef> {
        self.agents.remove(name)
    }

    /// Resolve model for an agent, considering provider compatibility.
    pub fn resolve_model(&self, agent_name: &str, provider: &str) -> Option<ModelTier> {
        self.get(agent_name).map(|agent| {
            // Provider compatibility: some providers don't support Opus
            if provider == "bedrock" && agent.model_tier == ModelTier::Opus {
                ModelTier::Sonnet // Downgrade for Bedrock compatibility
            } else {
                agent.model_tier
            }
        })
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

/// Define the 19 built-in agents.
fn builtin_agents() -> Vec<AgentDef> {
    vec![
        agent("analyst", "Analyzes requirements and breaks down problems", ModelTier::Sonnet,
              &["analysis", "planning"], &["read"], &["analyze", "plan"]),
        agent("architect", "Designs system architecture and patterns", ModelTier::Opus,
              &["design", "architecture"], &["read", "write"], &["design", "architect"]),
        agent("executor", "Implements code changes efficiently", ModelTier::Sonnet,
              &["implementation", "coding"], &["read", "write", "bash"], &["implement", "code"]),
        agent("debugger", "Finds and fixes bugs systematically", ModelTier::Sonnet,
              &["debugging", "troubleshooting"], &["read", "write", "bash"], &["debug", "fix"]),
        agent("verifier", "Verifies correctness and quality", ModelTier::Sonnet,
              &["verification", "quality"], &["read", "bash"], &["verify", "test"]),
        agent("reviewer", "Reviews code for quality and security", ModelTier::Sonnet,
              &["review", "quality"], &["read"], &["review", "audit"]),
        agent("tester", "Writes and runs tests", ModelTier::Sonnet,
              &["testing"], &["read", "write", "bash"], &["test", "coverage"]),
        agent("documenter", "Writes documentation and comments", ModelTier::Haiku,
              &["documentation"], &["read", "write"], &["document", "explain"]),
        agent("refactorer", "Improves code structure without changing behavior", ModelTier::Sonnet,
              &["refactoring", "improvement"], &["read", "write"], &["refactor", "improve"]),
        agent("optimizer", "Optimizes performance and resource usage", ModelTier::Opus,
              &["optimization", "performance"], &["read", "write", "bash"], &["optimize", "benchmark"]),
        agent("security", "Audits code for security vulnerabilities", ModelTier::Opus,
              &["security", "audit"], &["read"], &["security-audit", "vulnerability-scan"]),
        agent("devops", "Handles deployment and infrastructure", ModelTier::Sonnet,
              &["devops", "deployment"], &["read", "write", "bash"], &["deploy", "configure"]),
        agent("planner", "Creates detailed execution plans", ModelTier::Opus,
              &["planning", "strategy"], &["read"], &["plan", "strategize"]),
        agent("researcher", "Investigates technologies and solutions", ModelTier::Sonnet,
              &["research", "investigation"], &["read", "bash"], &["research", "investigate"]),
        agent("migrator", "Handles data and schema migrations", ModelTier::Sonnet,
              &["migration", "data"], &["read", "write", "bash"], &["migrate", "transform"]),
        agent("mentor", "Explains concepts and provides guidance", ModelTier::Haiku,
              &["education", "guidance"], &["read"], &["explain", "guide"]),
        agent("qa", "Quality assurance and integration testing", ModelTier::Sonnet,
              &["qa", "integration"], &["read", "bash"], &["qa-test", "integration-test"]),
        agent("releaser", "Manages releases and versioning", ModelTier::Haiku,
              &["release", "versioning"], &["read", "write", "bash"], &["release", "version"]),
        agent("monitor", "Monitors system health and metrics", ModelTier::Haiku,
              &["monitoring", "observability"], &["read", "bash"], &["monitor", "observe"]),
    ]
}

fn agent(
    name: &str,
    description: &str,
    tier: ModelTier,
    tags: &[&str],
    tools: &[&str],
    capabilities: &[&str],
) -> AgentDef {
    AgentDef {
        name: name.to_string(),
        description: description.to_string(),
        system_prompt: String::new(),
        model_tier: tier,
        tool_permissions: tools.iter().map(|s| s.to_string()).collect(),
        tags: tags.iter().map(|s| s.to_string()).collect(),
        capabilities: capabilities.iter().map(|s| s.to_string()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_agents_count() {
        let registry = AgentRegistry::with_builtins();
        assert_eq!(registry.count(), 19);
    }

    #[test]
    fn test_get_agent() {
        let registry = AgentRegistry::with_builtins();
        let architect = registry.get("architect").unwrap();
        assert_eq!(architect.model_tier, ModelTier::Opus);
    }

    #[test]
    fn test_get_normalized() {
        let registry = AgentRegistry::with_builtins();
        let result = registry.get_normalized("ARCHITECT");
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "architect");
    }

    #[test]
    fn test_get_missing() {
        let registry = AgentRegistry::with_builtins();
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_list_names() {
        let registry = AgentRegistry::with_builtins();
        let names = registry.list_names();
        assert_eq!(names.len(), 19);
        // Should be sorted
        for i in 1..names.len() {
            assert!(names[i] >= names[i - 1]);
        }
    }

    #[test]
    fn test_agents_for_tier() {
        let registry = AgentRegistry::with_builtins();
        let opus_agents = registry.agents_for_tier(ModelTier::Opus);
        assert!(!opus_agents.is_empty());
        for a in &opus_agents {
            assert_eq!(a.model_tier, ModelTier::Opus);
        }
    }

    #[test]
    fn test_find_by_tag() {
        let registry = AgentRegistry::with_builtins();
        let security = registry.find_by_tag("security");
        assert!(!security.is_empty());
    }

    #[test]
    fn test_find_by_capability() {
        let registry = AgentRegistry::with_builtins();
        let coders = registry.find_by_capability("code");
        assert!(!coders.is_empty());
    }

    #[test]
    fn test_register_custom_agent() {
        let mut registry = AgentRegistry::new();
        registry.register(AgentDef {
            name: "custom".to_string(),
            description: "Custom agent".to_string(),
            system_prompt: "Be custom".to_string(),
            model_tier: ModelTier::Sonnet,
            tool_permissions: vec!["read".to_string()],
            tags: vec!["custom".to_string()],
            capabilities: vec![],
        });
        assert_eq!(registry.count(), 1);
        assert!(registry.get("custom").is_some());
    }

    #[test]
    fn test_unregister() {
        let mut registry = AgentRegistry::with_builtins();
        let initial = registry.count();
        let removed = registry.unregister("analyst");
        assert!(removed.is_some());
        assert_eq!(registry.count(), initial - 1);
    }

    #[test]
    fn test_build_prompt_with_system() {
        let agent = AgentDef {
            name: "test".to_string(),
            description: "Test".to_string(),
            system_prompt: "You are a test agent.".to_string(),
            model_tier: ModelTier::Haiku,
            tool_permissions: vec![],
            tags: vec![],
            capabilities: vec![],
        };
        let prompt = agent.build_prompt("some context");
        assert!(prompt.contains("You are a test agent."));
        assert!(prompt.contains("some context"));
    }

    #[test]
    fn test_build_prompt_without_system() {
        let agent = AgentDef {
            name: "helper".to_string(),
            description: "Helps with things".to_string(),
            system_prompt: String::new(),
            model_tier: ModelTier::Haiku,
            tool_permissions: vec![],
            tags: vec![],
            capabilities: vec![],
        };
        let prompt = agent.build_prompt("ctx");
        assert!(prompt.contains("helper"));
        assert!(prompt.contains("Helps with things"));
    }

    #[test]
    fn test_tool_permission_check() {
        let agent = AgentDef {
            name: "limited".to_string(),
            description: "Limited".to_string(),
            system_prompt: String::new(),
            model_tier: ModelTier::Haiku,
            tool_permissions: vec!["read".to_string(), "bash".to_string()],
            tags: vec![],
            capabilities: vec![],
        };
        assert!(agent.is_tool_permitted("read"));
        assert!(agent.is_tool_permitted("bash"));
        assert!(!agent.is_tool_permitted("write"));
    }

    #[test]
    fn test_empty_tool_permissions_allows_all() {
        let agent = AgentDef {
            name: "open".to_string(),
            description: "Open".to_string(),
            system_prompt: String::new(),
            model_tier: ModelTier::Haiku,
            tool_permissions: vec![],
            tags: vec![],
            capabilities: vec![],
        };
        assert!(agent.is_tool_permitted("anything"));
    }

    #[test]
    fn test_resolve_model_bedrock() {
        let registry = AgentRegistry::with_builtins();
        let tier = registry.resolve_model("architect", "bedrock");
        assert_eq!(tier, Some(ModelTier::Sonnet)); // Downgraded from Opus
    }

    #[test]
    fn test_resolve_model_default() {
        let registry = AgentRegistry::with_builtins();
        let tier = registry.resolve_model("architect", "anthropic");
        assert_eq!(tier, Some(ModelTier::Opus));
    }

    #[test]
    fn test_resolve_model_missing() {
        let registry = AgentRegistry::with_builtins();
        let tier = registry.resolve_model("missing", "anthropic");
        assert_eq!(tier, None);
    }

    #[test]
    fn test_has_capability() {
        let agent = agent("test", "test", ModelTier::Haiku, &[], &[], &["debug", "fix"]);
        assert!(agent.has_capability("debug"));
        assert!(!agent.has_capability("deploy"));
    }

    #[test]
    fn test_agent_serialization() {
        let a = agent("test", "desc", ModelTier::Sonnet, &["t1"], &["read"], &["cap1"]);
        let json = serde_json::to_string(&a).unwrap();
        let restored: AgentDef = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "test");
        assert_eq!(restored.model_tier, ModelTier::Sonnet);
    }
}
