//! Task decomposition: 7-step heuristic pipeline.
//! Classify -> Scope -> Split -> Dependencies -> Order -> Validate -> Format

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Task classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    SingleStep,
    MultiStep,
    Research,
    Debug,
    Refactor,
}

/// Complexity scope estimate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskScope {
    pub estimated_loc: usize,
    pub estimated_files: usize,
    pub concepts: Vec<String>,
    pub complexity: Complexity,
}

/// Complexity level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Complexity {
    Low,
    Medium,
    High,
}

/// A subtask with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtask {
    pub id: usize,
    pub title: String,
    pub description: String,
    pub file_ownership: Vec<String>,
    pub dependencies: Vec<usize>,
    pub estimated_effort: u32, // minutes
}

/// A decomposed task plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposedPlan {
    pub original_task: String,
    pub task_type: TaskType,
    pub scope: TaskScope,
    pub subtasks: Vec<Subtask>,
    pub execution_order: Vec<usize>,
    pub is_valid: bool,
    pub validation_errors: Vec<String>,
}

/// The task decomposer engine.
pub struct TaskDecomposer;

impl TaskDecomposer {
    pub fn new() -> Self {
        Self
    }

    /// Decompose a task description into a structured plan.
    pub fn decompose(&self, task: &str) -> DecomposedPlan {
        // Step 1: Classify
        let task_type = self.classify(task);

        // Step 2: Scope
        let scope = self.estimate_scope(task, &task_type);

        // Step 3: Split
        let mut subtasks = self.split_into_subtasks(task, &task_type);

        // Step 4: Dependencies
        self.detect_dependencies(&mut subtasks);

        // Step 5: Order
        let execution_order = self.topological_sort(&subtasks);

        // Step 6: Validate
        let validation_errors = self.validate(&subtasks, &execution_order);

        // Step 7: Format
        DecomposedPlan {
            original_task: task.to_string(),
            task_type,
            scope,
            subtasks,
            execution_order: execution_order.clone(),
            is_valid: validation_errors.is_empty(),
            validation_errors,
        }
    }

    /// Step 1: Classify the task type.
    fn classify(&self, task: &str) -> TaskType {
        let lower = task.to_lowercase();

        if lower.contains("debug") || lower.contains("fix bug") || lower.contains("error") {
            return TaskType::Debug;
        }
        if lower.contains("refactor") || lower.contains("restructure") || lower.contains("reorganize") {
            return TaskType::Refactor;
        }
        if lower.contains("research") || lower.contains("investigate") || lower.contains("analyze") {
            return TaskType::Research;
        }

        // Count action verbs to detect multi-step
        let action_words = ["create", "implement", "add", "write", "build", "test", "deploy", "configure", "setup", "update"];
        let action_count = action_words.iter().filter(|w| lower.contains(*w)).count();

        if action_count >= 2 || lower.contains(" and ") || lower.contains(" then ") {
            TaskType::MultiStep
        } else {
            TaskType::SingleStep
        }
    }

    /// Step 2: Estimate the scope.
    fn estimate_scope(&self, task: &str, task_type: &TaskType) -> TaskScope {
        let lower = task.to_lowercase();
        let word_count = task.split_whitespace().count();

        // Extract concepts mentioned
        let concept_keywords = [
            "api", "database", "auth", "ui", "frontend", "backend", "testing",
            "deployment", "configuration", "monitoring", "logging", "security",
            "performance", "caching", "queue", "notification", "migration",
        ];
        let concepts: Vec<String> = concept_keywords
            .iter()
            .filter(|c| lower.contains(*c))
            .map(|c| c.to_string())
            .collect();

        let (estimated_loc, estimated_files, complexity) = match task_type {
            TaskType::SingleStep => (50, 1, Complexity::Low),
            TaskType::Debug => (30, 2, Complexity::Medium),
            TaskType::Research => (0, 0, Complexity::Low),
            TaskType::Refactor => {
                if word_count > 50 {
                    (300, 8, Complexity::High)
                } else {
                    (150, 4, Complexity::Medium)
                }
            }
            TaskType::MultiStep => {
                if concepts.len() > 3 {
                    (500, 10, Complexity::High)
                } else if concepts.len() > 1 {
                    (200, 5, Complexity::Medium)
                } else {
                    (100, 3, Complexity::Low)
                }
            }
        };

        TaskScope {
            estimated_loc,
            estimated_files,
            concepts,
            complexity,
        }
    }

