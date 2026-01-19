# Refactor: Merge Conversations into Response Turns

**Root Feature:** `cl-d735`

## Overview

Simplify the scenario format by removing the separate `conversations` map and instead allowing response rules to have optional follow-up `turns`. This provides multi-turn conversation support with a simpler, more intuitive API.

**Current approach** (two separate concepts):
```toml
[[responses]]
pattern = { type = "contains", text = "hello" }
response = "Hi!"

[conversations.login-flow]
turns = [
    { expect = { type = "contains", text = "login" }, response = "Username:" },
    { expect = { type = "any" }, response = "Password:" },
]
```

**New approach** (unified):
```toml
[[responses]]
pattern = { type = "contains", text = "login" }
response = "Username:"
turns = [
    { expect = { type = "any" }, response = "Password:" },
    { expect = { type = "any" }, response = "Login successful!" }
]
```

**Benefits**:
- Single concept: responses, some with follow-up turns
- Entry pattern is just `pattern` (no separate first-turn logic)
- Scoped state: each response rule tracks its own turn index
- Simpler config: no `[conversations.name]` indirection
- Easier to understand matching flow

**What's NOT in this refactor**:
- Named conversation references (can add later if needed)
- Branching/conditional turns (keep it simple)
- Parallel conversation tracking (one active sequence at a time)

---

## Project Structure

```
crates/cli/
├── src/
│   ├── config.rs              # UPDATE: Add turns to ResponseRule, remove conversations
│   ├── scenario.rs            # UPDATE: Add turn state tracking and matching
│   ├── scenario_tests.rs      # UPDATE: Add turn matching tests
│   ├── api.rs                 # UPDATE: Handle turns in execute()
│   └── api_tests.rs           # UPDATE: Add turn tests
├── scenarios/
│   ├── multi_turn.toml        # UPDATE: Convert to new format
│   └── simple.toml            # NO CHANGES (no turns)
└── tests/
    ├── scenario_turns.rs      # NEW: Integration tests for turn behavior
    └── scenario_fields.rs     # UPDATE: Remove conversation tests, add turn tests
docs/
└── SCENARIOS.md               # UPDATE: Simplify documentation
```

---

## Phase 1: Update Data Structures

**Goal**: Add `turns` field to `ResponseRule`, deprecate `conversations`.

### Changes to `src/config.rs`

```rust
/// A single response rule
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseRule {
    /// Pattern to match against prompt (entry pattern for turn sequences)
    pub pattern: PatternSpec,

    /// Response to return when pattern matches
    pub response: ResponseSpec,

    /// Optional failure to inject instead of responding
    #[serde(default)]
    pub failure: Option<FailureSpec>,

    /// How many times this rule can match (None = unlimited)
    #[serde(default)]
    pub max_matches: Option<u32>,

    /// Optional follow-up turns after initial match
    /// When present, subsequent prompts match against turns in sequence
    #[serde(default)]
    pub turns: Vec<ConversationTurn>,  // NEW
}

/// A single turn in a multi-turn sequence
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConversationTurn {
    /// Expected prompt pattern for this turn
    pub expect: PatternSpec,
    /// Response for this turn
    pub response: ResponseSpec,
    /// Optional failure for this turn
    #[serde(default)]
    pub failure: Option<FailureSpec>,
}
```

### Remove from `ScenarioConfig`

```rust
pub struct ScenarioConfig {
    // ... existing fields ...

    // REMOVE this field:
    // pub conversations: HashMap<String, ConversationSpec>,
}
```

### Remove `ConversationSpec`

The `ConversationSpec` struct is no longer needed since turns are inline.

**Verification**:
- `cargo build -p claudeless` compiles
- Existing scenarios without turns still parse
- New format with inline turns parses

---

## Phase 2: Add Turn State to Scenario

**Goal**: Track which response rule (if any) has an active turn sequence.

### Changes to `src/scenario.rs`

