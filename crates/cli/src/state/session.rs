// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Session/conversation state tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A tool call within a turn
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TurnToolCall {
    pub tool: String,
    pub input: serde_json::Value,
    pub output: Option<String>,
}

/// A conversation turn in a session
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Turn {
    /// Turn sequence number
    pub seq: u32,

    /// User prompt
    pub prompt: String,

    /// Assistant response
    pub response: String,

    /// Timestamp (millis since epoch for serialization compatibility)
    pub timestamp_ms: u64,

    /// Tool calls made during this turn
    #[serde(default)]
    pub tool_calls: Vec<TurnToolCall>,
}

impl Turn {
    /// Create a new turn
    pub fn new(seq: u32, prompt: String, response: String) -> Self {
        Self {
            seq,
            prompt,
            response,
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            tool_calls: Vec::new(),
        }
    }

    /// Get timestamp as SystemTime
    pub fn timestamp(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_millis(self.timestamp_ms)
    }
}

/// Session state for a conversation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    /// Unique session ID
    pub id: String,

    /// Session creation time (millis since epoch)
    pub created_at_ms: u64,

    /// Last activity time (millis since epoch)
    pub last_active_ms: u64,

    /// Project path this session is associated with
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,

    /// Conversation turns
    pub turns: Vec<Turn>,

    /// Session metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Session {
    /// Create a new session
    pub fn new(id: impl Into<String>) -> Self {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        Self {
            id: id.into(),
            created_at_ms: now_ms,
            last_active_ms: now_ms,
            project_path: None,
            turns: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Create a session with a specific timestamp (for testing)
    pub fn new_at(id: impl Into<String>, timestamp_ms: u64) -> Self {
        Self {
            id: id.into(),
            created_at_ms: timestamp_ms,
            last_active_ms: timestamp_ms,
            project_path: None,
            turns: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set project path
    pub fn with_project(mut self, path: impl Into<String>) -> Self {
        self.project_path = Some(path.into());
        self
    }

    /// Add a turn to the session
    pub fn add_turn(&mut self, prompt: String, response: String) -> &Turn {
        let turn = Turn::new(self.turns.len() as u32, prompt, response);
        self.turns.push(turn);
        self.last_active_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        self.turns.last().unwrap()
    }

    /// Add a turn with a specific timestamp (for testing)
    pub fn add_turn_at(
        &mut self,
        prompt: String,
        response: String,
        timestamp_ms: u64,
    ) -> &mut Turn {
        let mut turn = Turn::new(self.turns.len() as u32, prompt, response);
        turn.timestamp_ms = timestamp_ms;
        self.turns.push(turn);
        self.last_active_ms = timestamp_ms;
        self.turns.last_mut().unwrap()
    }

    /// Get the last turn
    pub fn last_turn(&self) -> Option<&Turn> {
        self.turns.last()
    }

    /// Get mutable last turn
    pub fn last_turn_mut(&mut self) -> Option<&mut Turn> {
        self.turns.last_mut()
    }

    /// Check if session is expired
    pub fn is_expired(&self, max_age: Duration) -> bool {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
            - self.last_active_ms;
        elapsed > max_age.as_millis() as u64
    }

    /// Check if session is expired relative to a given timestamp
    pub fn is_expired_at(&self, max_age: Duration, current_ms: u64) -> bool {
        let elapsed = current_ms.saturating_sub(self.last_active_ms);
        elapsed > max_age.as_millis() as u64
    }

    /// Get turn count
    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }

    /// Load session from file
    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Save session to file
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }
}

// =============================================================================
// JSONL Format Types (matching real Claude CLI v2.1.12)
// =============================================================================

/// User message content for JSONL.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserMessage {
    /// Role is always "user".
    pub role: &'static str,
    /// Message content.
    pub content: String,
}

/// User message line in JSONL format.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserMessageLine {
    /// Parent message UUID (null for first message, UUID for tool results).
    pub parent_uuid: Option<String>,
    /// Whether this is a sidechain session.
    pub is_sidechain: bool,
    /// User type (e.g., "external").
    pub user_type: String,
    /// Current working directory.
    pub cwd: String,
    /// Session ID.
    pub session_id: String,
    /// CLI version (e.g., "2.1.12").
    pub version: String,
    /// Current git branch (empty if not in a git repo).
    pub git_branch: String,
    /// Type is always "user".
    #[serde(rename = "type")]
    pub line_type: &'static str,
    /// Message payload.
    pub message: UserMessage,
    /// Message UUID.
    pub uuid: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
}

