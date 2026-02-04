// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

// Build scripts are expected to panic on failure.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let path = std::path::Path::new(&out_dir).join("env_names.rs");
    let mut f = std::fs::File::create(path).unwrap();

    let vars = [
        ("CLAUDELESS_CONFIG_DIR", "CLAUDELESS_CONFIG_DIR"),
        ("CLAUDELESS_STATE_DIR", "CLAUDELESS_STATE_DIR"),
        ("CLAUDE_CONFIG_DIR", "CLAUDE_CONFIG_DIR"),
        (
            "CLAUDELESS_EXIT_HINT_TIMEOUT_MS",
            "CLAUDELESS_EXIT_HINT_TIMEOUT_MS",
        ),
        ("CLAUDELESS_COMPACT_DELAY_MS", "CLAUDELESS_COMPACT_DELAY_MS"),
        ("CLAUDELESS_HOOK_TIMEOUT_MS", "CLAUDELESS_HOOK_TIMEOUT_MS"),
        ("CLAUDELESS_MCP_TIMEOUT_MS", "CLAUDELESS_MCP_TIMEOUT_MS"),
        (
            "CLAUDELESS_RESPONSE_DELAY_MS",
            "CLAUDELESS_RESPONSE_DELAY_MS",
        ),
        ("CARGO_BIN_EXE_CLAUDELESS", "CARGO_BIN_EXE_claudeless"),
        ("CARGO_TARGET_DIR", "CARGO_TARGET_DIR"),
        ("HOME", "HOME"),
    ];

    for (const_name, env_name) in vars {
        writeln!(f, "pub const {const_name}: &str = \"{env_name}\";").unwrap();
    }
}