```rust
/// Compiled scenario ready for matching
#[derive(Debug)]
pub struct Scenario {
    config: ScenarioConfig,
    compiled_patterns: Vec<CompiledRule>,
    match_counts: Vec<u32>,

    // NEW: Turn sequence state
    /// Index of the response rule with an active turn sequence (None = no active sequence)
    active_rule: Option<usize>,
    /// Current turn index within the active rule's turns (0-indexed)
    current_turn: usize,
    /// Compiled matchers for turns (lazily populated)
    compiled_turns: Vec<Vec<Matcher>>,
}

impl Scenario {
    pub fn from_config(config: ScenarioConfig) -> Result<Self, ScenarioError> {
        // ... existing validation ...

        // Compile response patterns
        let mut compiled = Vec::new();
        let mut compiled_turns = Vec::new();

        for (idx, rule) in config.responses.iter().enumerate() {
            let matcher = compile_pattern(&rule.pattern)?;
            compiled.push(CompiledRule {
                matcher,
                rule_index: idx,
            });

            // Compile turn patterns for this rule
            let mut turn_matchers = Vec::new();
            for turn in &rule.turns {
                turn_matchers.push(compile_pattern(&turn.expect)?);
            }
            compiled_turns.push(turn_matchers);
        }

        let match_counts = vec![0; config.responses.len()];

        Ok(Self {
            config,
            compiled_patterns: compiled,
            match_counts,
            active_rule: None,
            current_turn: 0,
            compiled_turns,
        })
    }

    /// Find matching response for a prompt
    pub fn match_prompt(&mut self, prompt: &str) -> Option<MatchResult> {
        // If we have an active turn sequence, try to match the current turn
        if let Some(rule_idx) = self.active_rule {
            let turn_idx = self.current_turn;
            let rule = &self.config.responses[rule_idx];

            if turn_idx < rule.turns.len() {
                let matcher = &self.compiled_turns[rule_idx][turn_idx];
                if matcher(prompt) {
                    self.current_turn += 1;

                    // Deactivate if we've completed all turns
                    if self.current_turn >= rule.turns.len() {
                        self.active_rule = None;
                        self.current_turn = 0;
                    }

                    return Some(MatchResult::Turn {
                        rule_index: rule_idx,
                        turn_index: turn_idx,
                    });
                }
            }

            // Turn didn't match - deactivate sequence and fall through to normal matching
            self.active_rule = None;
            self.current_turn = 0;
        }

        // Normal response matching
        for compiled in &self.compiled_patterns {
            let rule = &self.config.responses[compiled.rule_index];

            // Check max_matches limit
            if let Some(max) = rule.max_matches {
                if self.match_counts[compiled.rule_index] >= max {
                    continue;
                }
            }

            if (compiled.matcher)(prompt) {
                self.match_counts[compiled.rule_index] += 1;

                // If this rule has turns, activate the sequence
                if !rule.turns.is_empty() {
                    self.active_rule = Some(compiled.rule_index);
                    self.current_turn = 0;
                }

                return Some(MatchResult::Response {
                    rule_index: compiled.rule_index,
                });
            }
        }

        None
    }

    /// Get response for a match result
    pub fn get_response(&self, result: &MatchResult) -> &ResponseSpec {
        match result {
            MatchResult::Response { rule_index } => {
                &self.config.responses[*rule_index].response
            }
            MatchResult::Turn { rule_index, turn_index } => {
                &self.config.responses[*rule_index].turns[*turn_index].response
            }
        }
    }

    /// Get failure for a match result (if any)
    pub fn get_failure(&self, result: &MatchResult) -> Option<&FailureSpec> {
        match result {
            MatchResult::Response { rule_index } => {
                self.config.responses[*rule_index].failure.as_ref()
            }
            MatchResult::Turn { rule_index, turn_index } => {
                self.config.responses[*rule_index].turns[*turn_index].failure.as_ref()
            }
        }
    }

    /// Check if a turn sequence is active
    pub fn has_active_sequence(&self) -> bool {
        self.active_rule.is_some()
    }

    /// Reset turn state (useful for tests)
    pub fn reset_turns(&mut self) {
        self.active_rule = None;
        self.current_turn = 0;
    }

    /// Reset all state including match counts and turns
    pub fn reset_counts(&mut self) {
        for count in &mut self.match_counts {
            *count = 0;
        }
        self.reset_turns();
    }
}

/// Result of matching a prompt
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatchResult {
    /// Matched a top-level response rule
    Response { rule_index: usize },
    /// Matched a turn within an active sequence
    Turn { rule_index: usize, turn_index: usize },
}
```

