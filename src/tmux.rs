use std::{
    path::Path,
    process::{Command, ExitStatus},
};

use crate::sessions::SessionManagerImpl;

use anyhow::Result;

pub struct TmuxSessionManager;

impl SessionManagerImpl for TmuxSessionManager {
    fn new_session(&self, session_name: &str, session_path: &Path) -> Result<ExitStatus> {
        let mut tmux_command = Command::new("tmux");
        tmux_command
            .arg("new-session")
            .arg("-s")
            .arg(&session_name)
            .arg("-d")
            .arg("-c")
            .arg(&session_path);

        Ok(tmux_command.status()?)
    }

    fn has_session(&self, session_name: &str) -> Result<bool> {
        let status = Command::new("tmux")
            .args(["has-session", "-t", format!("={}", &session_name).as_str()])
            .status()?;
        Ok(status.success())
    }

    fn switch_client(&self, session_name: &str) -> Result<()> {
        if is_in_tmux() {
            switch_client_inside(session_name)?
        } else {
            switch_client_outside(session_name)?
        }
        Ok(())
    }
    fn is_same_session(&self, session_name: &str) -> bool {
        if !is_in_tmux() {
            return false;
        }

        let current_session = Command::new("tmux")
            .arg("display-message")
            .arg("-p")
            .arg("#S")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        Some(session_name) == current_session.as_deref()
    }
}

fn is_in_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}

fn switch_client_inside(session_name: &str) -> Result<()> {
    Command::new("tmux")
        .arg("switch-client")
        .arg("-t")
        .arg(format!("={}", &session_name))
        .spawn()?;
    Ok(())
}

fn switch_client_outside(session_name: &str) -> Result<()> {
    Command::new("tmux")
        .arg("attach")
        .arg("-t")
        .arg(format!("={}", &session_name))
        .status()?;
    Ok(())
}
