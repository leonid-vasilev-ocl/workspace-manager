use anyhow::{Context, Result};
use std::{
    io::Write,
    process::{Command, Stdio},
};

use crate::{format::get_workspace_display_items, selectors::SelectorImpl, workspace::Workspace};

pub struct FzfSelector;

impl SelectorImpl for FzfSelector {
    fn select<'a>(&self, workspaces: &'a [Workspace]) -> Result<Option<&'a Workspace>> {
        Ok(call_fzf_with_workspaces(workspaces)?)
    }
}

fn add_reload_command(command: &mut Command) {
    command.arg("--bind=ctrl-r:reload(wsm ls)");
}

fn call_fzf_with_workspaces(workspaces: &[Workspace]) -> Result<Option<&Workspace>> {
    let mut command = Command::new("fzf");

    command
        .arg("--ansi")
        .arg("--delimiter=\t")
        .arg("--with-nth=2..")
        .arg("--layout=reverse") // Puts the input at the top
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());

    add_reload_command(&mut command);

    let mut child = command.spawn()?;

    let input = get_workspace_display_items(workspaces)?.join("\n");

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
