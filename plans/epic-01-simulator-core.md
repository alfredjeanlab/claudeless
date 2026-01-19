# Epic 1: Claudeless Core

## Overview

Create a test crate that emulates the `claude` CLI for integration testing. The simulator provides a controllable test double that responds to the same CLI interface as real Claude, enabling deterministic integration testing without API costs or flakiness.

This epic focuses on the core simulation capabilities:
- **CLI interface** matching the flags that oj uses
- **Response scripting** via configuration files
- **Failure injection** for testing error handling paths
- **Output capture** for test assertions
- **Rust API** for programmatic test configuration

**What's NOT in this epic** (deferred to Epic 2):
- State directory emulation (`~/.claude/*`)
- Hook simulation
- Actual LLM responses (only canned/scripted responses)

## Project Structure

```
crates/
├── claudeless/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs                 # Binary entry point
│   │   ├── lib.rs                  # Library exports
│   │   ├── cli.rs                  # CLI argument parsing
│   │   ├── config.rs               # Scenario configuration types
│   │   ├── scenario.rs             # Scenario matching and loading
│   │   ├── output.rs               # Output format handling
│   │   ├── failure.rs              # Failure injection modes
│   │   ├── capture.rs              # Interaction capture/recording
│   │   └── api.rs                  # Rust test helper API
│   ├── tests/
│   └── scenarios/                  # Example scenario files
```

## Dependencies

```toml
[package]
name = "claudeless"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "claudeless"
path = "src/main.rs"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
thiserror = "2"
regex = "1"
glob = "0.3"
tokio = { version = "1", features = ["fs", "io-std", "time", "sync"] }
tempfile = "3"

[dev-dependencies]
proptest = "1"
yare = "3"
```

## Implementation Phases

### Phase 1: CLI Interface & Basic Scaffolding

**Goal**: Implement CLI parser accepting the same flags as real Claude.

**Deliverables**:
1. New `crates/cli` crate with binary
2. CLI argument parsing matching Claude's interface
3. Basic main loop for version/help

**Key Types**:

```rust
// cli.rs
#[derive(Parser, Debug)]
#[command(name = "claude", version, about = "Claude CLI Simulator")]
pub struct Cli {
    pub prompt: Option<String>,
    #[arg(short = 'p', long)]
    pub print: bool,
    #[arg(long, default_value = "claude-sonnet-4-20250514")]
    pub model: String,
    #[arg(long, value_enum, default_value = "text")]
    pub output_format: OutputFormat,
    #[arg(long)]
    pub max_tokens: Option<u32>,
    #[arg(long)]
    pub system_prompt: Option<String>,
    #[arg(long, short = 'c')]
    pub continue_conversation: bool,
    #[arg(long, short = 'r')]
    pub resume: Option<String>,
    #[arg(long = "allowedTools")]
    pub allowed_tools: Vec<String>,
    #[arg(long = "disallowedTools")]
    pub disallowed_tools: Vec<String>,
    #[arg(long = "permission-mode")]
    pub permission_mode: Option<String>,
    #[arg(long)]
    pub cwd: Option<String>,
    // Simulator-specific flags
    #[arg(long, env = "CLAUDELESS_SCENARIO")]
    pub scenario: Option<String>,
    #[arg(long, env = "CLAUDELESS_CAPTURE")]
    pub capture: Option<String>,
    #[arg(long, env = "CLAUDELESS_FAILURE")]
    pub failure: Option<FailureMode>,
    #[arg(long, env = "CLAUDELESS_DELAY_MS")]
    pub delay_ms: Option<u64>,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum OutputFormat { Text, Json, StreamJson }

#[derive(Clone, Debug, ValueEnum)]
pub enum FailureMode {
    NetworkUnreachable, ConnectionTimeout, AuthError,
    RateLimit, OutOfCredits, PartialResponse, MalformedJson,
}
```

---

### Phase 2: Scenario Configuration & Matching

**Goal**: Implement scenario configuration files with scripted responses based on prompt patterns.

