use anyhow::{Context, Result};
use std::{
    io::Write,
    process::{Command, Stdio},
};

use crate::config::Workspace;

pub fn call_fzf_with_workspaces(workspaces: &[Workspace]) -> Result<String> {
    let mut child = Command::new("fzf")
        .arg("--preview")
        .arg(
            "sh -c '
    sess=$(basename {2..}); 
    if tmux has-session -t \"$sess\" 2>/dev/null; then
        tmux list-windows -t \"$sess\" -F \"#I:#W\" | while read -r line; do
            index=$(echo $line | cut -d: -f1);
            name=$(echo $line | cut -d: -f2);
            printf \"\\033[32m── Window $index: $name ──\\033[0m\\n\";
            tmux capture-pane -pt \"$sess:$index\" -eS -5 -E 10 | sed \"s/^/  /\";
            echo \"\";
        done;
    else
        printf \"\\033[33m--- Session Not Active ---\\033[0m\\n\";
        ls -p --color=always {2..};
    fi
'",
        )
        .arg("--preview-window")
        .arg("right:65%:border-left")
        .arg("--layout=reverse") // Puts the input at the top
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let input = workspaces
        .iter()
        .enumerate()
        .map(|(i, ws)| format!("{} {}", i + 1, ws.path.to_string_lossy()))
        .collect::<Vec<_>>()
        .join("\n");

    {
        let mut stdin = child.stdin.take().context("Failed to open fzf stdin")?;
        stdin.write_all(input.as_bytes())?;
    }

    let output = child
        .wait_with_output()
        .context("can't get output from fzf")?;

    let selected = String::from_utf8_lossy(&output.stdout)
        .trim()
        .split_once(" ")
        .unwrap_or_default()
        .1
        .to_string();

    Ok(selected)
}
