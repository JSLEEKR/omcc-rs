//! omcc-rs: High-performance Claude Code orchestration toolkit.
//! Rust reimplementation of oh-my-claudecode.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use omcc_rs::agents::AgentRegistry;
use omcc_rs::autopilot::AutopilotPipeline;
use omcc_rs::config::{ModelTier, OmcConfig};
use omcc_rs::decompose::TaskDecomposer;
use omcc_rs::hook::{HookBridge, HookOutput};
use omcc_rs::hud::{HudRenderer, HudState};
use omcc_rs::keyword::KeywordDetector;
use omcc_rs::router::{ModelRouter, RoutingContext};
use omcc_rs::skills::SkillLearner;

#[derive(Parser)]
#[command(name = "omcc", version = "1.0.0", about = "High-performance Claude Code orchestration toolkit")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Config file path
    #[arg(short, long, default_value = ".omc/config.yaml")]
    config: PathBuf,

    /// State directory
    #[arg(short, long, default_value = ".omc/state")]
    state_dir: PathBuf,

    /// Disable colors
    #[arg(long, global = true)]
    no_color: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run in hook bridge mode (reads JSON from stdin)
    Hook,

    /// Analyze a prompt and show routing decision
    Route {
        /// The prompt to analyze
        prompt: String,
    },

    /// Decompose a task into subtasks
    Decompose {
        /// The task description
        task: String,
    },

    /// Detect magic keywords in text
    Keywords {
        /// The text to scan
        text: String,
    },

    /// List all available agents
    Agents {
        /// Filter by model tier
        #[arg(short, long)]
        tier: Option<String>,
    },

    /// Show HUD statusline
    Status,

    /// Show learned skills
    Skills,

    /// Start autopilot pipeline for a task
    Autopilot {
        /// The task description
        task: String,
        /// Maximum iterations
        #[arg(short, long, default_value = "10")]
        max_iter: u32,
    },
}

fn main() {
    let cli = Cli::parse();

    // Load config
    let config = if cli.config.exists() {
        OmcConfig::from_yaml_file(&cli.config).unwrap_or_default()
    } else {
        OmcConfig::default()
    };

    match cli.command {
        Some(Commands::Hook) => run_hook_bridge(&config),
        Some(Commands::Route { prompt }) => run_route(&prompt),
        Some(Commands::Decompose { task }) => run_decompose(&task),
        Some(Commands::Keywords { text }) => run_keywords(&text),
        Some(Commands::Agents { tier }) => run_agents(tier.as_deref()),
        Some(Commands::Status) => run_status(cli.no_color),
        Some(Commands::Skills) => run_skills(),
        Some(Commands::Autopilot { task, max_iter }) => run_autopilot(&task, max_iter),
        None => run_hook_bridge(&config),
    }
}

fn run_hook_bridge(config: &OmcConfig) {
    let mut bridge = HookBridge::new();
    let detector = KeywordDetector::new();
    let router = ModelRouter::new();

    // Register keyword detection handler
    bridge.register("keyword_detector", move |input| {
        let text = input.payload.as_str().unwrap_or("");
        let matches = detector.detect(text);
        if matches.is_empty() {
            Ok(HookOutput::passthrough())
        } else {
            let mode = &matches[0].mode;
            Ok(HookOutput::with_injection(&format!(
                "Mode activated: {}",
                mode
            )))
        }
    });

    // Register model selection handler
    bridge.register("model_selection", move |input| {
        let prompt = input.payload.as_str().unwrap_or("");
        let decision = router.route(prompt, &RoutingContext::default());
        Ok(HookOutput::with_model(&decision.tier.to_string()))
    });

    // Register session lifecycle
    bridge.register("session_start", |_input| Ok(HookOutput::passthrough()));
    bridge.register("session_end", |_input| Ok(HookOutput::passthrough()));
    bridge.register("permission_check", |_input| Ok(HookOutput::passthrough()));
    bridge.register("notification", |_input| Ok(HookOutput::passthrough()));
    bridge.register("pre_tool_use", |_input| Ok(HookOutput::passthrough()));
    bridge.register("post_tool_use", |_input| Ok(HookOutput::passthrough()));

    let _ = config;
    if let Err(e) = bridge.run_oneshot() {
        eprintln!("Hook bridge error: {}", e);
        std::process::exit(1);
    }
}