**Deliverables**:
1. `ScenarioConfig` type for TOML/JSON scenario files
2. Prompt pattern matching (exact, regex, glob, contains)
3. Multi-turn conversation support
4. Default response for unmatched prompts

**Key Types**:

```rust
// config.rs
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ScenarioConfig {
    pub name: String,
    pub default_response: Option<ResponseSpec>,
    pub responses: Vec<ResponseRule>,
    pub conversations: HashMap<String, ConversationSpec>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResponseRule {
    pub pattern: PatternSpec,
    pub response: ResponseSpec,
    pub failure: Option<FailureSpec>,
    pub max_matches: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PatternSpec {
    Exact { text: String },
    Regex { pattern: String },
    Glob { pattern: String },
    Contains { text: String },
    Any,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ResponseSpec {
    Simple(String),
    Detailed {
        text: String,
        tool_calls: Vec<ToolCallSpec>,
        usage: Option<UsageSpec>,
        delay_ms: Option<u64>,
    },
}

// scenario.rs
pub struct Scenario {
    config: ScenarioConfig,
    compiled_patterns: Vec<CompiledRule>,
    match_counts: Vec<u32>,
}

impl Scenario {
    pub fn load(path: &Path) -> Result<Self, ScenarioError>;
    pub fn from_config(config: ScenarioConfig) -> Result<Self, ScenarioError>;
    pub fn match_prompt(&mut self, prompt: &str) -> Option<&ResponseRule>;
    // Compile PatternSpec → closure that tests prompts
}
```

**Example scenario** (`scenarios/simple.toml`):
```toml
name = "basic-responses"

[[responses]]
pattern = { type = "contains", text = "hello" }
response = "Hello! How can I help you today?"

[[responses]]
pattern = { type = "regex", pattern = "(?i)fix.*bug" }
response = { text = "I'll help you fix that bug.", delay_ms = 100 }

[default_response]
text = "I'm not sure how to help with that."
```

---

### Phase 3: Output Format Handling

**Goal**: Implement output formatting matching real Claude output.

**Deliverables**:
1. Text output mode (simple response text)
2. JSON output mode (structured response object)
3. Stream-JSON output mode (line-delimited JSON events)

**Key Types**:

```rust
// output.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonResponse {
    pub id: String,
    pub model: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: String,
    pub usage: Usage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    MessageStart { message: StreamMessage },
    ContentBlockStart { index: u32, content_block: ContentBlock },
    ContentBlockDelta { index: u32, delta: Delta },
    ContentBlockStop { index: u32 },
    MessageDelta { delta: MessageDelta, usage: Usage },
    MessageStop,
}

pub struct OutputWriter<W: Write> { /* writer, format, model */ }

impl<W: Write> OutputWriter<W> {
    pub fn new(writer: W, format: OutputFormat, model: String) -> Self;
    pub fn write_response(&mut self, response: &ResponseSpec, tool_calls: &[ToolCallSpec]) -> io::Result<()>;
    // Delegates to write_text, write_json, or write_stream_json based on format
}
```

---

### Phase 4: Failure Injection

**Goal**: Implement failure modes simulating various error conditions.

**Deliverables**:
1. Network unreachable, connection timeout
2. Authentication errors, rate limiting
3. Partial response (stream interruption)
4. Malformed JSON response

**Key Types**:

```rust
// failure.rs
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FailureSpec {
    NetworkUnreachable,
    ConnectionTimeout { after_ms: u64 },
    AuthError { message: String },
    RateLimit { retry_after: u64 },
    OutOfCredits,
    PartialResponse { partial_text: String },
    MalformedJson { raw: String },
}

pub struct FailureExecutor;

impl FailureExecutor {
    pub async fn execute<W: Write>(spec: &FailureSpec, writer: &mut W) -> Result<(), io::Error>;
    pub fn from_mode(mode: &FailureMode) -> FailureSpec;
    // Each variant writes appropriate error output and sets exit code
}
```

---

### Phase 5: Interaction Capture

**Goal**: Record all interactions for test assertions.

**Deliverables**:
1. `CaptureLog` for recording interactions
2. JSON capture file format (JSONL)
3. API for querying capture log

