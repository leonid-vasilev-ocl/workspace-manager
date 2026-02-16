use anyhow::{Context, Result};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

use crate::{
    ns::Notification,
    selectors::SelectorImpl,
    workspace::{Workspace, WorkspaceName},
};

pub struct FzfSelector;

impl SelectorImpl for FzfSelector {
    fn select<'a>(&self, workspaces: &'a [Workspace]) -> Result<Option<&'a Workspace>> {
        Ok(call_fzf_with_workspaces(workspaces)?)
    }
}

fn style_text(text: &str, bold: bool, color_code: Option<u8>) -> String {
    let mut parts: Vec<String> = Vec::new();

    if bold {
        parts.push("1".to_string())
    }

    if let Some(color) = color_code {
        parts.push(color.to_string())
    }

    if parts.is_empty() {
        text.to_string()
    } else {
        format!("\x1b[{}m{}\x1b[0m", parts.join(";"), text)
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

fn get_select_display_item(
    i: usize,
    name: &str,
    path: &Path,
    notification: Option<&Notification>,
    max_name_len: usize,
) -> Result<String> {
    let static_padding = 8;
    let left_padding = max_name_len + static_padding;
    let st = format!(
        "{}\t{:width$} {}",
        i,
        name,
        path.to_string_lossy(),
        width = left_padding
    );
    if notification.is_some() {
        Ok(style_text(&st, true, Some(33)))
    } else {
        Ok(st)
    }
}

fn call_fzf_with_workspaces(workspaces: &[Workspace]) -> Result<Option<&Workspace>> {
    let mut child = Command::new("fzf")
        .arg("--ansi")
        .arg("--delimiter=\t")
        .arg("--with-nth=2..")
        .arg("--layout=reverse") // Puts the input at the top
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let name_path = get_name_path(&workspaces)?;
    let mut name_max_len: usize = 0;

    for (name, _) in &name_path {
        let len = name.len();
        if len > name_max_len {
            name_max_len = len;
        }
    }

    let input = &name_path
        .iter()
        .enumerate()
        .map(|(i, (name, path))| {
            get_select_display_item(
                i,
                name,
                path,
                workspaces[i].notification.as_ref(),
                name_max_len,
            )
        })
        .collect::<Result<Vec<String>>>()?
        .join("\n");

    {
        let mut stdin = child.stdin.take().context("Failed to open fzf stdin")?;
        stdin.write_all(input.as_bytes())?;
    }

    let output = child
        .wait_with_output()
        .context("can't get output from fzf")?;

    let workspace = String::from_utf8_lossy(&output.stdout)
        .trim()
        .split_once("\t")
        .and_then(|(first, _)| first.parse::<usize>().ok())
        .and_then(|index| workspaces.get(index));

    Ok(workspace)
}
