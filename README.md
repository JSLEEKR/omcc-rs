# omcc-rs

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021-orange.svg?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/Tests-170-green.svg?style=for-the-badge)](src/)

High-performance Claude Code orchestration toolkit -- a Rust reimplementation of [oh-my-claudecode](https://github.com/Yeachan-Heo/oh-my-claudecode) (19.5K stars).

## Why This Exists

oh-my-claudecode is a powerful multi-agent orchestration system for Claude Code, adding magic keywords, smart model routing, task decomposition, autopilot pipelines, and more. However, it's built in TypeScript/Node.js, which means:

- **200-500ms cold start** on every hook invocation
- **~50MB** of node_modules
- **50-200ms latency** per hook call
- **80-150MB** memory footprint

omcc-rs solves all of these by reimplementing the core orchestration engine in Rust:

| Dimension | TypeScript (Original) | Rust (omcc-rs) |
|-----------|----------------------|----------------|
| Startup time | ~200-500ms | <5ms |
| Binary size | ~50MB (node_modules) | ~5-10MB single binary |
| Memory usage | ~80-150MB (V8 heap) | ~5-15MB |
| Hook latency | 50-200ms per call | <5ms per call |
| Dependencies | 5+ npm packages | Zero runtime deps |
| Type safety | Runtime (Zod) | Compile-time (serde) |
| Deployment | npm install + Node.js 20+ | Single binary |

## Features

### Hook Bridge (Central Router)
The heart of omcc-rs. Reads JSON from stdin, routes to 14+ hook handlers, writes JSON to stdout. Handles the full Claude Code hook lifecycle:

- `session_start` / `session_end` -- Session lifecycle management
- `pre_tool_use` / `post_tool_use` -- Tool call validation and learning
- `keyword_detector` -- Magic keyword detection
- `model_selection` -- Smart model routing
- `task_decomposition` -- Automatic task breakdown
- `agent_delegation` -- Agent selection and routing
- `permission_check` -- Tool permission validation
- `notification` -- Event dispatching
- `context_injection` -- Dynamic rule injection
- `recovery` -- Error recovery
- `pre_compact` / `post_compact` -- State persistence around compaction

### Magic Keywords
Natural language triggers that activate specialized modes without memorizing commands:

| Keyword | Aliases | Mode |
|---------|---------|------|
| `autopilot` | `auto pilot`, `auto-pilot` | Autopilot pipeline |
| `ralph` | `persistent`, `persist mode` | Persistent verify-fix loops |
| `ultrawork` | `ulw`, `ultra work` | Ultra-work mode |
| `ultrathink` | `ultra think`, `deep think` | Deep thinking mode |
| `compact` | `compress context` | Context compaction |
| `research mode` | `deep research` | Research mode |
| `plan mode` | `planning mode` | Planning mode |
| `debug mode` | `debugging mode` | Debug mode |

Features:
- Code block stripping (avoids false matches inside code)
- Case-insensitive matching
- Non-Latin script detection (CJK support)
- Task-size suppression (short prompts don't trigger complex modes)
- Custom keyword registration

### Smart Model Routing
Proactive complexity analysis routes tasks to the optimal model tier:

```
score = lexical_score(prompt) * 0.4
      + structural_score(prompt) * 0.3
      + context_score(history) * 0.3

if score < 0.3 -> Haiku (fast, simple tasks)
if score < 0.7 -> Sonnet (balanced)
else           -> Opus (complex reasoning)
```

**Lexical signals**: Code patterns (implement, architect, refactor), technical terms (database, microservice, distributed), prompt length, simple task detection.

**Structural signals**: Nesting depth, file path references, code block presence, numbered lists/structured plans.

**Context signals**: Conversation length, file count, error presence, previous tier momentum.

Each decision includes a confidence score indicating how much the signals agree.

### Task Decomposition
7-step heuristic pipeline that breaks complex tasks into structured plans:

1. **Classify**: Single-step, multi-step, debug, refactor, or research
2. **Scope**: Estimate LOC, file count, concepts, complexity level
3. **Split**: Break into atomic subtasks based on task structure
4. **Dependencies**: Detect inter-task dependencies from file ownership
5. **Order**: Topological sort to determine execution sequence
6. **Validate**: Check for orphan dependencies, cycles, empty tasks
7. **Format**: Output as structured plan with execution order

Supports numbered lists, conjunction splitting, and template-based decomposition for common task types (debugging, research).

### Autopilot Pipeline
4-stage state machine with configurable behavior:

```
IDLE -> PLANNING -> EXECUTING -> VERIFYING -> QA -> COMPLETE
                    ^                |
                    |   FAILED <-----+ (retry up to max_retries)
```

- Configurable max iterations and max retries
- Automatic verification and QA stages (can be disabled)
- Verification failure retries from the executing stage
- Pipeline abort and reset support
- Full state serialization for persistence
- Progress percentage tracking

### 19 Specialized Agents
Built-in agent registry with model tier assignments and capability definitions:

| Agent | Tier | Role |
|-------|------|------|
| analyst | Sonnet | Requirements analysis |
| architect | Opus | System design |
| executor | Sonnet | Code implementation |
| debugger | Sonnet | Bug finding and fixing |
| verifier | Sonnet | Correctness verification |
| reviewer | Sonnet | Code review |
| tester | Sonnet | Test writing |
| documenter | Haiku | Documentation |
| refactorer | Sonnet | Code improvement |
| optimizer | Opus | Performance optimization |
| security | Opus | Security auditing |
| devops | Sonnet | Deployment/infrastructure |
| planner | Opus | Execution planning |
| researcher | Sonnet | Technology investigation |
| migrator | Sonnet | Data/schema migration |
| mentor | Haiku | Concept explanation |
| qa | Sonnet | Integration testing |
| releaser | Haiku | Release management |
| monitor | Haiku | System monitoring |

Features:
- Case-insensitive lookup
- Custom agent registration
- Tool permission enforcement
- Provider compatibility (Bedrock downgrade)
- Tag and capability-based search

### Skill Learning
Automatic pattern detection from successful tool uses:

- SHA-256 deduplication prevents duplicate patterns
- Confidence scoring: base 50, +5 per reuse, -10 on failure
- Promotion threshold: 70+ confidence becomes a learned skill
- Trigger word extraction (stopword filtering)
- Import/export for persistence across sessions

### HUD Statusline
Terminal statusline rendering with full state display:

```
[AP] | sonnet | @executor | [EXECUTING 40%] | git:main | ctx:30% | tok:15k/5k | rate:80 | agents:[executor,reviewer]
```

Features:
- Mode-specific icons (AP, RL, UW, UT, CM)
- Color-coded indicators (context usage, rate limits)
- Model-specific coloring (Opus=magenta, Sonnet=blue, Haiku=green)
- Compact mode for narrow terminals
- Progress bar rendering
- ANSI stripping for width calculation
- Configurable max width with smart truncation

### State Persistence
File-based state management under `.omc/state/`:

- Session isolation with unique IDs
- Atomic writes (write to temp, then rename)
- Key-value state entries with optional TTL
- Notepad system (survives compaction)
- Automatic cleanup of expired entries
- Session listing, deletion, and recovery

### Configuration
YAML-based configuration for all subsystems:

```yaml
agents:
  - name: custom-agent
    description: "My custom agent"
    model_tier: opus
    tool_permissions: [read, write, bash]
    tags: [custom]

routing_rules:
  - name: complex_tasks
    threshold: 0.7
    target_tier: opus
    patterns: ["architect", "design"]

autopilot:
  max_retries: 5
  max_iterations: 20
  auto_verify: true
  auto_qa: true

keywords:
  custom_mode:
    triggers: ["mymode", "custom"]
    mode: "custom"
    description: "My custom mode"
```

## Installation

```bash
# Build from source
cargo build --release

# The binary is at target/release/omcc (or omcc.exe on Windows)
```

## Usage

### Hook Bridge Mode (Default)
```bash
# Reads JSON from stdin, processes through registered handlers, outputs JSON to stdout
echo '{"hook_type": "keyword_detector", "payload": "autopilot fix all bugs"}' | omcc hook
```

### Model Routing
```bash
omcc route "Implement a distributed microservice with authentication"
# Output:
# Routing Decision:
#   Tier:       opus
#   Score:      0.782
#   Lexical:    0.600
#   Structural: 0.150
#   Context:    0.000
#   Confidence: 0.823
#   Reason:     Complex task -- high complexity signals
```

### Task Decomposition
```bash
omcc decompose "Create a REST API with database integration and deploy to production"
# Output:
# Task Decomposition:
#   Type:       MultiStep
#   Complexity: High
#   Subtasks (3):
#     [1] Create a REST API (deps: none, ~20min)
#     [2] database integration (deps: 1, ~20min)
#     [3] deploy to production (deps: 2, ~20min)
```

### Keyword Detection
```bash
omcc keywords "autopilot refactor the entire codebase"
# Output:
# Detected Keywords:
#   - "autopilot" -> mode: autopilot (pos: 0)
```

### Agent Listing
```bash
omcc agents
omcc agents --tier opus  # Filter by tier
```

### HUD Status
```bash
omcc status
omcc status --no-color
```

### Autopilot Pipeline
```bash
omcc autopilot "Build and test the user service"
```

## Architecture

```
src/
  main.rs         -- CLI entry point (clap-based)
  lib.rs          -- Module declarations
  config/mod.rs   -- YAML configuration (serde_yaml)
  state/mod.rs    -- File-based state persistence
  keyword/mod.rs  -- Magic keyword detection (regex)
  router/mod.rs   -- Smart model routing (scoring engine)
  decompose/mod.rs-- Task decomposition (7-step pipeline)
  agents/mod.rs   -- Agent registry (19 built-in agents)
  skills/mod.rs   -- Skill learning (SHA-256 dedup)
  autopilot/mod.rs-- Autopilot pipeline (state machine)
  hook/mod.rs     -- Hook bridge (JSON stdin/stdout router)
  hud/mod.rs      -- HUD statusline (ANSI rendering)
```

## Testing

```bash
cargo test
# 170 tests across 10 modules
```

Test coverage by module:
- `config`: 13 tests (YAML/JSON parsing, defaults, lookups)
- `state`: 11 tests (session lifecycle, persistence, TTL)
- `keyword`: 18 tests (detection, code blocks, CJK, custom keywords)
- `router`: 17 tests (scoring, thresholds, context, serialization)
- `decompose`: 17 tests (classification, scoping, sorting, validation)
- `agents`: 19 tests (registry, lookup, permissions, providers)
- `skills`: 14 tests (hashing, learning, promotion, import/export)
- `autopilot`: 16 tests (state machine, transitions, retries, abort)
- `hook`: 20 tests (bridge routing, JSON processing, handlers)
- `hud`: 19 tests (rendering, colors, truncation, progress bars)

## Dependencies

| Crate | Purpose |
|-------|---------|
| `serde` + `serde_json` | JSON serialization |
| `serde_yaml` | YAML configuration |
| `clap` | CLI argument parsing |
| `regex` | Keyword pattern matching |
| `sha2` | SHA-256 deduplication |
| `chrono` | Timestamps |
| `thiserror` | Error handling |

## Comparison with Original

| Feature | oh-my-claudecode (TS) | omcc-rs (Rust) |
|---------|----------------------|----------------|
| Hook bridge | 75KB bridge.ts | Type-safe enum routing |
| Keywords | Regex + runtime validation | Compile-time matching |
| Agents | 19 agents, runtime types | 19 agents, static types |
| Routing | Zod-validated scoring | serde-derived scoring |
| Decomposition | 23.5KB pipeline | 7-step typed pipeline |
| Autopilot | File-based state | Atomic writes + serde |
| Skills | SHA-256 + confidence | Same algorithm, zero-copy |
| HUD | ANSI rendering | Same rendering, no GC |
| State | JSON files | Atomic JSON writes |
| Team mode | tmux integration | Not implemented (v2) |
| Notifications | Telegram/Discord/Slack | Not implemented (v2) |
| Auto-update | npm-based | Not applicable |

## License

MIT License - Copyright (c) 2026 JSLEEKR
