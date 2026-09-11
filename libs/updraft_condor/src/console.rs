//! Console prompts for the values the tool cannot detect.

use anyhow::{Context as _, Result};
use std::io::{BufRead as _, IsTerminal as _, Write as _};

/// Prints the question and returns the trimmed answer line.
pub fn prompt(question: &str) -> Result<String> {
    let mut stdout = std::io::stdout().lock();
    write!(stdout, "{question}: ")?;
    stdout.flush()?;
    let mut answer = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut answer)
        .context("failed to read the console")?;
    Ok(answer.trim().to_owned())
}

/// Keeps a console window open after a fatal error, so the message stays
/// readable when the tool was started with a double click.
pub fn wait_for_enter() {
    if !std::io::stdin().is_terminal() {
        return;
    }
    let mut stdout = std::io::stdout().lock();
    let _ = write!(stdout, "Press Enter to close.");
    let _ = stdout.flush();
    let mut line = String::new();
    let _ = std::io::stdin().lock().read_line(&mut line);
}
