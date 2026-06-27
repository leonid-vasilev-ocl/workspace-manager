use std::{
    collections::HashSet,
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

fn get_running_path() -> PathBuf {
    std::env::temp_dir().join("wsm").join("running")
}

/// Identifies one instance within a workspace. tmux gives each pane a stable id
/// ($TMUX_PANE, e.g. "%25"); two instances in one workspace run in different
/// panes, so they get separate markers.
fn instance_key() -> String {
    std::env::var("TMUX_PANE")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "default".to_string())
}

/// Remove this instance's running marker (it stopped or paused for input).
fn end_running(name: &str) {
    let dir = get_running_path().join(name);
    let _ = std::fs::remove_file(dir.join(instance_key()));
    let _ = std::fs::remove_dir(&dir); // best-effort; only removes if now empty
}

pub fn notify(path: &Path) -> Result<()> {
    let config = Config::load()?;

    let ws = config
        .get_ws(path)
        .ok_or_else(|| anyhow!("no workspaces for such path: {}", path.display()))?;

    let name = ws.get_name_or_last_path()?;

    info!("notification name is: {}", name);

    end_running(name); // this instance stopped/paused -> drop its running marker

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

/// `wsm running <path>`: mark THIS instance (tmux pane) as running in the
/// workspace. Multiple instances each get their own marker; the workspace stays
/// "running" while any live one remains. No attached-session suppression —
/// running is informational, not an alert, and does not touch the notify bell.
pub fn running(path: &Path) -> Result<()> {
    let config = Config::load()?;

    let ws = config
        .get_ws(path)
        .ok_or_else(|| anyhow!("no workspaces for such path: {}", path.display()))?;

    let name = ws.get_name_or_last_path()?;

    let dir = get_running_path().join(name);
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(instance_key()), [])?;

    Ok(())
}

/// A workspace is running if any of its instance markers belongs to a live pane.
/// `live_panes` = Some(current pane ids) prunes stale markers (instances whose
/// pane closed without a Stop hook); None skips validation (just presence).
pub fn is_running(ws: &impl WorkspaceName, live_panes: Option<&HashSet<String>>) -> bool {
    let Ok(name) = ws.get_name_or_last_path() else {
        return false;
    };

    let dir = get_running_path().join(name);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return false;
    };

    let mut running = false;
    for entry in entries.flatten() {
        let key = entry.file_name().to_string_lossy().to_string();
        match live_panes {
            Some(panes) if key != "default" && !panes.contains(&key) => {
                let _ = std::fs::remove_file(entry.path()); // prune dead instance
            }
            _ => running = true,
        }
    }
    running
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
