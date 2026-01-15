mod config;
mod fzf;
mod tmux;

use crate::config::Config;
use anyhow::{Context, Ok, Result, anyhow};
use std::path::{Path, PathBuf};

const HELP_TEXT: &str = r"
usage: wsm [command]
commands:
    select [-p]         Select a workspace (default command). Use -p to only println
    add [path]          Add a workspace (default path is current directory)
    remove [path]       Remove a workspace (default path is current directory)
    ls                  List all workspaces
    ";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Err(err) = handle_app(&args) {
        eprintln!("error: {:#} \n{}", err, HELP_TEXT);
        std::process::exit(1)
    }

    std::process::exit(0)
}

fn handle_app(args: &[String]) -> Result<()> {
    handle_ws_commands(&args[1..])
}

fn handle_ws_commands(args: &[String]) -> Result<()> {
    let (main_arg, rest_args) = args
        .split_first()
        .with_context(|| anyhow!("missing argument"))?;

    match main_arg.as_str() {
        "select" => handle_ws_select(rest_args)?,
        "add" => handle_add(rest_args)?,
        "remove" => handle_remove(rest_args)?,
        "ls" => handle_ls()?,
        "help" => {
            println!("{}", HELP_TEXT);
        }
        _ => {
            return Err(anyhow!("unknown command: {}", main_arg));
        }
    };

    Ok(())
}

fn get_path_from_args(args: &[String]) -> Result<PathBuf> {
    let path = match args.len() {
        0 => std::env::current_dir()?,
        1 => PathBuf::from(&args[0]).canonicalize()?,
        _ => Err(anyhow!("too many arguments"))?,
    };

    if !path.is_dir() {
        Err(anyhow!("path is not a directory"))?;
    }

    Ok(path)
}

fn handle_add(args: &[String]) -> Result<()> {
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

fn handle_remove(args: &[String]) -> Result<()> {
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

// print session name instead of switch_client
fn handle_ws_select(args: &[String]) -> Result<()> {
    let only_print_session_name = args.contains(&"-p".to_string());

    let config = Config::load()?;
    let workspaces = config.get_ws_all();

    let session_path = match fzf::call_fzf_with_workspaces(workspaces)? {
        Some(p) => p.as_ref(),
        None => return Ok(()),
    };

    let session_name = get_session_name(session_path);

    let is_in_tmux = tmux::is_in_tmux();

    let attach_to_tmux_external = !is_in_tmux && !only_print_session_name;
    let attach_to_tmux_from_tmux = is_in_tmux && !only_print_session_name;

    if is_in_tmux && tmux::is_same_tmux_session(&session_name) {
        return Ok(());
    }

    if !is_in_tmux || !tmux::has_session(&session_name)? {
        tmux::new_session(&session_name, session_path, attach_to_tmux_external)?;
    }

    if attach_to_tmux_from_tmux {
        tmux::switch_client(&session_name)?;
    }

    if only_print_session_name {
        println!("{}", &session_name)
    }

    Ok(())
}

fn get_session_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .replace(".", "_")
}
