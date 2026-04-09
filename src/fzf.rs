use anyhow::{Context, Result};
use std::{
    io::Write,
    process::{Command, Stdio},
};

use crate::{
    format::get_workspace_display_items,
    selectors::SelectorImpl,
    workspace::{Workspace, WorkspaceName},
};

pub struct FzfSelector;

impl SelectorImpl for FzfSelector {
    fn select<'a>(
        &self,
        workspaces: &'a [Workspace],
        current_session: Option<&'a str>,
    ) -> Result<Option<&'a Workspace>> {
        Ok(call_fzf_with_workspaces(workspaces, current_session)?)
    }
}

fn add_reload_command(command: &mut Command) {
    command.arg("--bind=ctrl-r:reload(wsm ls -o -n)");
}

fn add_kill_session_command(command: &mut Command) {
    command.arg("--bind=ctrl-x:execute-silent(wsm kill {2})+reload-sync(wsm ls -o -n)");
}

trait FzfCommand {
    fn add_help(&mut self) -> &mut Self;
}

impl FzfCommand for Command {
    fn add_help(&mut self) -> &mut Self {
        self.arg("--footer")
            .arg(
                "enter: select | ctrl-r: reload | ctrl-x: kill session | space: jump | esc: cancel",
            )
            .arg("--footer-border");

        self
    }
}

fn call_fzf_with_workspaces<'a>(
    workspaces: &'a [Workspace],
    current_session: Option<&'a str>,
) -> Result<Option<&'a Workspace>> {
    let mut command = Command::new("fzf");

    command
        .arg("--ansi")
        .arg("--delimiter=\t")
        .arg("--with-nth=2..")
        .arg("--layout=reverse") // Puts the input at the top
        .arg("--bind=space:jump,jump:accept")
        .add_help()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());

    add_reload_command(&mut command);
    add_kill_session_command(&mut command);

    let mut child = command.spawn()?;

    let input = get_workspace_display_items(workspaces, current_session)?.join("\n");

    {
        let mut stdin = child.stdin.take().context("Failed to open fzf stdin")?;
        stdin.write_all(input.as_bytes())?;
    }

    let output = child
        .wait_with_output()
        .context("can't get output from fzf")?;

    let workspace = String::from_utf8_lossy(&output.stdout)
        .trim()
        .split("\t")
        .nth(1)
        .map(|s| s.trim())
        .and_then(|s| {
            workspaces.iter().find(|w| match w.get_name_or_last_path() {
                Ok(name) => name == s,
                Err(_) => false,
            })
        });

    Ok(workspace)
}
