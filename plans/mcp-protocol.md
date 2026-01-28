# Implementation Plan: MCP Protocol Message Types

## Overview

Define MCP-specific protocol messages that layer on top of JSON-RPC. These types represent the wire format for MCP client-server communication, enabling tool discovery and execution through the MCP protocol.

This is Phase 2 of the larger MCP implementation (see `plans/mcp.md`).

## Project Structure

```
crates/cli/src/mcp/
├── mod.rs              # Update: add protocol module export
├── protocol.rs         # NEW: MCP protocol message types
└── protocol_tests.rs   # NEW: Serialization/deserialization tests
```

## Dependencies

No new dependencies required. Uses existing:
- `serde` - Serialization/deserialization
- `serde_json` - JSON value types

## Implementation Phases

### Phase 1: Core Protocol Types

**Goal:** Define the basic MCP message types for initialization.

**File:** `crates/cli/src/mcp/protocol.rs`

```rust
use serde::{Deserialize, Serialize};

/// MCP protocol version we support.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Client information sent during initialization.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            name: "claudeless".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}

/// Client capabilities sent during initialization.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    /// We don't support any optional capabilities yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<serde_json::Value>,
}

/// Initialize request parameters.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    pub client_info: ClientInfo,
}

impl Default for InitializeParams {
    fn default() -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION.into(),
            capabilities: ClientCapabilities::default(),
            client_info: ClientInfo::default(),
        }
    }
}
```

**Tasks:**
1. Create `protocol.rs` with module doc comment
2. Define `PROTOCOL_VERSION` constant
3. Implement `ClientInfo` with defaults
4. Implement `ClientCapabilities` (empty for now)
5. Implement `InitializeParams`

---

### Phase 2: Server Response Types

**Goal:** Define types for parsing server responses.

```rust
/// Server information from initialize response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
}

/// Server capabilities from initialize response.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    /// Whether server supports tools.
    #[serde(default)]
    pub tools: Option<ToolsCapability>,
}

/// Tools capability details.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapability {
    /// Whether tool list may change.
    #[serde(default)]
    pub list_changed: bool,
}

/// Initialize response result.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}
```

**Tasks:**
1. Define `ServerInfo`
2. Define `ServerCapabilities` with optional tool support
3. Define `ToolsCapability`
4. Define `InitializeResult`

---

### Phase 3: Tool Discovery Types

**Goal:** Define types for the `tools/list` method.

```rust
/// Tool information from MCP server.
///
/// Note: This parallels `McpToolDef` in config.rs but represents
/// the wire format. Conversion methods bridge the two.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    /// Unique tool name.
    pub name: String,

    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,

    /// JSON Schema for tool input.
    pub input_schema: serde_json::Value,
}

/// Response from tools/list method.
#[derive(Debug, Clone, Deserialize)]
pub struct ToolsListResult {
    pub tools: Vec<ToolInfo>,
}
```

**Conversion to existing types:**

```rust
use super::config::McpToolDef;

impl ToolInfo {
    /// Convert to McpToolDef for internal use.
    pub fn into_tool_def(self, server_name: &str) -> McpToolDef {
        McpToolDef {
            name: self.name,
            description: self.description.unwrap_or_default(),
            input_schema: self.input_schema,
            server_name: server_name.into(),
        }
    }
}
```

**Tasks:**
1. Define `ToolInfo` matching MCP spec
2. Define `ToolsListResult`
3. Implement conversion to `McpToolDef`

---

### Phase 4: Tool Execution Types

**Goal:** Define types for the `tools/call` method.

```rust
/// Parameters for tools/call method.
#[derive(Debug, Clone, Serialize)]
pub struct ToolCallParams {
    /// Tool name to invoke.
    pub name: String,

    /// Arguments matching the tool's input schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

/// Content block in tool response.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ContentBlock {
    /// Plain text content.
    Text { text: String },

    /// Base64-encoded image.
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },

    /// Resource reference.
    Resource {
        uri: String,
        #[serde(default)]
        text: Option<String>,
        #[serde(default, rename = "mimeType")]
        mime_type: Option<String>,
    },
}

/// Response from tools/call method.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResult {
    /// Content blocks returned by the tool.
    pub content: Vec<ContentBlock>,

    /// Whether the tool execution resulted in an error.
    #[serde(default)]
    pub is_error: bool,
}
```

**Conversion to existing types:**

```rust
use super::tools::McpToolResult;

impl ToolCallResult {
    /// Convert to McpToolResult for internal use.
    pub fn into_tool_result(self) -> McpToolResult {
        if self.is_error {
            let error_text = self.content.iter()
                .filter_map(|c| match c {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            McpToolResult::failure(error_text)
        } else {
            McpToolResult::success(serde_json::to_value(&self.content).unwrap_or_default())
        }
    }
}

impl ContentBlock {
    /// Extract text content if this is a text block.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            ContentBlock::Text { text } => Some(text),
            _ => None,
        }
    }
}
```

**Tasks:**
1. Define `ToolCallParams`
2. Define `ContentBlock` enum with all variants
3. Define `ToolCallResult`
4. Implement conversion to `McpToolResult`
5. Add helper methods on `ContentBlock`

---

### Phase 5: Module Integration

**Goal:** Export protocol types and add tests.

