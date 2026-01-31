# Main.rs Orchestration Extraction

## Problem

`main.rs` is a 600+ line god function handling:
- CLI parsing and validation
- Settings and scenario loading
- MCP server initialization
- TUI vs print mode branching
- Capture logging setup
- Failure injection
- Response matching and tool execution
- State persistence
- Output formatting

Changes to any concern risk breaking others. The function is untestable as a unit.

## Plan

1. **Extract `Runtime` struct** that owns the composed subsystems:
   ```rust
   pub struct Runtime {
       context: RuntimeContext,      // renamed from SessionContext
       scenario: Option<Scenario>,
       executor: Box<dyn ToolExecutor>,
       state: Option<StateWriter>,
       capture: Option<CaptureLog>,
   }
   ```

2. **Define `Runtime::execute(&mut self, prompt: &str) -> Result<Response>`** that encapsulates:
   - Scenario matching
   - Tool execution loop
   - State recording
   - Capture logging

3. **Extract `RuntimeBuilder`** for the initialization sequence:
   - `RuntimeBuilder::new(cli)` — parse and validate
   - `.with_scenario(path)` — load scenario
   - `.with_mcp(configs)` — initialize MCP servers
   - `.with_capture(path)` — setup capture logging
   - `.build() -> Result<Runtime>`

4. **Reduce main() to**:
   ```rust
   fn main() -> Result<()> {
       let cli = Cli::parse();
       let runtime = RuntimeBuilder::new(cli).build()?;

       if runtime.should_use_tui() {
           tui::run(runtime)
       } else {
           runtime.execute_print_mode()
       }
   }
   ```

5. **Add integration tests** that construct `Runtime` directly without going through main()
