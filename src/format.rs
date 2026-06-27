use std::path::Path;

use crate::{
    ns::Notification,
    workspace::{Workspace, WorkspaceName},
};
use anyhow::Result;

//TODO: make this configurable via config and builder
pub fn get_workspace_display_items(
    wss: &[Workspace],
    current_session: Option<&str>,
) -> Result<Vec<String>> {
    let name_path = get_name_path(wss)?;

    let mut name_max_len: usize = 0;

    for (name, _) in &name_path {
        let len = name.len();
        if len > name_max_len {
            name_max_len = len;
        }
    }

    name_path
        .iter()
        .enumerate()
        .map(|(i, (name, path))| {
            get_select_display_item(
                i,
                name,
                path,
                wss[i].is_open,
                wss[i].is_running,
                current_session,
                wss[i].notification.as_ref(),
                name_max_len,
            )
        })
        .collect()
}

fn get_select_display_item(
    i: usize,
    name: &str,
    path: &Path,
    is_open: bool,
    is_running: bool,
    current_session: Option<&str>,
    notification: Option<&Notification>,
    max_name_len: usize,
) -> Result<String> {
    let path = path.to_string_lossy();

    // open-session icon — original color (untouched; do not repurpose it)
    let right_text = if is_open {
        let open_label = format!("{:width$}", "\u{ebc8}", width = 4);

        let color = match current_session {
            Some(session) if session == name => Some(Color::Basic(33)),
            _ => Some(Color::Basic(32)),
        };

        format!("{}{}", style_text(&open_label, true, color), path)
    } else {
        format!("{:width$}{}", "", path, width = 4)
    };

    // separate "running" icon (green), independent of the open-session icon
    let right_text = if is_running {
        let running_label = format!("{:width$}", "\u{f111}", width = 3);
        format!(
            "{}{}",
            style_text(&running_label, true, Some(Color::Ansi256(114))),
            right_text,
        )
    } else {
        format!("{:width$}{}", "", right_text, width = 3)
    };

    let right_text = if notification.is_some() {
        let notification_label = format!("{:width$}", "\u{f06a}", width = 3);
        format!(
            "{}{}",
            style_text(&notification_label, true, Some(Color::Basic(33))),
            right_text,
        )
    } else {
        format!("{:width$}{}", "", right_text, width = 3)
    };

    let st = format!(
        "{}\t{:width$}\t{}",
        i,
        name,
        right_text,
        width = max_name_len + 10
    );

    if notification.is_some() {
        Ok(style_text(&st, true, Some(Color::Basic(33))))
    } else {
        Ok(st)
    }
}

fn get_name_path(wss: &[Workspace]) -> Result<Vec<(&str, &Path)>> {
    let mut res: Vec<(&str, &Path)> = Vec::new();

    for ws in wss {
        let name = ws.get_name_or_last_path()?;
        res.push((name, ws.path.as_ref()))
    }

    Ok(res)
}

#[derive(Clone, Copy)]
enum Color {
    Basic(u8),
    Ansi256(u8),
}

fn style_text(text: &str, bold: bool, color: Option<Color>) -> String {
    let mut parts: Vec<String> = Vec::new();

    if bold {
        parts.push("1".to_string())
    }

    match color {
        Some(Color::Basic(n)) => parts.push(n.to_string()),
        Some(Color::Ansi256(n)) => {
            parts.push("38".to_string());
            parts.push("5".to_string());
            parts.push(n.to_string());
        }
        None => {}
    }

    if parts.is_empty() {
        text.to_string()
    } else {
        format!("\x1b[{}m{}\x1b[0m", parts.join(";"), text)
    }
}
