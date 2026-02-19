use anyhow::{Result, anyhow};
use std::{
    cmp::Ordering,
    collections::HashSet,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    ns::{Notification, get_notification},
    sessions::{SessionManager, SessionManagerImpl},
};

pub struct WorkspacesBuilder<'a> {
    config: &'a Config,
    session_manager: Option<&'a SessionManager>,
    is_collect_notification: bool,
}

impl<'a> WorkspacesBuilder<'a> {
    pub fn new(config: &'a Config) -> Self {
        WorkspacesBuilder {
            config,
            session_manager: None,
            is_collect_notification: false,
        }
    }

    pub fn get_open_sessions(mut self, session_manager: &'a SessionManager) -> Self {
        self.session_manager = Some(session_manager);
        self
    }

    pub fn collect_notifications(mut self) -> Self {
        self.is_collect_notification = true;
        self
    }

    pub fn build(&self) -> Result<Vec<Workspace>> {
        let mut workspaces: Vec<Workspace> = self
            .config
            .get_ws_all()
            .into_iter()
            .map(|c| {
                let mut notification = None;
                if self.is_collect_notification {
                    notification = get_notification(c);
                }

                Workspace {
                    name: c.name.clone(),
                    path: c.path.clone(),
                    notification,
                    is_open: false,
                }
            })
            .collect();

        if let Some(sessions) = self.session_manager {
            let active_sessions_names: HashSet<String> =
                sessions.list_active_sessions()?.into_iter().collect();
            for ws in workspaces.iter_mut() {
                let ws_name = ws.get_name_or_last_path()?;
                ws.is_open = active_sessions_names.contains(ws_name);
            }
        }

        Ok(workspaces)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkspaceConfig {
    pub name: Option<String>,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct Workspace {
    pub name: Option<String>,
    pub path: PathBuf,
    pub notification: Option<Notification>,
    pub is_open: bool,
}

//Ordering part
pub fn order_by_notification(a: &Workspace, b: &Workspace) -> Ordering {
    let has_a = a.notification.is_some();
    let has_b = b.notification.is_some();

    let elapsed_a = a
        .notification
        .as_ref()
        .map(|n| n.elapsed)
        .unwrap_or(u64::MAX);

    let elapsed_b = b
        .notification
        .as_ref()
        .map(|n| n.elapsed)
        .unwrap_or(u64::MAX);

    has_b.cmp(&has_a).then_with(|| elapsed_b.cmp(&elapsed_a))
}

pub fn order_by_open_session(a: &Workspace, b: &Workspace) -> Ordering {
    let open_a = a.is_open;
    let open_b = b.is_open;

    open_b.cmp(&open_a)
}

pub trait WorkspaceName {
    fn get_name_or_last_path(&self) -> Result<&str>;
}

impl WorkspaceName for WorkspaceConfig {
    fn get_name_or_last_path(&self) -> Result<&str> {
        self.name
            .as_deref()
            .or_else(|| self.path.file_name().and_then(|os| os.to_str()))
            .ok_or_else(|| anyhow!("can't get name from path: {}", self.path.to_string_lossy()))
    }
}

impl WorkspaceName for Workspace {
    fn get_name_or_last_path(&self) -> Result<&str> {
        self.name
            .as_deref()
            .or_else(|| self.path.file_name().and_then(|os| os.to_str()))
            .ok_or_else(|| anyhow!("can't get name from path: {}", self.path.to_string_lossy()))
    }
}

impl AsRef<Path> for Workspace {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}
