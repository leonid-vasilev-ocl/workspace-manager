use anyhow::{Context, Result};
use std::{
    io::Write,
    process::{Command, Stdio},
};

use crate::config::Workspace;

pub fn call_fzf_with_workspaces(workspaces: &[Workspace]) -> Result<Option<&Workspace>> {
    let mut child = Command::new("fzf")
        .arg("--layout=reverse") // Puts the input at the top
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
        .arg("hidden")
        .arg("--bind")
        .arg("ctrl-t:toggle-preview")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let input = workspaces
        .iter()
        .enumerate()
        .map(|(i, ws)| {
            let name = match &ws.name {
                Some(n) => n,
                None => ws
                    .path
                    .file_name()
                    .and_then(|os| os.to_str())
                    .with_context(|| {
                        format!("can not get name for path: {}", ws.path.to_string_lossy())
                    })?,
            };
            Ok(format!("{} {} {}", i, name, ws.path.to_string_lossy()))
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
        .split_once(" ")
        .and_then(|(first, _)| first.parse::<usize>().ok())
        .and_then(|index| workspaces.get(index));

    Ok(workspace)
}
