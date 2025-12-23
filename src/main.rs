mod config;
mod fzf;

use crate::config::Config;
use anyhow::{Context, Ok, Result, anyhow};
use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Err(err) = handle_app(args) {
        eprintln!("error: {}", err);
        std::process::exit(1)
    }
}

fn handle_app(args: Vec<String>) -> Result<()> {
    if args.len() <= 1 {
        handle_ws_select()?
    } else {
        handle_ws_modify(args)?
    }

    Ok(())
}

fn handle_ws_modify(args: Vec<String>) -> Result<()> {
    let main_arg = &args[1];

    match main_arg.as_str() {
        "add" => handle_add(args)?,
        "remove" => handle_remove(args)?,
        "ls" => handle_ls()?,
        _ => {
            return Err(anyhow!(
                "unknown argument: {}. Use (add, remove or ls)",
                main_arg
            ));
        }
    };

    Ok(())
}

fn get_path_from_args(args: Vec<String>) -> Result<PathBuf> {
    let path = match args.len() {
        2 => std::env::current_dir()?,
        3 => PathBuf::from(&args[2]).canonicalize()?,
        _ => Err(anyhow!("too many arguments"))?,
    };

    if !path.is_dir() {
        Err(anyhow!("path is not a directory"))?;
    }

    Ok(path)
}

fn handle_add(args: Vec<String>) -> Result<()> {
    let path = get_path_from_args(args)?;
    let mut config = Config::load()?;

    if config.has_ws(&path) {
        return Err(anyhow!("workspace already exists"));
    }

    config.add_ws(&path);
    config.save()?;

    println!("Added workspace: {}", path.display());
    Ok(())
}

fn handle_ls() -> Result<()> {
    let config = Config::load()?;
    let workspaces = config.get_ws_all();
    for ws in workspaces {
        println!("{}", ws.path.display())
    }
    Ok(())
}

fn handle_remove(args: Vec<String>) -> Result<()> {
    let path = get_path_from_args(args)?;
    let mut config = Config::load()?;

    if (config.has_ws(&path)) == false {
        return Err(anyhow!("workspace does not exist"));
    }

    config.remove_ws(&path);
    config.save()?;

    println!("Removed workspace: {}", path.display());
    Ok(())
}

fn handle_ws_select() -> Result<()> {
    let config = Config::load()?;
    let workspaces = config.get_ws_all();
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

    let output = child.wait_with_output().expect("can't get output from fzf");

    let selected = String::from_utf8_lossy(&output.stdout)
        .trim()
        .split_once(" ")
        .unwrap_or_default()
        .1
        .to_string();

    if selected.is_empty() {
        return Ok(());
    }

    let session_name = get_session_name(&selected);

    let is_in_tmux = std::env::var("TMUX").is_ok();

    let mut tmux_command = Command::new("tmux");
    tmux_command
        .arg("new-session")
        .arg("-A")
        .arg("-s")
        .arg(&session_name)
        .arg("-c")
        .arg(&selected);

    if is_in_tmux {
        tmux_command.arg("-d").status()?;

        Command::new("tmux")
            .arg("switch-client")
            .arg("-t")
            .arg(&session_name)
            .status()?;
    } else {
        tmux_command.status()?;
    }

    Ok(())
}

fn get_session_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .replace(".", "_")
}
