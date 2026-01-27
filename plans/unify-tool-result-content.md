# Plan: Unify Tool Result Content Types

## Problem

Two nearly identical types for tool result content:

| Type | Location | Variants |
|------|----------|----------|
| `ToolResultContent` | `tools/result.rs:99` | `Text { text }`, `Image { data, media_type }` |
| `ToolResultContentBlock` | `output.rs:321` | `Text { text }`, `Image { data, media_type }` |

Both are used for tool execution results but defined separately.

Additionally, `ToolResultBlock::from_result()` manually converts between them with a verbose match.

## Files to Modify

- `crates/cli/src/tools/result.rs` - Keep as canonical type
- `crates/cli/src/output.rs` - Use type alias and add conversion
- `crates/cli/src/lib.rs` - Ensure proper exports

## Implementation

### Step 1: Keep ToolResultContent as canonical

In `tools/result.rs` (already exists):
```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolResultContent {
    Text { text: String },
    Image { data: String, media_type: String },
}
```

### Step 2: Create type alias in output.rs

```rust
// Before
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolResultContentBlock {
    Text { text: String },
    Image { data: String, media_type: String },
}

// After
pub use crate::tools::ToolResultContent as ToolResultContentBlock;
```

### Step 3: Simplify ToolResultBlock::from_result

```rust
// Before
impl ToolResultBlock {
    pub fn from_result(result: &crate::tools::ToolExecutionResult) -> Self {
        Self {
            block_type: "tool_result".to_string(),
            tool_use_id: result.tool_use_id.clone(),
            is_error: result.is_error,
            content: result
                .content
                .iter()
                .map(|c| match c {
                    crate::tools::ToolResultContent::Text { text } => {
                        ToolResultContentBlock::Text { text: text.clone() }
                    }
                    crate::tools::ToolResultContent::Image { data, media_type } => {
                        ToolResultContentBlock::Image {
                            data: data.clone(),
                            media_type: media_type.clone(),
                        }
                    }
                })
                .collect(),
        }
    }
}

// After
impl ToolResultBlock {
    pub fn from_result(result: &crate::tools::ToolExecutionResult) -> Self {
        Self {
            block_type: "tool_result".to_string(),
            tool_use_id: result.tool_use_id.clone(),
            is_error: result.is_error,
            content: result.content.clone(),
        }
    }
}
```

### Step 4: Verify serde compatibility

Both types use the same serde attributes:
- `#[serde(tag = "type", rename_all = "snake_case")]`

This ensures JSON serialization remains compatible.

### Step 5: Update any imports

Search for uses of `ToolResultContentBlock`:
```rust
// If any code imports it directly, update to:
use crate::output::ToolResultContentBlock;
// or
use crate::tools::ToolResultContent;
```

## Alternative: Keep Both but Add From impl

If breaking the API is a concern, add conversion instead:

```rust
impl From<ToolResultContent> for ToolResultContentBlock {
    fn from(content: ToolResultContent) -> Self {
        match content {
            ToolResultContent::Text { text } => Self::Text { text },
            ToolResultContent::Image { data, media_type } => Self::Image { data, media_type },
        }
    }
}

// Then simplify from_result:
impl ToolResultBlock {
    pub fn from_result(result: &ToolExecutionResult) -> Self {
        Self {
            block_type: "tool_result".to_string(),
            tool_use_id: result.tool_use_id.clone(),
            is_error: result.is_error,
            content: result.content.iter().cloned().map(Into::into).collect(),
        }
    }
}
```

## Testing

- Existing serialization tests must pass
- Tool execution results should serialize identically

## Lines Changed

- ~10 lines removed (duplicate enum definition)
- ~10 lines removed (verbose match in from_result)
- ~2 lines added (type alias or From impl)
- Net: ~18 lines reduced