/// Tool result content block (for tool result messages).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResultContent {
    /// Tool use ID that this result corresponds to.
    pub tool_use_id: String,
    /// Type is always "tool_result".
    #[serde(rename = "type")]
    pub content_type: &'static str,
    /// Tool result content (string or structured).
    pub content: String,
}

/// User message with tool result content.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResultUserMessage {
    /// Role is always "user".
    pub role: &'static str,
    /// Tool result content blocks.
    pub content: Vec<ToolResultContent>,
}

/// Tool result message line in JSONL format.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultMessageLine {
    /// Parent message UUID (the assistant message with tool_use).
    pub parent_uuid: String,
    /// Whether this is a sidechain session.
    pub is_sidechain: bool,
    /// User type (e.g., "external").
    pub user_type: String,
    /// Current working directory.
    pub cwd: String,
    /// Session ID.
    pub session_id: String,
    /// CLI version (e.g., "2.1.12").
    pub version: String,
    /// Current git branch (empty if not in a git repo).
    pub git_branch: String,
    /// Type is always "user".
    #[serde(rename = "type")]
    pub line_type: &'static str,
    /// Message payload with tool result content.
    pub message: ToolResultUserMessage,
    /// Message UUID.
    pub uuid: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Tool-specific result data.
    pub tool_use_result: serde_json::Value,
    /// UUID of assistant message containing the tool_use.
    #[serde(rename = "sourceToolAssistantUUID")]
    pub source_tool_assistant_uuid: String,
}

/// Text content block for assistant messages.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    /// Plain text content.
    #[serde(rename = "text")]
    Text {
        /// The text content.
        text: String,
    },
    /// Tool use content.
    #[serde(rename = "tool_use")]
    ToolUse {
        /// Tool use ID.
        id: String,
        /// Tool name.
        name: String,
        /// Tool input.
        input: serde_json::Value,
    },
}

/// Cache creation breakdown for usage statistics.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CacheCreation {
    /// Ephemeral 5-minute input tokens.
    #[serde(default)]
    pub ephemeral_5m_input_tokens: u32,
    /// Ephemeral 1-hour input tokens.
    #[serde(default)]
    pub ephemeral_1h_input_tokens: u32,
}

/// Usage statistics for API response.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens.
    pub input_tokens: u32,
    /// Cache creation input tokens.
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    /// Cache read input tokens.
    #[serde(default)]
    pub cache_read_input_tokens: u32,
    /// Cache creation breakdown.
    #[serde(default)]
    pub cache_creation: CacheCreation,
    /// Output tokens.
    pub output_tokens: u32,
    /// Service tier (e.g., "standard").
    #[serde(default)]
    pub service_tier: String,
}

impl Usage {
    /// Create a default usage with standard tier.
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_creation: CacheCreation::default(),
            output_tokens,
            service_tier: "standard".to_string(),
        }
    }
}

/// Assistant message content for JSONL (includes API envelope fields).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssistantMessage {
    /// Model name.
    pub model: String,
    /// Message ID (e.g., "msg_...").
    pub id: String,
    /// Type is always "message".
    #[serde(rename = "type")]
    pub message_type: &'static str,
    /// Role is always "assistant".
    pub role: &'static str,
    /// Content blocks (text, tool_use, etc.).
    pub content: Vec<ContentBlock>,
    /// Stop reason (e.g., "end_turn", "tool_use").
    pub stop_reason: Option<String>,
    /// Stop sequence (if applicable).
    pub stop_sequence: Option<String>,
    /// Usage statistics.
    pub usage: Usage,
}

/// Assistant message line in JSONL format.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMessageLine {
    /// Parent message UUID (the user message).
    pub parent_uuid: String,
    /// Whether this is a sidechain session.
    pub is_sidechain: bool,
    /// User type (e.g., "external").
    pub user_type: String,
    /// Current working directory.
    pub cwd: String,
    /// Session ID.
    pub session_id: String,
    /// CLI version (e.g., "2.1.12").
    pub version: String,
    /// Current git branch (empty if not in a git repo).
    pub git_branch: String,
    /// Message payload.
    pub message: AssistantMessage,
    /// Request ID.
    pub request_id: String,
    /// Type is always "assistant".
    #[serde(rename = "type")]
    pub line_type: &'static str,
    /// Message UUID.
    pub uuid: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
}

