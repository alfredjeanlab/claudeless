use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

mod pty;
mod screen;
mod script;

#[derive(Parser, Debug)]
#[command(name = "capsh", about = "Headless terminal capture with scripting DSL")]
struct Args {
    /// Directory to save frame snapshots
    #[arg(long)]
    frames: Option<PathBuf>,

    /// Script file (use - for stdin)
    #[arg(long)]
    script: Option<PathBuf>,

    /// Terminal width
    #[arg(long, default_value = "80")]
    cols: u16,

    /// Terminal height
    #[arg(long, default_value = "24")]
    rows: u16,

    /// Command to run
    #[arg(last = true, required = true)]
    command: Vec<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("capsh: {:?}", args);
    todo!("implement session loop")
}
