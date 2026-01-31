# StateWriter Simplification

## Problem

`StateWriter` has 8+ methods with repetitive boilerplate:

```rust
pub fn record_user_message(&mut self, prompt: &str) -> io::Result<String> {
    let project_dir = self.project_dir();
    std::fs::create_dir_all(&project_dir)?;
    let jsonl_path = self.session_jsonl_path();
    let user_uuid = Uuid::new_v4().to_string();
    let git_branch = get_git_branch();
    // ... 20 more lines
}

pub fn record_assistant_response(&mut self, ...) -> io::Result<String> {
    let jsonl_path = self.session_jsonl_path();
    let assistant_uuid = Uuid::new_v4().to_string();
    let request_id = format!("req_{}", Uuid::new_v4().simple());
    let message_id = format!("msg_{}", Uuid::new_v4().simple());
    let git_branch = get_git_branch();
    // ... 20 more lines
}
```

Each method: generates UUIDs, resolves paths, appends JSONL, updates index.

## Plan

1. **Extract `MessageIds` struct** for UUID generation:
   ```rust
   struct MessageIds {
       uuid: String,
       request_id: String,
       message_id: String,
   }

   impl MessageIds {
       fn new() -> Self { ... }
       fn user() -> Self { Self { uuid: new_uuid(), ..Default::default() } }
       fn assistant() -> Self { Self::new() }
   }
   ```

2. **Extract `WriteContext`** for common per-write state:
   ```rust
   struct WriteContext<'a> {
       jsonl_path: PathBuf,
       git_branch: String,
       timestamp: DateTime<Utc>,
       session_id: &'a str,
       cwd: &'a str,
       version: &'static str,
   }
   ```

3. **Simplify record methods** to use shared context:
   ```rust
   pub fn record_user_message(&mut self, prompt: &str) -> io::Result<String> {
       let ctx = self.write_context()?;
       let ids = MessageIds::user();
       append_user_message(&ctx, &ids, prompt)?;
       self.on_message_written();
       Ok(ids.uuid)
   }
   ```

4. **Consider a single `record()` method** with enum:
   ```rust
   pub fn record(&mut self, message: Message) -> io::Result<String> { ... }

   enum Message<'a> {
       User { prompt: &'a str },
       Assistant { parent: &'a str, content: Vec<ContentBlock> },
       ToolResult { tool_use_id: &'a str, ... },
   }
   ```