/// Queue operation for -p mode (first line in session).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueOperationLine {
    /// Type is always "queue-operation".
    #[serde(rename = "type")]
    pub line_type: &'static str,
    /// Operation type (e.g., "dequeue").
    pub operation: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Session ID.
    pub session_id: String,
}

/// Write queue-operation line for -p mode.
pub fn write_queue_operation(
    path: &Path,
    session_id: &str,
    operation: &str,
    timestamp: DateTime<Utc>,
) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    let line = QueueOperationLine {
        line_type: "queue-operation",
        operation: operation.to_string(),
        timestamp: timestamp.to_rfc3339(),
        session_id: session_id.to_string(),
    };
    writeln!(file, "{}", serde_json::to_string(&line)?)?;
    Ok(())
}

/// Parameters for writing JSONL turns.
#[derive(Clone, Debug)]
pub struct TurnParams<'a> {
    /// Session ID (UUID).
    pub session_id: &'a str,
    /// User message UUID.
    pub user_uuid: &'a str,
    /// Assistant message UUID.
    pub assistant_uuid: &'a str,
    /// Request ID.
    pub request_id: &'a str,
    /// User prompt text.
    pub prompt: &'a str,
    /// Assistant response text.
    pub response: &'a str,
    /// Model name.
    pub model: &'a str,
    /// Current working directory.
    pub cwd: &'a str,
    /// CLI version string.
    pub version: &'a str,
    /// Git branch name.
    pub git_branch: &'a str,
    /// Message ID for assistant message.
    pub message_id: &'a str,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
}

/// Append a conversation turn to a JSONL file.
pub fn append_turn_jsonl(path: &Path, params: &TurnParams) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    let timestamp_str = params.timestamp.to_rfc3339();

    // Write user message line
    let user_line = UserMessageLine {
        parent_uuid: None,
        is_sidechain: false,
        user_type: "external".to_string(),
        cwd: params.cwd.to_string(),
        session_id: params.session_id.to_string(),
        version: params.version.to_string(),
        git_branch: params.git_branch.to_string(),
        line_type: "user",
        message: UserMessage {
            role: "user",
            content: params.prompt.to_string(),
        },
        uuid: params.user_uuid.to_string(),
        timestamp: timestamp_str.clone(),
    };
    writeln!(file, "{}", serde_json::to_string(&user_line)?)?;

    // Write assistant message line
    let assistant_line = AssistantMessageLine {
        parent_uuid: params.user_uuid.to_string(),
        is_sidechain: false,
        user_type: "external".to_string(),
        cwd: params.cwd.to_string(),
        session_id: params.session_id.to_string(),
        version: params.version.to_string(),
        git_branch: params.git_branch.to_string(),
        message: AssistantMessage {
            model: params.model.to_string(),
            id: params.message_id.to_string(),
            message_type: "message",
            role: "assistant",
            content: vec![ContentBlock::Text {
                text: params.response.to_string(),
            }],
            stop_reason: None,
            stop_sequence: None,
            usage: Usage::new(2, 1),
        },
        request_id: params.request_id.to_string(),
        line_type: "assistant",
        uuid: params.assistant_uuid.to_string(),
        timestamp: timestamp_str,
    };
    writeln!(file, "{}", serde_json::to_string(&assistant_line)?)?;

    Ok(())
}

/// Parameters for writing a user message JSONL line.
#[derive(Clone, Debug)]
pub struct UserMessageParams<'a> {
    /// Session ID (UUID).
    pub session_id: &'a str,
    /// User message UUID.
    pub user_uuid: &'a str,
    /// Parent UUID (None for first message, Some for tool results).
    pub parent_uuid: Option<&'a str>,
    /// User prompt text (or tool result content).
    pub content: UserMessageContent<'a>,
    /// Current working directory.
    pub cwd: &'a str,
    /// CLI version string.
    pub version: &'a str,
    /// Git branch name.
    pub git_branch: &'a str,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
}

/// User message content variants.
#[derive(Clone, Debug)]
pub enum UserMessageContent<'a> {
    /// Simple text prompt.
    Text(&'a str),
    /// Tool result content.
    ToolResult {
        /// Tool use ID that this result corresponds to.
        tool_use_id: &'a str,
        /// Tool result content string.
        content: &'a str,
        /// Tool-specific result data (e.g., oldTodos/newTodos for TodoWrite).
        tool_use_result: serde_json::Value,
        /// UUID of assistant message containing the tool_use.
        source_tool_assistant_uuid: &'a str,
    },
}