**Verification**:
- `cargo test -p claudeless scenario` passes
- Turn sequences activate and advance correctly
- Turn mismatch deactivates sequence and falls through

---

## Phase 3: Update API Layer

**Goal**: Update `SimulatorHandle::execute()` to use new matching.

### Changes to `src/api.rs`

```rust
impl SimulatorHandle {
    pub fn execute_with_args(&self, prompt: &str, model: Option<&str>) -> String {
        match self {
            Self::InProcess {
                scenario, capture, ..
            } => {
                let mut s = scenario.lock();

                let args = CapturedArgs {
                    prompt: Some(prompt.to_string()),
                    model: model.unwrap_or("claude-test").to_string(),
                    // ... other fields ...
                };

                let (text, matched_rule) = if let Some(result) = s.match_prompt(prompt) {
                    // Check for failure first
                    if let Some(_failure) = s.get_failure(&result) {
                        // TODO: Handle failure injection
                        (String::new(), Some("failure".to_string()))
                    } else {
                        let response = s.get_response(&result);
                        let text = match response {
                            ResponseSpec::Simple(text) => text.clone(),
                            ResponseSpec::Detailed { text, .. } => text.clone(),
                        };
                        let matched = match result {
                            MatchResult::Response { rule_index } => {
                                format!("response[{}]", rule_index)
                            }
                            MatchResult::Turn { rule_index, turn_index } => {
                                format!("response[{}].turn[{}]", rule_index, turn_index)
                            }
                        };
                        (text, Some(matched))
                    }
                } else if let Some(default) = s.default_response() {
                    let text = match default {
                        ResponseSpec::Simple(text) => text.clone(),
                        ResponseSpec::Detailed { text, .. } => text.clone(),
                    };
                    (text, Some("default".to_string()))
                } else {
                    (String::new(), None)
                };

                capture.record(
                    args,
                    CapturedOutcome::Response {
                        text: text.clone(),
                        matched_rule,
                        delay_ms: 0,
                    },
                );

                text
            }
        }
    }
}
```

**Verification**:
- `cargo test -p claudeless api` passes
- Multi-turn sequences work through API

---

## Phase 4: Update Scenario Files

**Goal**: Convert existing `multi_turn.toml` to new format, remove `conversations` usage.

### Update `scenarios/multi_turn.toml`

**Before**:
```toml
name = "multi-turn"

[conversations.login-flow]
turns = [
    { expect = { type = "contains", text = "login" }, response = "Please enter your username:" },
    { expect = { type = "any" }, response = "Please enter your password:" },
    { expect = { type = "any" }, response = "Login successful! Welcome." }
]

[conversations.code-review]
turns = [
    { expect = { type = "contains", text = "review" }, response = "I'll review your code." },
    { expect = { type = "any" }, response = "I found a few issues..." },
    { expect = { type = "contains", text = "fix" }, response = "Here's the corrected code." }
]

[[responses]]
pattern = { type = "any" }
response = "Please start a conversation by asking me to login or review code."
```

**After**:
```toml
name = "multi-turn"

# Login flow - entry pattern triggers the sequence
[[responses]]
pattern = { type = "contains", text = "login" }
response = "Please enter your username:"
turns = [
    { expect = { type = "any" }, response = "Please enter your password:" },
    { expect = { type = "any" }, response = "Login successful! Welcome." }
]

# Code review flow
[[responses]]
pattern = { type = "contains", text = "review" }
response = "I'll review your code. Please share it."
turns = [
    { expect = { type = "any" }, response = "I found a few issues. Let me explain..." },
    { expect = { type = "contains", text = "fix" }, response = "Here's the corrected code." }
]

# Fallback for non-matching prompts
[[responses]]
pattern = { type = "any" }
response = "Please start by asking me to login or review code."
```

**Verification**:
- `cargo run -p claudeless -- --scenario scenarios/multi_turn.toml -p "login"` returns username prompt
- Sequential prompts advance through turns

