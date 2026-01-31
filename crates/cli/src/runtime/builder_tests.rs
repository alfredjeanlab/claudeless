// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use clap::Parser;

use super::*;

#[test]
fn builder_validates_cli() {
    // Create a valid CLI and verify builder creation succeeds
    let cli = Cli::try_parse_from(["claude", "-p", "test"]).unwrap();
    let builder = RuntimeBuilder::new(cli);
    assert!(builder.is_ok());
}