    /// Step 3: Split into subtasks.
    fn split_into_subtasks(&self, task: &str, task_type: &TaskType) -> Vec<Subtask> {
        match task_type {
            TaskType::SingleStep => {
                vec![Subtask {
                    id: 1,
                    title: "Execute task".to_string(),
                    description: task.to_string(),
                    file_ownership: vec![],
                    dependencies: vec![],
                    estimated_effort: 15,
                }]
            }
            TaskType::Debug => {
                vec![
                    Subtask {
                        id: 1,
                        title: "Reproduce the issue".to_string(),
                        description: "Identify and reproduce the bug".to_string(),
                        file_ownership: vec![],
                        dependencies: vec![],
                        estimated_effort: 10,
                    },
                    Subtask {
                        id: 2,
                        title: "Root cause analysis".to_string(),
                        description: "Find the root cause of the issue".to_string(),
                        file_ownership: vec![],
                        dependencies: vec![1],
                        estimated_effort: 20,
                    },
                    Subtask {
                        id: 3,
                        title: "Implement fix".to_string(),
                        description: "Apply the fix for the identified root cause".to_string(),
                        file_ownership: vec![],
                        dependencies: vec![2],
                        estimated_effort: 15,
                    },
                    Subtask {
                        id: 4,
                        title: "Verify fix".to_string(),
                        description: "Test that the fix resolves the issue".to_string(),
                        file_ownership: vec![],
                        dependencies: vec![3],
                        estimated_effort: 10,
                    },
                ]
            }
            TaskType::Research => {
                vec![
                    Subtask {
                        id: 1,
                        title: "Gather information".to_string(),
                        description: "Research and collect relevant information".to_string(),
                        file_ownership: vec![],
                        dependencies: vec![],
                        estimated_effort: 30,
                    },
                    Subtask {
                        id: 2,
                        title: "Analyze findings".to_string(),
                        description: "Analyze and synthesize the collected information".to_string(),
                        file_ownership: vec![],
                        dependencies: vec![1],
                        estimated_effort: 20,
                    },
                    Subtask {
                        id: 3,
                        title: "Summarize results".to_string(),
                        description: "Create a summary of findings and recommendations".to_string(),
                        file_ownership: vec![],
                        dependencies: vec![2],
                        estimated_effort: 15,
                    },
                ]
            }
            TaskType::Refactor | TaskType::MultiStep => {
                self.extract_subtasks_from_text(task)
            }
        }
    }

    /// Extract subtasks from task text by analyzing structure.
    fn extract_subtasks_from_text(&self, task: &str) -> Vec<Subtask> {
        let mut subtasks = Vec::new();
        let mut id = 1;

        // Try to split by numbered items, "and", or sentences
        let lines: Vec<&str> = task.lines().collect();

        // Check for numbered list
        let numbered_items: Vec<&str> = lines
            .iter()
            .filter(|l| {
                let trimmed = l.trim();
                trimmed.starts_with(|c: char| c.is_ascii_digit())
                    && (trimmed.contains('.') || trimmed.contains(')'))
            })
            .copied()
            .collect();

        if numbered_items.len() >= 2 {
            for item in numbered_items {
                let cleaned = item.trim().trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ')').trim();
                subtasks.push(Subtask {
                    id,
                    title: truncate_str(cleaned, 80),
                    description: cleaned.to_string(),
                    file_ownership: vec![],
                    dependencies: if id > 1 { vec![id - 1] } else { vec![] },
                    estimated_effort: 20,
                });
                id += 1;
            }
        } else {
            // Split by conjunctions
            let parts: Vec<&str> = task.split(" and ").collect();
            if parts.len() >= 2 {
                for part in parts {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() {
                        subtasks.push(Subtask {
                            id,
                            title: truncate_str(trimmed, 80),
                            description: trimmed.to_string(),
                            file_ownership: vec![],
                            dependencies: if id > 1 { vec![id - 1] } else { vec![] },
                            estimated_effort: 20,
                        });
                        id += 1;
                    }
                }
            } else {
                // Single complex task
                subtasks.push(Subtask {
                    id: 1,
                    title: "Analyze requirements".to_string(),
                    description: "Understand what needs to be done".to_string(),
                    file_ownership: vec![],
                    dependencies: vec![],
                    estimated_effort: 10,
                });
                subtasks.push(Subtask {
                    id: 2,
                    title: "Implement changes".to_string(),
                    description: task.to_string(),
                    file_ownership: vec![],
                    dependencies: vec![1],
                    estimated_effort: 30,
                });
                subtasks.push(Subtask {
                    id: 3,
                    title: "Test and verify".to_string(),
                    description: "Verify the implementation works correctly".to_string(),
                    file_ownership: vec![],
                    dependencies: vec![2],
                    estimated_effort: 15,
                });
            }
        }

