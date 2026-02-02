// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Custom help formatter producing commander.js-style output.
//!
//! Replaces clap's built-in help rendering to match real Claude Code's
//! commander.js-based help output exactly.

// =============================================================================
// Data structures
// =============================================================================

/// A complete help page specification.
pub struct HelpSpec {
    /// Usage line (e.g., "claude [options] [command] [prompt]")
    pub usage: &'static str,
    /// Description text (may be multi-line for wrapping)
    pub description: &'static str,
    /// Ordered sections (Arguments, Options, Commands, etc.)
    pub sections: Vec<HelpSection>,
    /// Optional trailing text (e.g., Claudeless-specific options)
    pub after_text: Option<String>,
}

/// A section within a help page (e.g., "Options:", "Commands:").
pub struct HelpSection {
    /// Section header (e.g., "Options")
    pub title: &'static str,
    /// Items in this section (entries or raw text blocks)
    pub items: Vec<HelpItem>,
}

/// An item within a help section.
pub enum HelpItem {
    /// A flag/command entry with aligned description.
    Entry {
        /// Left-hand side (e.g., "-c, --continue" or "doctor")
        flags: &'static str,
        /// Description text
        description: &'static str,
    },
    /// A raw text block (e.g., examples between command entries).
    Text(&'static str),
}

// =============================================================================
// Rendering
// =============================================================================

/// Render a help page to a string.
///
/// Calculates alignment column automatically based on the longest entry
/// across all sections: `desc_col = 2 + max(flags.len()) + 2`.
///
/// When `wrap_width` is specified, descriptions wrap at that total line width.
/// When `None`, descriptions are output on a single line (no wrapping).
pub fn render_help(spec: &HelpSpec, wrap_width: Option<usize>) -> String {
    let mut out = String::new();

    // Usage line
    out.push_str(&format!("Usage: {}\n", spec.usage));

    // Description
    if !spec.description.is_empty() {
        out.push('\n');
        out.push_str(spec.description);
        out.push('\n');
    }

    // Calculate alignment column across all sections
    let desc_col = calc_desc_col(&spec.sections);

    // Render sections
    for section in &spec.sections {
        out.push('\n');
        out.push_str(section.title);
        out.push_str(":\n");

        for item in &section.items {
            match item {
                HelpItem::Entry { flags, description } => {
                    render_entry(&mut out, flags, description, desc_col, wrap_width);
                }
                HelpItem::Text(text) => {
                    out.push_str(text);
                }
            }
        }
    }

    out
}

/// Calculate the description column from all sections' entries.
fn calc_desc_col(sections: &[HelpSection]) -> usize {
    let max_flags_len = sections
        .iter()
        .flat_map(|s| s.items.iter())
        .filter_map(|item| match item {
            HelpItem::Entry { flags, .. } => Some(flags.len()),
            HelpItem::Text(_) => None,
        })
        .max()
        .unwrap_or(0);

    // 2 spaces indent + flags + 2 spaces padding
    2 + max_flags_len + 2
}

/// Render a single entry with alignment and optional wrapping.
fn render_entry(
    out: &mut String,
    flags: &str,
    description: &str,
    desc_col: usize,
    wrap_width: Option<usize>,
) {
    let indent = 2;
    let flags_width = indent + flags.len();

    if description.is_empty() {
        out.push_str(&format!("  {}\n", flags));
        return;
    }

    if flags_width >= desc_col {
        // Flags too long — description on next line
        out.push_str(&format!("  {}\n", flags));
        let padding = " ".repeat(desc_col);
        if let Some(width) = wrap_width {
            let max_desc = width - desc_col;
            let wrapped = wrap_text(description, max_desc);
            for line in &wrapped {
                out.push_str(&format!("{}{}\n", padding, line));
            }
        } else {
            out.push_str(&format!("{}{}\n", padding, description));
        }
    } else {
        // Normal case — flags + padding + description on same line
        let padding = desc_col - flags_width;
        if let Some(width) = wrap_width {
            let max_desc = width - desc_col;
            let wrapped = wrap_text(description, max_desc);
            for (i, line) in wrapped.iter().enumerate() {
                if i == 0 {
                    out.push_str(&format!("  {}{}{}\n", flags, " ".repeat(padding), line));
                } else {
                    out.push_str(&format!("{}{}\n", " ".repeat(desc_col), line));
                }
            }
        } else {
            out.push_str(&format!(
                "  {}{}{}\n",
                flags,
                " ".repeat(padding),
                description
            ));
        }
    }
}

/// Wrap text at word boundaries to fit within `max_width` characters.
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if text.len() < max_width {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split(' ') {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() < max_width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

// =============================================================================
// Help page specifications
// =============================================================================

/// Render the main `claude --help` output.
pub fn render_main_help() -> String {
    let spec = HelpSpec {
        usage: "claude [options] [command] [prompt]",
        description: "Claude Code - starts an interactive session by default, use -p/--print for\nnon-interactive output",
        sections: vec![
            HelpSection {
                title: "Arguments",
                items: vec![
                    HelpItem::Entry { flags: "prompt", description: "Your prompt" },
                ],
            },
            HelpSection {
                title: "Options",
                items: main_options(),
            },
            HelpSection {
                title: "Commands",
                items: vec![
                    HelpItem::Entry { flags: "doctor", description: "Check the health of your Claude Code auto-updater" },
                    HelpItem::Entry { flags: "install [options] [target]", description: "Install Claude Code native build. Use [target] to specify version (stable, latest, or specific version)" },
                    HelpItem::Entry { flags: "mcp", description: "Configure and manage MCP servers" },
                    HelpItem::Entry { flags: "plugin", description: "Manage Claude Code plugins" },
                    HelpItem::Entry { flags: "setup-token", description: "Set up a long-lived authentication token (requires Claude subscription)" },
                    HelpItem::Entry { flags: "update", description: "Check for updates and install if available" },
                ],
            },
        ],
        after_text: None,
    };
    render_help(&spec, None)
}

/// Render `claude doctor --help`.
pub fn render_doctor_help() -> String {
    let spec = HelpSpec {
        usage: "claude doctor [options]",
        description: "Check the health of your Claude Code auto-updater",
        sections: vec![HelpSection {
            title: "Options",
            items: vec![help_entry()],
        }],
        after_text: None,
    };
    render_help(&spec, Some(80))
}

/// Render `claude install --help`.
pub fn render_install_help() -> String {
    let spec = HelpSpec {
        usage: "claude install [options] [target]",
        description: "Install Claude Code native build. Use [target] to specify version (stable,\nlatest, or specific version)",
        sections: vec![HelpSection {
            title: "Options",
            items: vec![
                HelpItem::Entry {
                    flags: "--force",
                    description: "Force installation even if already installed",
                },
                help_entry(),
            ],
        }],
        after_text: None,
    };
    render_help(&spec, Some(80))
}

/// Render `claude mcp --help`.
pub fn render_mcp_help() -> String {
    let spec = HelpSpec {
        usage: "claude mcp [options] [command]",
        description: "Configure and manage MCP servers",
        sections: vec![
            HelpSection {
                title: "Options",
                items: vec![help_entry()],
            },
            HelpSection {
                title: "Commands",
                items: vec![
                    HelpItem::Entry {
                        flags: "add [options] <name> <commandOrUrl> [args...]",
                        description: "Add an MCP server to Claude Code.",
                    },
                    HelpItem::Text("  \n  Examples:\n    # Add HTTP server:\n    claude mcp add --transport http sentry https://mcp.sentry.dev/mcp\n  \n    # Add HTTP server with headers:\n    claude mcp add --transport http corridor https://app.corridor.dev/api/mcp --header \"Authorization: Bearer ...\"\n  \n    # Add stdio server with environment variables:\n    claude mcp add -e API_KEY=xxx my-server -- npx my-mcp-server\n  \n    # Add stdio server with subprocess flags:\n    claude mcp add my-server -- my-command --some-flag arg1\n"),
                    HelpItem::Entry {
                        flags: "add-from-claude-desktop [options]",
                        description: "Import MCP servers from Claude Desktop (Mac and WSL only)",
                    },
                    HelpItem::Entry {
                        flags: "add-json [options] <name> <json>",
                        description: "Add an MCP server (stdio or SSE) with a JSON string",
                    },
                    HelpItem::Entry {
                        flags: "get <name>",
                        description: "Get details about an MCP server",
                    },
                    HelpItem::Entry {
                        flags: "help [command]",
                        description: "display help for command",
                    },
                    HelpItem::Entry {
                        flags: "list",
                        description: "List configured MCP servers",
                    },
                    HelpItem::Entry {
                        flags: "remove [options] <name>",
                        description: "Remove an MCP server",
                    },
                    HelpItem::Entry {
                        flags: "reset-project-choices",
                        description: "Reset all approved and rejected project-scoped (.mcp.json) servers within this project",
                    },
                    HelpItem::Entry {
                        flags: "serve [options]",
                        description: "Start the Claude Code MCP server",
                    },
                ],
            },
        ],
        after_text: None,
    };
    render_help(&spec, None)
}

/// Render `claude mcp add --help`.
pub fn render_mcp_add_help() -> String {
    let spec = HelpSpec {
        usage: "claude mcp add [options] <name> <commandOrUrl> [args...]",
        description: "Add an MCP server to Claude Code.\n\nExamples:\n  # Add HTTP server:\n  claude mcp add --transport http sentry https://mcp.sentry.dev/mcp\n\n  # Add HTTP server with headers:\n  claude mcp add --transport http corridor https://app.corridor.dev/api/mcp --header \"Authorization: Bearer ...\"\n\n  # Add stdio server with environment variables:\n  claude mcp add -e API_KEY=xxx my-server -- npx my-mcp-server\n\n  # Add stdio server with subprocess flags:\n  claude mcp add my-server -- my-command --some-flag arg1",
        sections: vec![HelpSection {
            title: "Options",
            items: vec![
                HelpItem::Entry {
                    flags: "-e, --env <env...>",
                    description: "Set environment variables (e.g. -e KEY=value)",
                },
                HelpItem::Entry {
                    flags: "-H, --header <header...>",
                    description: "Set WebSocket headers (e.g. -H \"X-Api-Key: abc123\" -H \"X-Custom: value\")",
                },
                help_entry(),
                HelpItem::Entry {
                    flags: "-s, --scope <scope>",
                    description: "Configuration scope (local, user, or project) (default: \"local\")",
                },
                HelpItem::Entry {
                    flags: "-t, --transport <transport>",
                    description: "Transport type (stdio, sse, http). Defaults to stdio if not specified.",
                },
            ],
        }],
        after_text: None,
    };
    render_help(&spec, Some(80))
}

/// Render `claude mcp serve --help`.
pub fn render_mcp_serve_help() -> String {
    let spec = HelpSpec {
        usage: "claude mcp serve [options]",
        description: "Start the Claude Code MCP server",
        sections: vec![HelpSection {
            title: "Options",
            items: vec![
                HelpItem::Entry {
                    flags: "-d, --debug",
                    description: "Enable debug mode",
                },
                help_entry(),
                HelpItem::Entry {
                    flags: "--verbose",
                    description: "Override verbose mode setting from config",
                },
            ],
        }],
        after_text: None,
    };
    render_help(&spec, Some(80))
}

/// Render `claude plugin --help`.
pub fn render_plugin_help() -> String {
    let spec = HelpSpec {
        usage: "claude plugin [options] [command]",
        description: "Manage Claude Code plugins",
        sections: vec![
            HelpSection {
                title: "Options",
                items: vec![help_entry()],
            },
            HelpSection {
                title: "Commands",
                items: vec![
                    HelpItem::Entry {
                        flags: "disable [options] [plugin]",
                        description: "Disable an enabled plugin",
                    },
                    HelpItem::Entry {
                        flags: "enable [options] <plugin>",
                        description: "Enable a disabled plugin",
                    },
                    HelpItem::Entry {
                        flags: "help [command]",
                        description: "display help for command",
                    },
                    HelpItem::Entry {
                        flags: "install|i [options] <plugin>",
                        description: "Install a plugin from available marketplaces (use plugin@marketplace for specific marketplace)",
                    },
                    HelpItem::Entry {
                        flags: "list [options]",
                        description: "List installed plugins",
                    },
                    HelpItem::Entry {
                        flags: "marketplace",
                        description: "Manage Claude Code marketplaces",
                    },
                    HelpItem::Entry {
                        flags: "uninstall|remove [options] <plugin>",
                        description: "Uninstall an installed plugin",
                    },
                    HelpItem::Entry {
                        flags: "update [options] <plugin>",
                        description: "Update a plugin to the latest version (restart required to apply)",
                    },
                    HelpItem::Entry {
                        flags: "validate [options] <path>",
                        description: "Validate a plugin or marketplace manifest",
                    },
                ],
            },
        ],
        after_text: None,
    };
    render_help(&spec, Some(80))
}

/// Render `claude plugin marketplace --help`.
pub fn render_plugin_marketplace_help() -> String {
    let spec = HelpSpec {
        usage: "claude plugin marketplace [options] [command]",
        description: "Manage Claude Code marketplaces",
        sections: vec![
            HelpSection {
                title: "Options",
                items: vec![help_entry()],
            },
            HelpSection {
                title: "Commands",
                items: vec![
                    HelpItem::Entry {
                        flags: "add [options] <source>",
                        description: "Add a marketplace from a URL, path, or GitHub repo",
                    },
                    HelpItem::Entry {
                        flags: "help [command]",
                        description: "display help for command",
                    },
                    HelpItem::Entry {
                        flags: "list [options]",
                        description: "List all configured marketplaces",
                    },
                    HelpItem::Entry {
                        flags: "remove|rm [options] <name>",
                        description: "Remove a configured marketplace",
                    },
                    HelpItem::Entry {
                        flags: "update [options] [name]",
                        description: "Update marketplace(s) from their source - updates all if no name specified",
                    },
                ],
            },
        ],
        after_text: None,
    };
    render_help(&spec, Some(80))
}

/// Render `claude setup-token --help`.
pub fn render_setup_token_help() -> String {
    let spec = HelpSpec {
        usage: "claude setup-token [options]",
        description: "Set up a long-lived authentication token (requires Claude subscription)",
        sections: vec![HelpSection {
            title: "Options",
            items: vec![help_entry()],
        }],
        after_text: None,
    };
    render_help(&spec, Some(80))
}

/// Render `claude update --help`.
pub fn render_update_help() -> String {
    let spec = HelpSpec {
        usage: "claude update [options]",
        description: "Check for updates and install if available",
        sections: vec![HelpSection {
            title: "Options",
            items: vec![help_entry()],
        }],
        after_text: None,
    };
    render_help(&spec, Some(80))
}

// =============================================================================
// Shared option entries
// =============================================================================

/// Standard `-h, --help` entry.
fn help_entry() -> HelpItem {
    HelpItem::Entry {
        flags: "-h, --help",
        description: "Display help for command",
    }
}

/// All options for the main help page, in alphabetical order.
fn main_options() -> Vec<HelpItem> {
    vec![
        HelpItem::Entry { flags: "--add-dir <directories...>", description: "Additional directories to allow tool access to" },
        HelpItem::Entry { flags: "--agent <agent>", description: "Agent for the current session. Overrides the 'agent' setting." },
        HelpItem::Entry { flags: "--agents <json>", description: "JSON object defining custom agents (e.g. '{\"reviewer\": {\"description\": \"Reviews code\", \"prompt\": \"You are a code reviewer\"}}')" },
        HelpItem::Entry { flags: "--allow-dangerously-skip-permissions", description: "Enable bypassing all permission checks as an option, without it being enabled by default. Recommended only for sandboxes with no internet access." },
        HelpItem::Entry { flags: "--allowedTools, --allowed-tools <tools...>", description: "Comma or space-separated list of tool names to allow (e.g. \"Bash(git:*) Edit\")" },
        HelpItem::Entry { flags: "--append-system-prompt <prompt>", description: "Append a system prompt to the default system prompt" },
        HelpItem::Entry { flags: "--betas <betas...>", description: "Beta headers to include in API requests (API key users only)" },
        HelpItem::Entry { flags: "--chrome", description: "Enable Claude in Chrome integration" },
        HelpItem::Entry { flags: "-c, --continue", description: "Continue the most recent conversation in the current directory" },
        HelpItem::Entry { flags: "--dangerously-skip-permissions", description: "Bypass all permission checks. Recommended only for sandboxes with no internet access." },
        HelpItem::Entry { flags: "-d, --debug [filter]", description: "Enable debug mode with optional category filtering (e.g., \"api,hooks\" or \"!statsig,!file\")" },
        HelpItem::Entry { flags: "--debug-file <path>", description: "Write debug logs to a specific file path (implicitly enables debug mode)" },
        HelpItem::Entry { flags: "--disable-slash-commands", description: "Disable all skills" },
        HelpItem::Entry { flags: "--disallowedTools, --disallowed-tools <tools...>", description: "Comma or space-separated list of tool names to deny (e.g. \"Bash(git:*) Edit\")" },
        HelpItem::Entry { flags: "--fallback-model <model>", description: "Enable automatic fallback to specified model when default model is overloaded (only works with --print)" },
        HelpItem::Entry { flags: "--file <specs...>", description: "File resources to download at startup. Format: file_id:relative_path (e.g., --file file_abc:doc.txt file_def:img.png)" },
        HelpItem::Entry { flags: "--fork-session", description: "When resuming, create a new session ID instead of reusing the original (use with --resume or --continue)" },
        HelpItem::Entry { flags: "--from-pr [value]", description: "Resume a session linked to a PR by PR number/URL, or open interactive picker with optional search term" },
        HelpItem::Entry { flags: "-h, --help", description: "Display help for command" },
        HelpItem::Entry { flags: "--ide", description: "Automatically connect to IDE on startup if exactly one valid IDE is available" },
        HelpItem::Entry { flags: "--include-partial-messages", description: "Include partial message chunks as they arrive (only works with --print and --output-format=stream-json)" },
        HelpItem::Entry { flags: "--input-format <format>", description: "Input format (only works with --print): \"text\" (default), or \"stream-json\" (realtime streaming input) (choices: \"text\", \"stream-json\")" },
        HelpItem::Entry { flags: "--json-schema <schema>", description: "JSON Schema for structured output validation. Example: {\"type\":\"object\",\"properties\":{\"name\":{\"type\":\"string\"}},\"required\":[\"name\"]}" },
        HelpItem::Entry { flags: "--max-budget-usd <amount>", description: "Maximum dollar amount to spend on API calls (only works with --print)" },
        HelpItem::Entry { flags: "--mcp-config <configs...>", description: "Load MCP servers from JSON files or strings (space-separated)" },
        HelpItem::Entry { flags: "--mcp-debug", description: "[DEPRECATED. Use --debug instead] Enable MCP debug mode (shows MCP server errors)" },
        HelpItem::Entry { flags: "--model <model>", description: "Model for the current session. Provide an alias for the latest model (e.g. 'sonnet' or 'opus') or a model's full name (e.g. 'claude-sonnet-4-5-20250929')." },
        HelpItem::Entry { flags: "--no-chrome", description: "Disable Claude in Chrome integration" },
        HelpItem::Entry { flags: "--no-session-persistence", description: "Disable session persistence - sessions will not be saved to disk and cannot be resumed (only works with --print)" },
        HelpItem::Entry { flags: "--output-format <format>", description: "Output format (only works with --print): \"text\" (default), \"json\" (single result), or \"stream-json\" (realtime streaming) (choices: \"text\", \"json\", \"stream-json\")" },
        HelpItem::Entry { flags: "--permission-mode <mode>", description: "Permission mode to use for the session (choices: \"acceptEdits\", \"bypassPermissions\", \"default\", \"delegate\", \"dontAsk\", \"plan\")" },
        HelpItem::Entry { flags: "--plugin-dir <paths...>", description: "Load plugins from directories for this session only (repeatable)" },
        HelpItem::Entry { flags: "-p, --print", description: "Print response and exit (useful for pipes). Note: The workspace trust dialog is skipped when Claude is run with the -p mode. Only use this flag in directories you trust." },
        HelpItem::Entry { flags: "--replay-user-messages", description: "Re-emit user messages from stdin back on stdout for acknowledgment (only works with --input-format=stream-json and --output-format=stream-json)" },
        HelpItem::Entry { flags: "-r, --resume [value]", description: "Resume a conversation by session ID, or open interactive picker with optional search term" },
        HelpItem::Entry { flags: "--session-id <uuid>", description: "Use a specific session ID for the conversation (must be a valid UUID)" },
        HelpItem::Entry { flags: "--setting-sources <sources>", description: "Comma-separated list of setting sources to load (user, project, local)." },
        HelpItem::Entry { flags: "--settings <file-or-json>", description: "Path to a settings JSON file or a JSON string to load additional settings from" },
        HelpItem::Entry { flags: "--strict-mcp-config", description: "Only use MCP servers from --mcp-config, ignoring all other MCP configurations" },
        HelpItem::Entry { flags: "--system-prompt <prompt>", description: "System prompt to use for the session" },
        HelpItem::Entry { flags: "--tools <tools...>", description: "Specify the list of available tools from the built-in set. Use \"\" to disable all tools, \"default\" to use all tools, or specify tool names (e.g. \"Bash,Edit,Read\")." },
        HelpItem::Entry { flags: "--verbose", description: "Override verbose mode setting from config" },
        HelpItem::Entry { flags: "-v, --version", description: "Output the version number" },
    ]
}

/// Build the claudeless-specific options section.
pub fn claudeless_options_section() -> String {
    let spec = HelpSpec {
        usage: "",
        description: "",
        sections: vec![HelpSection {
            title: "Claudeless Options",
            items: vec![
                HelpItem::Entry {
                    flags: "--capture <file>",
                    description: "Capture file for recording interactions",
                },
                HelpItem::Entry {
                    flags: "--claude-version <version>",
                    description: "Claude version to simulate",
                },
                HelpItem::Entry {
                    flags: "--failure <mode>",
                    description: "Failure mode to inject (choices: \"network-unreachable\", \"connection-timeout\", \"auth-error\", \"rate-limit\", \"out-of-credits\", \"partial-response\", \"malformed-json\")",
                },
                HelpItem::Entry {
                    flags: "--scenario <file>",
                    description: "Scenario file for scripted responses",
                },
            ],
        }],
        after_text: None,
    };

    // Render just the section part (skip empty usage/description)
    let desc_col = calc_desc_col(&spec.sections);
    let mut out = String::new();
    for section in &spec.sections {
        out.push_str(section.title);
        out.push_str(":\n");
        for item in &section.items {
            match item {
                HelpItem::Entry { flags, description } => {
                    render_entry(&mut out, flags, description, desc_col, None);
                }
                HelpItem::Text(text) => {
                    out.push_str(text);
                }
            }
        }
    }
    out
}

#[cfg(test)]
#[path = "help_tests.rs"]
mod tests;