---

## Phase 5: Integration Tests

**Goal**: Comprehensive tests for turn behavior.

### New test file: `tests/scenario_turns.rs`

```rust
//! Integration tests for response turn sequences

use claudeless::{PatternSpec, ResponseRule, ResponseSpec, ScenarioConfig};
use claudeless::scenario::Scenario;

#[test]
fn turn_sequence_advances() {
    let config = ScenarioConfig {
        responses: vec![ResponseRule {
            pattern: PatternSpec::Contains { text: "start".into() },
            response: ResponseSpec::Simple("Step 1".into()),
            failure: None,
            max_matches: None,
            turns: vec![
                ConversationTurn {
                    expect: PatternSpec::Any,
                    response: ResponseSpec::Simple("Step 2".into()),
                    failure: None,
                },
                ConversationTurn {
                    expect: PatternSpec::Any,
                    response: ResponseSpec::Simple("Step 3".into()),
                    failure: None,
                },
            ],
        }],
        ..Default::default()
    };

    let mut scenario = Scenario::from_config(config).unwrap();

    // First prompt activates sequence
    let r1 = scenario.match_prompt("start").unwrap();
    assert_eq!(scenario.get_response(&r1).text(), "Step 1");
    assert!(scenario.has_active_sequence());

    // Second prompt advances to turn 0
    let r2 = scenario.match_prompt("anything").unwrap();
    assert_eq!(scenario.get_response(&r2).text(), "Step 2");
    assert!(scenario.has_active_sequence());

    // Third prompt advances to turn 1 and completes
    let r3 = scenario.match_prompt("anything").unwrap();
    assert_eq!(scenario.get_response(&r3).text(), "Step 3");
    assert!(!scenario.has_active_sequence());
}

#[test]
fn turn_mismatch_deactivates_and_falls_through() {
    let config = ScenarioConfig {
        responses: vec![
            ResponseRule {
                pattern: PatternSpec::Contains { text: "start".into() },
                response: ResponseSpec::Simple("Started".into()),
                turns: vec![ConversationTurn {
                    expect: PatternSpec::Contains { text: "continue".into() },
                    response: ResponseSpec::Simple("Continued".into()),
                    failure: None,
                }],
                ..Default::default()
            },
            ResponseRule {
                pattern: PatternSpec::Any,
                response: ResponseSpec::Simple("Fallback".into()),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let mut scenario = Scenario::from_config(config).unwrap();

    // Activate sequence
    scenario.match_prompt("start");
    assert!(scenario.has_active_sequence());

    // Mismatch - should deactivate and fall through to "any" rule
    let result = scenario.match_prompt("wrong input").unwrap();
    assert!(!scenario.has_active_sequence());
    assert_eq!(scenario.get_response(&result).text(), "Fallback");
}

#[test]
fn turns_with_failures() {
    let config = ScenarioConfig {
        responses: vec![ResponseRule {
            pattern: PatternSpec::Contains { text: "start".into() },
            response: ResponseSpec::Simple("Started".into()),
            turns: vec![ConversationTurn {
                expect: PatternSpec::Any,
                response: ResponseSpec::Simple("".into()),
                failure: Some(FailureSpec::AuthError {
                    message: "Session expired".into(),
                }),
            }],
            ..Default::default()
        }],
        ..Default::default()
    };

    let mut scenario = Scenario::from_config(config).unwrap();

    scenario.match_prompt("start");
    let result = scenario.match_prompt("next").unwrap();
    assert!(scenario.get_failure(&result).is_some());
}

#[test]
fn max_matches_applies_to_sequence_entry() {
    let config = ScenarioConfig {
        responses: vec![ResponseRule {
            pattern: PatternSpec::Contains { text: "start".into() },
            response: ResponseSpec::Simple("Started".into()),
            max_matches: Some(1),
            turns: vec![ConversationTurn {
                expect: PatternSpec::Any,
                response: ResponseSpec::Simple("Turn 1".into()),
                failure: None,
            }],
            ..Default::default()
        }],
        ..Default::default()
    };

    let mut scenario = Scenario::from_config(config).unwrap();

    // First entry works
    assert!(scenario.match_prompt("start").is_some());
    scenario.match_prompt("next"); // Complete sequence

    // Second entry blocked by max_matches
    assert!(scenario.match_prompt("start").is_none());
}
```