**File:** `crates/cli/src/mcp/mod.rs` (update)

```rust
pub mod protocol;

pub use protocol::{
    ClientCapabilities, ClientInfo, ContentBlock, InitializeParams,
    InitializeResult, ServerCapabilities, ServerInfo, ToolCallParams,
    ToolCallResult, ToolInfo, ToolsListResult, PROTOCOL_VERSION,
};
```

**Tasks:**
1. Add `pub mod protocol;` to mod.rs
2. Add re-exports for key types
3. Verify existing code still compiles

---

### Phase 6: Comprehensive Tests

**File:** `crates/cli/src/mcp/protocol_tests.rs`

```rust
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use serde_json::json;

#[test]
fn serialize_initialize_params() {
    let params = InitializeParams::default();
    let json = serde_json::to_value(&params).unwrap();

    assert_eq!(json["protocolVersion"], "2024-11-05");
    assert_eq!(json["clientInfo"]["name"], "claudeless");
    assert!(json["capabilities"].is_object());
}

#[test]
fn deserialize_initialize_result() {
    let json = json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": { "listChanged": true }
        },
        "serverInfo": {
            "name": "test-server",
            "version": "1.0.0"
        }
    });

    let result: InitializeResult = serde_json::from_value(json).unwrap();
    assert_eq!(result.server_info.name, "test-server");
    assert!(result.capabilities.tools.is_some());
}

#[test]
fn deserialize_tools_list() {
    let json = json!({
        "tools": [
            {
                "name": "read_file",
                "description": "Read file contents",
                "inputSchema": {
                    "type": "object",
                    "properties": { "path": { "type": "string" } }
                }
            }
        ]
    });

    let result: ToolsListResult = serde_json::from_value(json).unwrap();
    assert_eq!(result.tools.len(), 1);
    assert_eq!(result.tools[0].name, "read_file");
}

#[test]
fn deserialize_content_blocks() {
    // Text block
    let text_json = json!({"type": "text", "text": "hello"});
    let text: ContentBlock = serde_json::from_value(text_json).unwrap();
    assert_eq!(text.as_text(), Some("hello"));

    // Image block
    let img_json = json!({
        "type": "image",
        "data": "base64...",
        "mimeType": "image/png"
    });
    let img: ContentBlock = serde_json::from_value(img_json).unwrap();
    assert!(matches!(img, ContentBlock::Image { .. }));

    // Resource block
    let res_json = json!({
        "type": "resource",
        "uri": "file:///path",
        "text": "content"
    });
    let res: ContentBlock = serde_json::from_value(res_json).unwrap();
    assert!(matches!(res, ContentBlock::Resource { .. }));
}

#[test]
fn tool_call_result_conversion() {
    let result = ToolCallResult {
        content: vec![ContentBlock::Text { text: "output".into() }],
        is_error: false,
    };

    let mcp_result = result.into_tool_result();
    assert!(mcp_result.success);

    let error_result = ToolCallResult {
        content: vec![ContentBlock::Text { text: "error message".into() }],
        is_error: true,
    };

    let mcp_error = error_result.into_tool_result();
    assert!(!mcp_error.success);
    assert_eq!(mcp_error.error, Some("error message".into()));
}

#[test]
fn tool_info_to_def_conversion() {
    let info = ToolInfo {
        name: "test_tool".into(),
        description: Some("A test tool".into()),
        input_schema: json!({"type": "object"}),
    };

    let def = info.into_tool_def("my-server");
    assert_eq!(def.name, "test_tool");
    assert_eq!(def.server_name, "my-server");
}
```

**Tasks:**
1. Create `protocol_tests.rs`
2. Test serialization of request types
3. Test deserialization of response types
4. Test all `ContentBlock` variants
5. Test conversion methods

## Key Implementation Details

### Serde Naming Conventions

MCP uses camelCase in JSON:
- Use `#[serde(rename_all = "camelCase")]` on structs
- Use `#[serde(rename = "mimeType")]` for specific fields
- Use `#[serde(tag = "type")]` for tagged enums

### Optional Fields

Handle missing fields gracefully:
- Use `#[serde(default)]` for optional fields with defaults
- Use `Option<T>` for truly optional fields
- Use `#[serde(skip_serializing_if = "Option::is_none")]` to omit None

### Type Bridging

The protocol types are wire-format focused. Existing types in `config.rs` and `tools.rs` are internal. Conversion methods bridge between them:

```
Wire Format (protocol.rs)    Internal (config.rs/tools.rs)
─────────────────────────    ────────────────────────────
ToolInfo          ──►        McpToolDef
ToolCallResult    ──►        McpToolResult
```

### Error Content

When `is_error: true`, the content typically contains text blocks with error details. The conversion extracts and concatenates these.

## Verification Plan

### Unit Tests

Run protocol tests:
```bash
cargo test -p claudeless mcp::protocol
```

Expected coverage:
- All request type serialization
- All response type deserialization
- All `ContentBlock` variants
- Conversion methods produce correct results

### Integration Check

Verify the new module integrates with existing code:
```bash
cargo check -p claudeless
cargo clippy -p claudeless --all-features
```

### Full Validation

Run the complete check suite:
```bash
make check
```

This ensures:
- All tests pass
- No clippy warnings
- Code is properly formatted
- Crate packages correctly
