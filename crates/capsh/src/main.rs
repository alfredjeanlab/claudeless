use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

mod nbio;
mod pty;
mod recording;
mod screen;
mod script;
mod session;

#[derive(Parser, Debug)]
#[command(name = "capsh", about = "Headless terminal capture with scripting DSL")]
struct Args {
    /// Directory to save frame snapshots
    #[arg(long)]
    frames: Option<PathBuf>,

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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let config = session::Config {
        command: args.command.join(" "),
        cols: args.cols,
        rows: args.rows,
        frames_dir: args.frames,
        script: script::load_stdin()?,
    };

    let exit_code = session::run(config).await?;
    std::process::exit(exit_code);
}