**Verification**:
- `cargo test -p claudeless --test scenario_turns` passes

---

## Phase 6: Update Documentation

**Goal**: Simplify `docs/SCENARIOS.md` to reflect the new unified model.

### Key Documentation Changes

1. **Remove "Multi-Turn Conversations" section** - replace with simpler "Turn Sequences" section

2. **Update Response Specifications** - add `turns` field documentation

3. **Simplify examples** - show inline turns instead of separate conversations

### Updated Section: Turn Sequences

```markdown
## Turn Sequences

Response rules can have follow-up `turns` for multi-step interactions.

### Basic Turn Sequence

```toml
[[responses]]
pattern = { type = "contains", text = "login" }
response = "Enter username:"
turns = [
    { expect = { type = "any" }, response = "Enter password:" },
    { expect = { type = "any" }, response = "Login successful!" }
]
```

### How Turn Sequences Work

1. When `pattern` matches, return `response` and activate the turn sequence
2. Subsequent prompts match against the current turn's `expect` pattern
3. If turn matches, return its `response` and advance to next turn
4. When all turns complete, sequence deactivates
5. If a turn doesn't match, sequence deactivates and normal matching resumes

### Turn Fields

| Field | Type | Description |
|-------|------|-------------|
| `expect` | pattern | Pattern to match for this turn |
| `response` | string/object | Response for this turn |
| `failure` | object | Optional failure for this turn |

### Turns with Failures

```toml
[[responses]]
pattern = { type = "contains", text = "auth" }
response = "Authenticating..."
turns = [
    { expect = { type = "any" }, response = "", failure = { type = "auth_error", message = "Invalid token" } }
]
```
```

### Items to Remove from Documentation

- `[conversations.name]` section
- `ConversationSpec` references
- Separate multi-turn conversation examples

**Verification**:
- Documentation accurately reflects new format
- All examples work with updated scenario parser

---

## Migration Guide

For users with existing `conversations` in their scenarios:

### Before (Old Format)
```toml
[conversations.my-flow]
turns = [
    { expect = { type = "contains", text = "start" }, response = "First" },
    { expect = { type = "any" }, response = "Second" },
]
```

### After (New Format)
```toml
[[responses]]
pattern = { type = "contains", text = "start" }  # First turn's expect becomes pattern
response = "First"                                # First turn's response
turns = [
    { expect = { type = "any" }, response = "Second" }  # Remaining turns
]
```

**Key difference**: The first turn's `expect` becomes the response rule's `pattern`, and the first turn's `response` becomes the rule's `response`. Remaining turns go in `turns` array.

---

## Verification Checklist

### Unit Tests
- [ ] `config.rs` - `turns` field parses correctly
- [ ] `scenario.rs` - Turn state tracking works
- [ ] `scenario.rs` - Turn matching advances correctly
- [ ] `scenario.rs` - Turn mismatch falls through
- [ ] `api.rs` - Execute handles turns

### Integration Tests
- [ ] `scenario_turns.rs` - Full sequence test
- [ ] `scenario_turns.rs` - Mismatch deactivation test
- [ ] `scenario_turns.rs` - Turns with failures test
- [ ] `scenario_turns.rs` - max_matches with turns test

### Scenario Files
- [ ] `multi_turn.toml` converted to new format
- [ ] `multi_turn.toml` loads without errors
- [ ] Other scenarios unaffected

### Documentation
- [ ] `docs/SCENARIOS.md` updated
- [ ] Old `conversations` section removed
- [ ] New `turns` section added
- [ ] Examples updated

### Commands
```bash
# Run all tests
cargo test -p claudeless

# Run turn-specific tests
cargo test -p claudeless turn

# Test multi_turn scenario
cargo run -p claudeless -- --scenario scenarios/multi_turn.toml -p "login"
# Expected: "Please enter your username:"

# Full CI check
make check
```