        subtasks
    }

    /// Step 4: Detect dependencies between subtasks.
    fn detect_dependencies(&self, subtasks: &mut [Subtask]) {
        // File-based dependency detection
        let ownership_map: HashMap<usize, HashSet<String>> = subtasks
            .iter()
            .map(|s| {
                (s.id, s.file_ownership.iter().cloned().collect())
            })
            .collect();

        for i in 0..subtasks.len() {
            for j in 0..i {
                let files_i: &HashSet<String> = &ownership_map[&subtasks[i].id];
                let files_j: &HashSet<String> = &ownership_map[&subtasks[j].id];
                if !files_i.is_empty() && !files_j.is_empty() && !files_i.is_disjoint(files_j) {
                    let dep_id = subtasks[j].id;
                    if !subtasks[i].dependencies.contains(&dep_id) {
                        subtasks[i].dependencies.push(dep_id);
                    }
                }
            }
        }
    }

    /// Step 5: Topological sort by dependencies.
    fn topological_sort(&self, subtasks: &[Subtask]) -> Vec<usize> {
        let mut in_degree: HashMap<usize, usize> = HashMap::new();
        let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();

        for st in subtasks {
            in_degree.entry(st.id).or_insert(0);
            adj.entry(st.id).or_default();
            for &dep in &st.dependencies {
                adj.entry(dep).or_default().push(st.id);
                *in_degree.entry(st.id).or_insert(0) += 1;
            }
        }

        let mut queue: Vec<usize> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();
        queue.sort();

        let mut order = Vec::new();
        while let Some(node) = queue.first().copied() {
            queue.remove(0);
            order.push(node);
            if let Some(neighbors) = adj.get(&node) {
                for &next in neighbors {
                    if let Some(deg) = in_degree.get_mut(&next) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(next);
                            queue.sort();
                        }
                    }
                }
            }
        }

        order
    }

    /// Step 6: Validate the decomposition.
    fn validate(&self, subtasks: &[Subtask], order: &[usize]) -> Vec<String> {
        let mut errors = Vec::new();

        // Check for orphan dependencies
        let ids: HashSet<usize> = subtasks.iter().map(|s| s.id).collect();
        for st in subtasks {
            for dep in &st.dependencies {
                if !ids.contains(dep) {
                    errors.push(format!("Subtask {} depends on non-existent task {}", st.id, dep));
                }
            }
        }

        // Check for cycles (order should contain all tasks)
        if order.len() != subtasks.len() {
            errors.push("Dependency cycle detected".to_string());
        }

        // Check for empty subtasks
        for st in subtasks {
            if st.title.is_empty() {
                errors.push(format!("Subtask {} has empty title", st.id));
            }
        }

        errors
    }
}

