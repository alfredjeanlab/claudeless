# ScenarioConfig Organization

## Problem

`ScenarioConfig` has 15 optional fields spanning unrelated concerns:

```rust
pub struct ScenarioConfig {
    // Identity
    pub name: String,
    pub default_model: Option<String>,
    pub claude_version: Option<String>,
    pub user_name: Option<String>,
    pub session_id: Option<String>,

    // Responses
    pub default_response: Option<ResponseSpec>,
    pub responses: Vec<ResponseRule>,

    // Tools
    pub tool_execution: Option<ToolExecutionConfig>,

    // Environment
    pub project_path: Option<String>,
    pub working_directory: Option<String>,
    pub trusted: bool,
    pub permission_mode: Option<String>,

    // Timing
    pub launch_timestamp: Option<String>,
    pub timeouts: Option<TimeoutConfig>,
}
```

Tests must specify unrelated fields. Adding new fields bloats the struct further.

## Plan

1. **Split into focused config structs**:

   ```rust
   pub struct ScenarioConfig {
       pub name: String,

       #[serde(flatten)]
       pub responses: ResponseConfig,

       #[serde(flatten)]
       pub identity: IdentityConfig,

       #[serde(flatten)]
       pub environment: EnvironmentConfig,

       #[serde(flatten)]
       pub timing: TimingConfig,
   }
   ```

2. **Define sub-configs with defaults**:

   ```rust
   #[derive(Default)]
   pub struct IdentityConfig {
       pub default_model: Option<String>,
       pub claude_version: Option<String>,
       pub user_name: Option<String>,
       pub session_id: Option<String>,
   }

   #[derive(Default)]
   pub struct EnvironmentConfig {
       pub project_path: Option<String>,
       pub working_directory: Option<String>,
       #[serde(default = "default_trusted")]
       pub trusted: bool,
       pub permission_mode: Option<String>,
   }
   ```

3. **Use `#[serde(flatten)]`** to maintain TOML/JSON compatibility — existing scenario files continue to work

4. **Move validation into sub-configs**:
   ```rust
   impl IdentityConfig {
       pub fn validate(&self) -> Result<(), ScenarioError> {
           if let Some(ref id) = self.session_id {
               uuid::Uuid::parse_str(id).map_err(|_| ...)?;
           }
           Ok(())
       }
   }
   ```