/// Append a user message to a JSONL file.
pub fn append_user_message_jsonl(path: &Path, params: &UserMessageParams) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    let timestamp_str = params.timestamp.to_rfc3339();

    match &params.content {
        UserMessageContent::Text(text) => {
            let user_line = UserMessageLine {
                parent_uuid: params.parent_uuid.map(String::from),
                is_sidechain: false,
                user_type: "external".to_string(),
                cwd: params.cwd.to_string(),
                session_id: params.session_id.to_string(),
                version: params.version.to_string(),
                git_branch: params.git_branch.to_string(),
                line_type: "user",
                message: UserMessage {
                    role: "user",
                    content: (*text).to_string(),
                },
                uuid: params.user_uuid.to_string(),
                timestamp: timestamp_str,
            };
            writeln!(file, "{}", serde_json::to_string(&user_line)?)?;
        }
        UserMessageContent::ToolResult {
            tool_use_id,
            content,
            tool_use_result,
            source_tool_assistant_uuid,
        } => {
            let tool_result_line = ToolResultMessageLine {
                parent_uuid: params
                    .parent_uuid
                    .map(String::from)
                    .unwrap_or_else(|| source_tool_assistant_uuid.to_string()),
                is_sidechain: false,
                user_type: "external".to_string(),
                cwd: params.cwd.to_string(),
                session_id: params.session_id.to_string(),
                version: params.version.to_string(),
                git_branch: params.git_branch.to_string(),
                line_type: "user",
                message: ToolResultUserMessage {
                    role: "user",
                    content: vec![ToolResultContent {
                        tool_use_id: (*tool_use_id).to_string(),
                        content_type: "tool_result",
                        content: (*content).to_string(),
                    }],
                },
                uuid: params.user_uuid.to_string(),
                timestamp: timestamp_str,
                tool_use_result: tool_use_result.clone(),
                source_tool_assistant_uuid: (*source_tool_assistant_uuid).to_string(),
            };
            writeln!(file, "{}", serde_json::to_string(&tool_result_line)?)?;
        }
    }

    Ok(())
}

/// Parameters for writing an assistant message JSONL line.
#[derive(Clone, Debug)]
pub struct AssistantMessageParams<'a> {
    /// Session ID (UUID).
    pub session_id: &'a str,
    /// Assistant message UUID.
    pub assistant_uuid: &'a str,
    /// Parent UUID (the user message).
    pub parent_uuid: &'a str,
    /// Request ID.
    pub request_id: &'a str,
    /// Message ID (msg_...).
    pub message_id: &'a str,
    /// Content blocks (text, tool_use, etc.).
    pub content: Vec<ContentBlock>,
    /// Model name.
    pub model: &'a str,
    /// Stop reason (e.g., "end_turn", "tool_use").
    pub stop_reason: Option<&'a str>,
    /// Current working directory.
    pub cwd: &'a str,
    /// CLI version string.
    pub version: &'a str,
    /// Git branch name.
    pub git_branch: &'a str,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
}

/// Append an assistant message to a JSONL file.
pub fn append_assistant_message_jsonl(
    path: &Path,
    params: &AssistantMessageParams,
) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    let timestamp_str = params.timestamp.to_rfc3339();

    let assistant_line = AssistantMessageLine {
        parent_uuid: params.parent_uuid.to_string(),
        is_sidechain: false,
        user_type: "external".to_string(),
        cwd: params.cwd.to_string(),
        session_id: params.session_id.to_string(),
        version: params.version.to_string(),
        git_branch: params.git_branch.to_string(),
        message: AssistantMessage {
            model: params.model.to_string(),
            id: params.message_id.to_string(),
            message_type: "message",
            role: "assistant",
            content: params.content.clone(),
            stop_reason: params.stop_reason.map(String::from),
            stop_sequence: None,
            usage: Usage::new(2, 1),
        },
        request_id: params.request_id.to_string(),
        line_type: "assistant",
        uuid: params.assistant_uuid.to_string(),
        timestamp: timestamp_str,
    };
    writeln!(file, "{}", serde_json::to_string(&assistant_line)?)?;

    Ok(())
}

/// Session manager for multiple sessions
pub struct SessionManager {
    /// Active sessions by ID
    sessions: HashMap<String, Session>,

    /// Current session ID
    current: Option<String>,