impl Default for TaskDecomposer {
    fn default() -> Self {
        Self::new()
    }
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if max_len < 4 {
        // Too small for truncation with "..."
        return s.chars().take(max_len).collect();
    }
    let char_count: usize = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{}...", truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decomposer() -> TaskDecomposer {
        TaskDecomposer::new()
    }

    #[test]
    fn test_classify_single_step() {
        let d = decomposer();
        assert_eq!(d.classify("rename the function"), TaskType::SingleStep);
    }

    #[test]
    fn test_classify_multi_step() {
        let d = decomposer();
        assert_eq!(
            d.classify("create a new module and add tests for it"),
            TaskType::MultiStep
        );
    }

    #[test]
    fn test_classify_debug() {
        let d = decomposer();
        assert_eq!(d.classify("debug the failing test"), TaskType::Debug);
    }

    #[test]
    fn test_classify_refactor() {
        let d = decomposer();
        assert_eq!(d.classify("refactor the auth module"), TaskType::Refactor);
    }

    #[test]
    fn test_classify_research() {
        let d = decomposer();
        assert_eq!(d.classify("investigate the memory leak"), TaskType::Research);
    }

    #[test]
    fn test_decompose_single_step() {
        let d = decomposer();
        let plan = d.decompose("rename variable x to count");
        assert_eq!(plan.task_type, TaskType::SingleStep);
        assert_eq!(plan.subtasks.len(), 1);
        assert!(plan.is_valid);
    }

    #[test]
    fn test_decompose_debug() {
        let d = decomposer();
        let plan = d.decompose("debug the null pointer error in user service");
        assert_eq!(plan.task_type, TaskType::Debug);
        assert_eq!(plan.subtasks.len(), 4);
        assert_eq!(plan.subtasks[0].title, "Reproduce the issue");
        assert!(plan.is_valid);
    }

    #[test]
    fn test_decompose_research() {
        let d = decomposer();
        let plan = d.decompose("research best practices for API design");
        assert_eq!(plan.task_type, TaskType::Research);
        assert_eq!(plan.subtasks.len(), 3);
    }

    #[test]
    fn test_decompose_multi_step_with_and() {
        let d = decomposer();
        let plan = d.decompose("create the user model and add validation and write tests");
        assert_eq!(plan.task_type, TaskType::MultiStep);
        assert!(plan.subtasks.len() >= 3);
    }

    #[test]
    fn test_decompose_numbered_list() {
        let d = decomposer();
        let task = "1. Create the database schema\n2. Implement the API endpoints\n3. Write integration tests\n4. Deploy to staging";
        let plan = d.decompose(task);
        assert!(plan.subtasks.len() >= 4);
    }

    #[test]
    fn test_execution_order_respects_dependencies() {
        let d = decomposer();
        let plan = d.decompose("debug the login error");
        // Each subtask should appear after its dependencies in the order
        let pos: HashMap<usize, usize> = plan
            .execution_order
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, i))
            .collect();
        for st in &plan.subtasks {
            for dep in &st.dependencies {
                assert!(pos[dep] < pos[&st.id], "Dependency {} should come before {}", dep, st.id);
            }
        }
    }

    #[test]
    fn test_scope_estimation_simple() {
        let d = decomposer();
        let scope = d.estimate_scope("rename x", &TaskType::SingleStep);
        assert_eq!(scope.complexity, Complexity::Low);
        assert_eq!(scope.estimated_files, 1);
    }

    #[test]
    fn test_scope_estimation_complex() {
        let d = decomposer();
        let scope = d.estimate_scope(
            "build api with database, auth, caching, and monitoring",
            &TaskType::MultiStep,
        );
        assert_eq!(scope.complexity, Complexity::High);
        assert!(scope.concepts.len() > 3);
    }

    #[test]
    fn test_validation_passes_for_valid_plan() {
        let d = decomposer();
        let plan = d.decompose("fix the bug in auth");
        assert!(plan.is_valid);
        assert!(plan.validation_errors.is_empty());
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world!", 8), "hello...");
    }

    #[test]
    fn test_truncate_str_unicode() {
        // Multi-byte chars must not panic from byte-offset slicing
        let unicode_str = "한국어 텍스트 입력 테스트 문장";
        let result = truncate_str(unicode_str, 8);
        assert!(result.ends_with("..."));
        assert!(result.chars().count() <= 8);
    }

    #[test]
    fn test_truncate_str_small_max() {
        assert_eq!(truncate_str("hello", 2), "he");
    }

    #[test]
    fn test_topological_sort_simple() {
        let d = decomposer();
        let subtasks = vec![
            Subtask { id: 1, title: "A".into(), description: String::new(), file_ownership: vec![], dependencies: vec![], estimated_effort: 10 },
            Subtask { id: 2, title: "B".into(), description: String::new(), file_ownership: vec![], dependencies: vec![1], estimated_effort: 10 },
            Subtask { id: 3, title: "C".into(), description: String::new(), file_ownership: vec![], dependencies: vec![1], estimated_effort: 10 },
            Subtask { id: 4, title: "D".into(), description: String::new(), file_ownership: vec![], dependencies: vec![2, 3], estimated_effort: 10 },
        ];
        let order = d.topological_sort(&subtasks);
        assert_eq!(order, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_plan_serialization() {
        let d = decomposer();
        let plan = d.decompose("rename x to y");
        let json = serde_json::to_string(&plan).unwrap();
        let restored: DecomposedPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.task_type, plan.task_type);
    }

    #[test]
    fn test_refactor_scope() {
        let d = decomposer();
        let scope = d.estimate_scope("refactor", &TaskType::Refactor);
        assert_eq!(scope.complexity, Complexity::Medium);
    }

    #[test]
    fn test_empty_task() {
        let d = decomposer();
        let plan = d.decompose("");
        assert_eq!(plan.task_type, TaskType::SingleStep);
        assert!(!plan.subtasks.is_empty());
    }
}
