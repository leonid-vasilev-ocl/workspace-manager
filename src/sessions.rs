use std::{path::Path, process::ExitStatus};

use crate::tmux::TmuxSessionManager;
use anyhow::Result;

pub enum SessionManager {
    Tmux(TmuxSessionManager),
}

pub trait SessionManagerImpl {
    fn list_active_sessions(&self) -> Result<Vec<String>>;
    fn new_session(&self, session_name: &str, session_path: &Path) -> Result<ExitStatus>;
    fn has_session(&self, session_name: &str) -> Result<bool>;
    fn switch_client(&self, session_name: &str) -> Result<()>;
    fn is_same_session(&self, session_name: &str) -> bool;
    fn get_attached_session(&self) -> Option<String>;
    fn kill_session(&self, session_name: &str) -> Result<()>;
    fn get_current_session(&self) -> Option<String>;
}

impl SessionManagerImpl for SessionManager {
    fn new_session(&self, session_name: &str, session_path: &Path) -> Result<ExitStatus> {
        match self {
            Self::Tmux(tmux) => tmux.new_session(session_name, session_path),
        }
    }

    fn has_session(&self, session_name: &str) -> Result<bool> {
        match self {
            Self::Tmux(tmux) => tmux.has_session(session_name),
        }
    }

    fn switch_client(&self, session_name: &str) -> Result<()> {
        match self {
            Self::Tmux(tmux) => tmux.switch_client(session_name),
        }
    }

    fn is_same_session(&self, session_name: &str) -> bool {
        match self {
            Self::Tmux(tmux) => tmux.is_same_session(session_name),
        }
    }

    fn get_attached_session(&self) -> Option<String> {
        match self {
            Self::Tmux(tmux) => tmux.get_attached_session(),
        }
    }

    fn list_active_sessions(&self) -> Result<Vec<String>> {
        match self {
            Self::Tmux(tmux) => tmux.list_active_sessions(),
        }
    }

    fn kill_session(&self, session_name: &str) -> Result<()> {
        match self {
            Self::Tmux(tmux) => tmux.kill_session(session_name),
        }
    }
    fn get_current_session(&self) -> Option<String> {
        match self {
            Self::Tmux(tmux) => tmux.get_current_session(),
        }
    }
}

pub fn get_formatted_session_name(name: &str) -> String {
    name.replace(".", "_")
}
