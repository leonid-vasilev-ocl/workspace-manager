mod config;
mod fzf;
mod tmux;

use crate::config::Config;
use anyhow::{Ok, Result, anyhow};
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Err(err) = handle_app(args) {
        eprintln!("error: {}", err);
        std::process::exit(1)
    }

    std::process::exit(0)
}

fn handle_app(args: Vec<String>) -> Result<()> {
    if args.len() <= 1 {
        handle_ws_select(false)?
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
        "-p" => handle_ws_select(true)?,
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

// print session name instead of switch_client
fn handle_ws_select(only_print_session_name: bool) -> Result<()> {
    let config = Config::load()?;
    let workspaces = config.get_ws_all();

    let session_path = fzf::call_fzf_with_workspaces(workspaces)?;

    if session_path.is_empty() {
        return Ok(());
    }

    let session_name = get_session_name(&session_path);

    let is_in_tmux = tmux::is_in_tmux();
    let attach_to_tmux_external = !is_in_tmux && !only_print_session_name;
    let attach_to_tmux_from_tmux = is_in_tmux && !only_print_session_name;

    if is_in_tmux && tmux::is_same_tmux_session(&session_name) {
        return Ok(());
    }

    tmux::new_session(&session_name, &session_path, attach_to_tmux_external)?;
    if attach_to_tmux_from_tmux {
        tmux::switch_client(&session_name)?;
    }

    if only_print_session_name {
        println!("{}", &session_name)
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
