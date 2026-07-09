use std::{
    fs::Metadata,
    path::{Path, PathBuf},
};

use crate::{
    config::Config,
    sessions::{get_formatted_session_name, SessionManager, SessionManagerImpl},
    tmux::TmuxSessionManager,
    workspace::WorkspaceName,
};
use anyhow::{anyhow, Result};

#[derive(Debug)]
pub struct Notification {
    pub path: PathBuf,
    pub elapsed: u64,
}

impl Notification {
    pub fn remove(&self) -> std::io::Result<()> {
        std::fs::remove_file(&self.path)
    }
}

fn get_ns_path() -> PathBuf {
    std::env::temp_dir().join("wsm").join("notify")
}

pub fn notify(path: &Path) -> Result<()> {
    let config = Config::load()?;

    let ws = config
        .get_ws(path)
        .ok_or_else(|| anyhow!("no workspaces for such path: {}", path.display()))?;

    let name = ws.get_name_or_last_path()?;

    info!("notification name is: {}", name);

    let sessions = SessionManager::Tmux(TmuxSessionManager);

    let session_name = get_formatted_session_name(name);
    let attached = sessions.get_attached_session();

    if Some(&session_name) == attached.as_ref() {
        warn!("NOTIFY used in the same attached session, should be ignored: {name}");
        return Ok(());
    }

    let path_buf = get_ns_path();
    std::fs::create_dir_all(&path_buf)?;

    let note_path = path_buf.join(name);
    std::fs::write(note_path, [])?;

    Ok(())
}

pub fn get_notification(ws: &impl WorkspaceName) -> Option<Notification> {
    let Ok(name) = ws.get_name_or_last_path() else {
        return None;
    };

    let note_path = get_ns_path().join(name);

    if !note_path.is_file() {
        return None;
    }

    let Ok(metadata) = std::fs::metadata(&note_path) else {
        return None;
    };

    let Ok(elapsed) = get_elapsed_from_metadata(metadata) else {
        return None;
    };

    Some(Notification {
        path: note_path,
        elapsed: elapsed,
    })
}

fn get_elapsed_from_metadata(metadata: Metadata) -> Result<u64> {
    let created = metadata.created()?;
    Ok(created.elapsed().map(|e| e.as_secs())?)
}
