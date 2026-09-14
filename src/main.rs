//! Prints `git status --short` for the current directory.

mod doctor;
mod git;
mod shell;

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

    let output = git::status(&shell, cwd.as_path()).await?;

    println!("{output}");

    Ok(())
}
