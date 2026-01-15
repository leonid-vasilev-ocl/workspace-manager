use std::{
    path::Path,
    process::{Command, ExitStatus},
};

use anyhow::Result;

pub fn new_session(session_name: &str, session_path: &Path, attach: bool) -> Result<ExitStatus> {
    let mut tmux_command = Command::new("tmux");
    tmux_command
        .arg("new-session")
        .arg("-A")
        .arg("-s")
        .arg(&session_name)
        .arg("-c")
        .arg(&session_path);

    let status = match attach {
        false => tmux_command.arg("-d").status()?,
        true => tmux_command.status()?,
    };

    Ok(status)
}

pub fn has_session(session_name: &str) -> Result<bool> {
    let status = Command::new("tmux")
        .args(["has-session", "-t", &session_name])
        .status()?;
    Ok(status.success())
}

pub fn switch_client(session_name: &str) -> Result<()> {
    Command::new("tmux")
        .arg("switch-client")
        .arg("-t")
        .arg(&session_name)
        .spawn()?;
    Ok(())
}

pub fn is_same_tmux_session(session_name: &str) -> bool {
    let current_session = Command::new("tmux")
        .arg("display-message")
        .arg("-p")
        .arg("#S")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    Some(session_name) == current_session.as_deref()
}

pub fn is_in_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}