    /// Session storage directory
    storage_dir: Option<PathBuf>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            current: None,
            storage_dir: None,
        }
    }

    /// Set storage directory for persistence
    pub fn with_storage(mut self, dir: impl Into<PathBuf>) -> Self {
        self.storage_dir = Some(dir.into());
        self
    }

    /// Get storage directory
    pub fn storage_dir(&self) -> Option<&PathBuf> {
        self.storage_dir.as_ref()
    }

    /// Create a new session and make it current
    pub fn create_session(&mut self) -> &mut Session {
        let id = generate_session_id();
        let session = Session::new(&id);
        self.sessions.insert(id.clone(), session);
        self.current = Some(id.clone());
        self.sessions.get_mut(&id).unwrap()
    }

    /// Create a new session with a specific ID
    pub fn create_session_with_id(&mut self, id: impl Into<String>) -> &mut Session {
        let id = id.into();
        let session = Session::new(&id);
        self.sessions.insert(id.clone(), session);
        self.current = Some(id.clone());
        self.sessions.get_mut(&id).unwrap()
    }

    /// Get current session ID
    pub fn current_id(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// Get or create the current session
    pub fn current_session(&mut self) -> &mut Session {
        if self.current.is_none() {
            self.create_session();
        }
        let id = self.current.as_ref().unwrap();
        self.sessions.get_mut(id).unwrap()
    }

    /// Get current session without creating
    pub fn get_current(&self) -> Option<&Session> {
        self.current.as_ref().and_then(|id| self.sessions.get(id))
    }

    /// Get session by ID
    pub fn get(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    /// Get mutable session by ID
    pub fn get_mut(&mut self, id: &str) -> Option<&mut Session> {
        self.sessions.get_mut(id)
    }

    /// Resume a session by ID
    pub fn resume(&mut self, id: &str) -> Option<&mut Session> {
        if self.sessions.contains_key(id) {
            self.current = Some(id.to_string());
            self.sessions.get_mut(id)
        } else if let Some(session) = self.load_session(id) {
            self.sessions.insert(id.to_string(), session);
            self.current = Some(id.to_string());
            self.sessions.get_mut(id)
        } else {
            None
        }
    }

    /// Continue the most recent session
    pub fn continue_session(&mut self) -> Option<&mut Session> {
        // Find most recent session
        let most_recent = self
            .sessions
            .values()
            .max_by_key(|s| s.last_active_ms)
            .map(|s| s.id.clone());

        if let Some(id) = most_recent {
            self.current = Some(id.clone());
            self.sessions.get_mut(&id)
        } else {
            // Try to find most recent from storage
            self.load_most_recent()
        }
    }

    /// Save current session to storage
    pub fn save_current(&self) -> std::io::Result<()> {
        let Some(ref dir) = self.storage_dir else {
            return Ok(());
        };
        let Some(ref id) = self.current else {
            return Ok(());
        };
        let Some(session) = self.sessions.get(id) else {
            return Ok(());
        };

        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("{}.json", id));
        session.save(&path)
    }

    /// Load a session from storage
    fn load_session(&self, id: &str) -> Option<Session> {
        let dir = self.storage_dir.as_ref()?;
        let path = dir.join(format!("{}.json", id));
        Session::load(&path).ok()
    }

    /// Load the most recent session from storage
    fn load_most_recent(&mut self) -> Option<&mut Session> {
        let dir = self.storage_dir.as_ref()?;
        if !dir.exists() {
            return None;
        }

        let mut most_recent: Option<Session> = None;

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "json") {
                    if let Ok(session) = Session::load(&path) {
                        if most_recent
                            .as_ref()
                            .map(|s| session.last_active_ms > s.last_active_ms)
                            .unwrap_or(true)
                        {
                            most_recent = Some(session);
                        }
                    }
                }
            }
        }

        if let Some(session) = most_recent {
            let id = session.id.clone();
            self.sessions.insert(id.clone(), session);
            self.current = Some(id.clone());
            self.sessions.get_mut(&id)
        } else {
            None
        }
    }

    /// List all session IDs
    pub fn list_ids(&self) -> Vec<&str> {
        self.sessions.keys().map(|s| s.as_str()).collect()
    }

    /// Get session count
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// Clear all sessions
    pub fn clear(&mut self) {
        self.sessions.clear();
        self.current = None;
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

fn generate_session_id() -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("session_{:x}", ts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_session() {
        let session = Session::new("test_session");
        assert_eq!(session.id, "test_session");
        assert!(session.turns.is_empty());
        assert!(session.project_path.is_none());
    }

    #[test]
    fn test_session_with_project() {
        let session = Session::new("test").with_project("/some/path");
        assert_eq!(session.project_path, Some("/some/path".to_string()));
    }

    #[test]
    fn test_add_turn() {
        let mut session = Session::new("test");
        session.add_turn("Hello".to_string(), "Hi there!".to_string());

        assert_eq!(session.turn_count(), 1);
        let turn = session.last_turn().unwrap();
        assert_eq!(turn.prompt, "Hello");
        assert_eq!(turn.response, "Hi there!");
        assert_eq!(turn.seq, 0);
    }

    #[test]
    fn test_multiple_turns() {
        let mut session = Session::new("test");
        session.add_turn("First".to_string(), "Response 1".to_string());
        session.add_turn("Second".to_string(), "Response 2".to_string());

        assert_eq!(session.turn_count(), 2);
        assert_eq!(session.turns[0].seq, 0);
        assert_eq!(session.turns[1].seq, 1);
    }

    #[test]
    fn test_session_expiration() {
        let session = Session::new_at("test", 0);
        // Session created at epoch, check if expired after 1 hour
        assert!(session.is_expired_at(Duration::from_secs(3600), 3600001));
        assert!(!session.is_expired_at(Duration::from_secs(3600), 1000));
    }

    #[test]
    fn test_session_save_load() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("session.json");

        let mut session = Session::new("test_session");
        session.add_turn("Hello".to_string(), "Hi!".to_string());
        session.save(&path).unwrap();

        let loaded = Session::load(&path).unwrap();
        assert_eq!(loaded.id, "test_session");
        assert_eq!(loaded.turn_count(), 1);
    }

    #[test]
    fn test_session_manager_create() {
        let mut manager = SessionManager::new();
        let session = manager.create_session();

        assert!(session.id.starts_with("session_"));
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_session_manager_create_with_id() {
        let mut manager = SessionManager::new();
        manager.create_session_with_id("my_session");

        assert_eq!(manager.current_id(), Some("my_session"));
        assert!(manager.get("my_session").is_some());
    }

    #[test]
    fn test_session_manager_current() {
        let mut manager = SessionManager::new();

        // Auto-creates session
        let session1 = manager.current_session();
        let id = session1.id.clone();

        // Returns same session
        let session2 = manager.current_session();
        assert_eq!(session2.id, id);
    }

    #[test]
    fn test_session_manager_resume() {
        let mut manager = SessionManager::new();
        manager.create_session_with_id("session_a");
        manager.create_session_with_id("session_b");

        assert_eq!(manager.current_id(), Some("session_b"));

        manager.resume("session_a");
        assert_eq!(manager.current_id(), Some("session_a"));
    }

    #[test]
    fn test_session_manager_continue() {
        let mut manager = SessionManager::new();

        // Create sessions with different timestamps
        let session1 = manager.create_session_with_id("old");
        session1.last_active_ms = 1000;

        let session2 = manager.create_session_with_id("new");
        session2.last_active_ms = 2000;

        // Switch to old
        manager.resume("old");
        assert_eq!(manager.current_id(), Some("old"));

        // Continue should pick newest
        manager.continue_session();
        assert_eq!(manager.current_id(), Some("new"));
    }

    #[test]
    fn test_session_manager_persistence() {
        let temp = tempfile::tempdir().unwrap();

        // Create and save session
        {
            let mut manager = SessionManager::new().with_storage(temp.path());
            let session = manager.create_session_with_id("persistent");
            session.add_turn("Hello".to_string(), "Hi!".to_string());
            manager.save_current().unwrap();
        }

        // Load in new manager
        {
            let mut manager = SessionManager::new().with_storage(temp.path());
            let session = manager.resume("persistent").unwrap();
            assert_eq!(session.turn_count(), 1);
        }
    }

    #[test]
    fn test_session_manager_clear() {
        let mut manager = SessionManager::new();
        manager.create_session_with_id("session_1");
        manager.create_session_with_id("session_2");

        assert_eq!(manager.len(), 2);
        manager.clear();
        assert!(manager.is_empty());
        assert!(manager.current_id().is_none());
    }

    #[test]
    fn test_turn_tool_calls() {
        let mut session = Session::new("test");
        let turn = session.add_turn_at("Hello".to_string(), "Hi!".to_string(), 1000);

        turn.tool_calls.push(TurnToolCall {
            tool: "Bash".to_string(),
            input: serde_json::json!({"command": "ls"}),
            output: Some("file1\nfile2".to_string()),
        });

        assert_eq!(session.last_turn().unwrap().tool_calls.len(), 1);
    }
}
