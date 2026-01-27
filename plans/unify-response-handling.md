# Plan: Unify Response Handling

## Problem

Response text extraction from `ResponseSpec` is repeated throughout the codebase:

```rust
match scenario.get_response(&result) {
    Some(crate::config::ResponseSpec::Simple(text)) => text.clone(),
    Some(crate::config::ResponseSpec::Detailed { text, .. }) => text.clone(),
    None => String::new(),
}
```

Found in:
- `tui/app/commands.rs:91-107` (shell command handling)
- `tui/app/commands.rs:169-184` (prompt processing)
- `main.rs:152-155` (capture outcome)
- `main.rs:198-200` (response text extraction)

`ResponseSpec::text_and_usage()` exists but returns a tuple and isn't used consistently.

## Files to Modify

- `crates/cli/src/config.rs` - Add helper methods to ResponseSpec
- `crates/cli/src/tui/app/commands.rs` - Use new helpers
- `crates/cli/src/main.rs` - Use new helpers

## Implementation

### Step 1: Enhance ResponseSpec methods

In `config.rs`, add to `ResponseSpec`:

```rust
impl ResponseSpec {
    /// Extract just the text content.
    pub fn text(&self) -> &str {
        match self {
            ResponseSpec::Simple(s) => s,
            ResponseSpec::Detailed { text, .. } => text,
        }
    }

    /// Extract text content as owned String.
    pub fn into_text(self) -> String {
        match self {
            ResponseSpec::Simple(s) => s,
            ResponseSpec::Detailed { text, .. } => text,
        }
    }

    /// Get tool calls if any.
    pub fn tool_calls(&self) -> &[ToolCallSpec] {
        match self {
            ResponseSpec::Simple(_) => &[],
            ResponseSpec::Detailed { tool_calls, .. } => tool_calls,
        }
    }

    /// Get delay if specified.
    pub fn delay_ms(&self) -> Option<u64> {
        match self {
            ResponseSpec::Simple(_) => None,
            ResponseSpec::Detailed { delay_ms, .. } => *delay_ms,
        }
    }

    /// Existing method - keep for compatibility
    pub fn text_and_usage(&self) -> (String, Option<UsageSpec>) {
        match self {
            ResponseSpec::Simple(s) => (s.clone(), None),
            ResponseSpec::Detailed { text, usage, .. } => (text.clone(), usage.clone()),
        }
    }
}
```

### Step 2: Add Scenario helper

In `scenario.rs`, add:

```rust
impl Scenario {
    /// Get response text for a match result, or empty string if none.
    pub fn response_text(&self, result: &MatchResult) -> String {
        self.get_response(result)
            .map(|r| r.text().to_string())
            .unwrap_or_default()
    }

    /// Get response text, falling back to default response.
    pub fn response_text_or_default(&mut self, prompt: &str) -> String {
        if let Some(result) = self.match_prompt(prompt) {
            self.response_text(&result)
        } else if let Some(default) = self.default_response() {
            default.text().to_string()
        } else {
            String::new()
        }
    }
}
```

### Step 3: Update commands.rs

```rust
// Before (lines 91-107)
let response_text = {
    let mut scenario = inner.scenario.lock();
    if let Some(result) = scenario.match_prompt(&command) {
        match scenario.get_response(&result) {
            Some(crate::config::ResponseSpec::Simple(text)) => text.clone(),
            Some(crate::config::ResponseSpec::Detailed { text, .. }) => text.clone(),
            None => String::new(),
        }
    } else if let Some(default) = scenario.default_response() {
        match default {
            crate::config::ResponseSpec::Simple(text) => text.clone(),
            crate::config::ResponseSpec::Detailed { text, .. } => text.clone(),
        }
    } else {
        format!("$ {}", command)
    }
};

// After
let response_text = {
    let mut scenario = inner.scenario.lock();
    let text = scenario.response_text_or_default(&command);
    if text.is_empty() {
        format!("$ {}", command)
    } else {
        text
    }
};
```

### Step 4: Update main.rs

```rust
// Before (lines 152-155)
let outcome = match &response {
    Some(spec) => CapturedOutcome::Response {
        text: match spec {
            ResponseSpec::Simple(s) => s.clone(),
            ResponseSpec::Detailed { text, .. } => text.clone(),
        },
        ...
    },
    ...
};

// After
let outcome = match &response {
    Some(spec) => CapturedOutcome::Response {
        text: spec.text().to_string(),
        ...
    },
    ...
};

// Before (lines 198-200)
let response_text = match &response {
    ResponseSpec::Simple(s) => s.clone(),
    ResponseSpec::Detailed { text, .. } => text.clone(),
};

// After
let response_text = response.text().to_string();
```

## Testing

- Existing tests should pass unchanged
- Add unit tests for new `ResponseSpec` methods

## Lines Changed

- ~20 lines added to config.rs (new methods)
- ~10 lines added to scenario.rs (helper method)
- ~40 lines removed (duplicate match blocks)
- Net: ~10 lines reduced