**Key Types**:

```rust
// capture.rs
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapturedInteraction {
    pub seq: u64,
    pub timestamp: SystemTime,
    pub elapsed: Duration,
    pub args: CapturedArgs,
    pub outcome: CapturedOutcome,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CapturedOutcome {
    Response { text: String, matched_rule: Option<String>, delay_ms: u64 },
    Failure { failure_type: String, message: String },
    NoMatch { used_default: bool },
}

pub struct CaptureLog {
    start: Instant,
    interactions: Arc<Mutex<Vec<CapturedInteraction>>>,
    file_writer: Option<Arc<Mutex<BufWriter<File>>>>,
}

impl CaptureLog {
    pub fn new() -> Self;
    pub fn with_file(path: &Path) -> io::Result<Self>;
    pub fn record(&self, args: CapturedArgs, outcome: CapturedOutcome);
    pub fn interactions(&self) -> Vec<CapturedInteraction>;
    pub fn find_by_prompt(&self, pattern: &str) -> Vec<CapturedInteraction>;
}
```

---

### Phase 6: Test Helper API

**Goal**: Implement Rust API for configuring and controlling the simulator in tests.

**Deliverables**:
1. `SimulatorBuilder` for fluent configuration
2. `SimulatorHandle` for in-process testing
3. `BinarySimulatorHandle` for integration tests

**Key Types**:

```rust
// api.rs
pub struct SimulatorBuilder {
    scenario: ScenarioConfig,
    capture: Option<PathBuf>,
    delay_ms: Option<u64>,
}

impl SimulatorBuilder {
    pub fn new() -> Self;
    pub fn scenario_file(self, path: impl AsRef<Path>) -> Self;
    pub fn respond_to(self, pattern: &str, response: &str) -> Self;
    pub fn respond_to_regex(self, pattern: &str, response: &str) -> Self;
    pub fn default_response(self, response: &str) -> Self;
    pub fn capture_to(self, path: impl Into<PathBuf>) -> Self;
    pub fn build_in_process(self) -> SimulatorHandle;
    pub fn build_binary(self) -> io::Result<BinarySimulatorHandle>;
}

pub enum SimulatorHandle {
    InProcess {
        scenario: Arc<Mutex<Scenario>>,
        capture: Arc<CaptureLog>,
        delay_ms: Option<u64>,
    },
}

impl SimulatorHandle {
    pub fn capture(&self) -> &CaptureLog;
    pub fn execute(&self, prompt: &str) -> String;
    pub fn assert_received(&self, pattern: &str);
    pub fn assert_count(&self, expected: usize);
}
```

**Usage Example**:
```rust
let sim = SimulatorBuilder::new()
    .respond_to("hello", "Hello! How can I help?")
    .respond_to("fix bug", "I'll fix that bug.")
    .default_response("I'm not sure what you mean.")
    .build_in_process();

let response = sim.execute("hello world");
assert_eq!(response, "Hello! How can I help?");
```

---

## Key Implementation Details

### PATH Shadowing

```rust
fn setup_test_path(sim_binary: &Path) -> String {
    let bin_dir = sim_binary.parent().unwrap();
    format!("{}:{}", bin_dir.display(), std::env::var("PATH").unwrap_or_default())
}
// Now subprocesses calling "claude" get claudeless
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (network, auth, rate limit) |
| 2 | Partial failure (stream interrupted) |

## Verification Plan

### Unit Tests

| Module | Key Tests |
|--------|-----------|
| `cli` | All flag combinations, defaults |
| `config` | TOML/JSON parsing, pattern types |
| `scenario` | Pattern matching, max_matches |
| `output` | Text/JSON/stream-JSON formats |
| `failure` | All failure modes, exit codes |
| `capture` | Recording, file output, queries |
| `api` | Builder, in-process mode |

### Test Commands

```bash
cargo test -p claudeless
cargo build -p claudeless
./target/debug/claudeless --scenario scenarios/simple.toml -p "hello"
CLAUDELESS_FAILURE=rate-limit ./target/debug/claudeless -p "test"
```
