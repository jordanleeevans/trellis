//! An entry-point TUI panel listing locally tracked `gh stack` stacks.

mod doctor;
mod git;
mod shell;
mod stack;
#[cfg(test)]
mod test_fixtures;
mod tui;

use anyhow::Result;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let cwd = env::current_dir()?;
    let shell = shell::ProcessShell;

    if let Err(err) = doctor::check(&shell, cwd.as_path()).await {
        eprintln!("{err}");
        std::process::exit(1);
    }

    tui::run(&shell, cwd.as_path()).await
}