fn run_route(prompt: &str) {
    let router = ModelRouter::new();
    let decision = router.route(prompt, &RoutingContext::default());

    println!("Routing Decision:");
    println!("  Tier:       {}", decision.tier);
    println!("  Score:      {:.3}", decision.score);
    println!("  Lexical:    {:.3}", decision.lexical_score);
    println!("  Structural: {:.3}", decision.structural_score);
    println!("  Context:    {:.3}", decision.context_score);
    println!("  Confidence: {:.3}", decision.confidence);
    println!("  Reason:     {}", decision.reason);
}

fn run_decompose(task: &str) {
    let decomposer = TaskDecomposer::new();
    let plan = decomposer.decompose(task);

    println!("Task Decomposition:");
    println!("  Type:       {:?}", plan.task_type);
    println!("  Complexity: {:?}", plan.scope.complexity);
    println!("  Est. LOC:   {}", plan.scope.estimated_loc);
    println!("  Est. Files: {}", plan.scope.estimated_files);
    println!("  Valid:      {}", plan.is_valid);
    println!();
    println!("Subtasks ({}):", plan.subtasks.len());
    for st in &plan.subtasks {
        let deps = if st.dependencies.is_empty() {
            "none".to_string()
        } else {
            st.dependencies.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(",")
        };
        println!("  [{}] {} (deps: {}, ~{}min)", st.id, st.title, deps, st.estimated_effort);
    }
    println!();
    println!("Execution Order: {:?}", plan.execution_order);
}

fn run_keywords(text: &str) {
    let detector = KeywordDetector::new();
    let matches = detector.detect(text);

    if matches.is_empty() {
        println!("No keywords detected.");
    } else {
        println!("Detected Keywords:");
        for m in &matches {
            println!("  - \"{}\" -> mode: {} (pos: {})", m.keyword, m.mode, m.position);
        }
    }
}

fn run_agents(tier_filter: Option<&str>) {
    let registry = AgentRegistry::with_builtins();

    let tier = tier_filter.and_then(|t| match t.to_lowercase().as_str() {
        "haiku" => Some(ModelTier::Haiku),
        "sonnet" => Some(ModelTier::Sonnet),
        "opus" => Some(ModelTier::Opus),
        _ => None,
    });

    println!("Registered Agents ({}):", registry.count());
    println!("{:<15} {:<8} {:<50}", "Name", "Tier", "Description");
    println!("{}", "-".repeat(73));

    let names = registry.list_names();
    for name in names {
        if let Some(agent) = registry.get(name) {
            if let Some(ref filter_tier) = tier {
                if &agent.model_tier != filter_tier {
                    continue;
                }
            }
            println!("{:<15} {:<8} {:<50}", agent.name, agent.model_tier, agent.description);
        }
    }
}

fn run_status(no_color: bool) {
    let renderer = HudRenderer::new(120, !no_color);
    let state = HudState {
        mode: Some("idle".to_string()),
        model: Some("sonnet".to_string()),
        stage: Some("IDLE".to_string()),
        progress: Some(0),
        ..Default::default()
    };
    println!("{}", renderer.render(&state));
}

fn run_skills() {
    let learner = SkillLearner::new();
    let skills = learner.skills();
    if skills.is_empty() {
        println!("No learned skills yet.");
    } else {
        for skill in skills {
            println!("  [{:.0}%] {} - {}", skill.confidence, skill.name, skill.description);
        }
    }
}

fn run_autopilot(task: &str, max_iter: u32) {
    let mut pipeline = AutopilotPipeline::new(task, max_iter, 3);
    pipeline.start();
    println!("Autopilot started: {}", pipeline.summary());
    println!("Task: {}", task);
    println!("Use hook bridge mode for actual pipeline execution.");
}
