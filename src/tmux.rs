use std::{
    path::Path,
    process::{Command, ExitStatus},
};

use crate::sessions::SessionManagerImpl;

use anyhow::{anyhow, Result};

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

    fn get_current_session(&self) -> Option<String> {
        if !is_in_tmux() {
            return None;
        }

        let output = Command::new("tmux")
            .arg("display-message")
            .arg("-p")
            .arg("#S")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        output
    }

    fn is_same_session(&self, session_name: &str) -> bool {
        let current_session = self.get_current_session();

        Some(session_name) == current_session.as_deref()
    }

    fn get_attached_session(&self) -> Option<String> {
        if !is_in_tmux() {
            return None;
        }

        let output = Command::new("tmux")
            .arg("list-sessions")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

        if let Some(sessions) = output {
            let sessions: Vec<&str> = sessions.split("\n").collect();

            return sessions
                .iter()
                .find(|s| s.contains("(attached)"))
                .and_then(|s| s.split_once(":"))
                .map(|s| s.0.to_string());
        }

        None
    }

    fn list_active_sessions(&self) -> Result<Vec<String>> {
        let output = Command::new("tmux").arg("list-sessions").output()?;

        let stdout = String::from_utf8(output.stdout)?;

        let active_sessions = stdout
            .lines()
            .map(|s| s.split_once(":").map(|(first, _)| first.to_string()))
            .collect::<Option<Vec<_>>>()
            .ok_or(anyhow!("Invalid tmux session name, missing (:)"))?;

        Ok(active_sessions)
    }

    fn list_running_sessions(&self) -> Result<std::collections::HashSet<String>> {
        let output = Command::new("tmux")
            .args(["list-panes", "-a", "-F", "#{session_name}\t#{pane_title}"])
            .output()?;
        if !output.status.success() {
            return Ok(std::collections::HashSet::new());
        }
        let stdout = String::from_utf8(output.stdout)?;
        let mut set = std::collections::HashSet::new();
        for line in stdout.lines() {
            if let Some((session, title)) = line.split_once('\t') {
                if title_is_running(title) {
                    set.insert(session.to_string());
                }
            }
        }
        Ok(set)
    }

    fn kill_session(&self, session_name: &str) -> Result<()> {
        Command::new("tmux")
            .arg("kill-session")
            .arg("-t")
            .arg(format!("={}", &session_name))
            .status()?;
        Ok(())
    }
}

/// A Claude pane title leads with a braille spinner glyph (U+2800–U+28FF) while
/// it is working; idle shows a sparkle/dingbat (U+2700–27FF). Same signal the
/// tmux status bar uses to color its running indicator.
fn title_is_running(title: &str) -> bool {
    matches!(title.chars().next(), Some(c) if ('\u{2800}'..='\u{28FF}').contains(&c))
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
