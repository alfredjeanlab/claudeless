# CLI Struct Reorganization

## Problem

`Cli` struct has 30+ fields mixing different concerns:

- Claude compatibility flags (`--model`, `--output-format`, `--system-prompt`)
- Permission flags (`--permission-mode`, `--dangerously-skip-permissions`)
- Session flags (`--continue`, `--resume`, `--session-id`)
- Simulator-specific flags (`--scenario`, `--capture`, `--failure`, `--tool-mode`)
- MCP flags (`--mcp-config`, `--strict-mcp-config`, `--mcp-debug`)

Validation is scattered across `validate_*` methods.

## Plan

1. **Group into nested structs using clap's `#[command(flatten)]`**:

   ```rust
   #[derive(Parser)]
   pub struct Cli {
       #[arg(value_name = "PROMPT")]
       pub prompt: Option<String>,

       #[command(flatten)]
       pub output: OutputOptions,

       #[command(flatten)]
       pub session: SessionOptions,

       #[command(flatten)]
       pub permissions: PermissionOptions,

       #[command(flatten)]
       pub mcp: McpOptions,

       #[command(flatten)]
       pub simulator: SimulatorOptions,
   }
   ```

2. **Define focused option structs**:
   - `OutputOptions` — format, verbose, debug, include_partial_messages
   - `SessionOptions` — continue, resume, session_id, no_session_persistence
   - `PermissionOptions` — permission_mode, allow_dangerously_skip, dangerously_skip
   - `McpOptions` — mcp_config, strict_mcp_config, mcp_debug
   - `SimulatorOptions` — scenario, capture, failure, tool_mode, claude_version

3. **Move validation into option structs**:
   ```rust
   impl PermissionOptions {
       pub fn validate(&self) -> Result<(), &'static str> { ... }
   }
   ```

4. **Add `Cli::validate(&self) -> Result<()>`** that calls all sub-validations
