// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Alfred Jean LLC

//! Test capture CLI entry point.

use clap::Parser;

/// Test capture CLI for recording and comparing test output
#[derive(Parser, Debug)]
#[command(name = "test-capture")]
#[command(about = "Capture and compare CLI test output")]
struct Cli {
    /// Placeholder argument
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if cli.verbose {
        eprintln!("test-capture: verbose mode enabled");
    }

    println!("test-capture: ready");
    Ok(())
}
