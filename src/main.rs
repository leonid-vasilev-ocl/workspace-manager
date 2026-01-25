mod commands;
mod config;
mod fzf;
mod tmux;

use crate::{
    commands::{ArgType, Command, CommandDef, ParseError},
    config::Config,
};
use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

fn define_command() -> CommandDef {
    let command = CommandDef::new(
        "wsm",
        "Command line workspace multiplexer, add workspaces to list and swtitch between them using fzf and tmux",
    );

    let select = CommandDef::new(
        "select",
        "Select a workspace in fzf and switch to tmux session(create + switch)",
    )
    .add_arg(
        "p",
        "print",
        ArgType::Flag,
        "creates tmux workspace and prints name instead of switching",
    );
    let command = command.add_subcommand(select);

    let add = CommandDef::new("add", "Add a workspace to fzf").add_arg(
        "n",
        "name",
        ArgType::Value,
        "Set specific custom name for the workspace",
    );
    let command = command.add_subcommand(add);

    let remove = CommandDef::new("remove", "remove workspace from fzf");
    let command = command.add_subcommand(remove);

    let ls = CommandDef::new("ls", "list all workspaces added");
    let command = command.add_subcommand(ls);

    command
}

fn handle_command() -> Result<()> {
    let command_def = define_command();
    let command = match command_def.parse(std::env::args()) {
        Err(err) => {
            let path = match &err {
                ParseError::UnknownCommand { path, name: _ } => path,
                ParseError::UnknownArg { path, name: _ } => path,
                ParseError::MissingArgValue { path, name: _ } => path,
                ParseError::UnexpectedArgValue { path, name: _ } => path,
                ParseError::MissingValue { path, name: _ } => path,
                ParseError::HelpRequested { path } => path,
            };
            if let ParseError::HelpRequested { path: _ } = err {
                eprintln!("{}", command_def.get_help(path));
            } else {
                eprintln!("{:#} \n{}", err, command_def.get_help(path));
            }
            return Err(anyhow!(err));
        }
        Ok(command) => anyhow::Ok(command),
    }?;

    let path = &command.get_path()[1..];

    let cmd_result = match path {
        ["select"] => handle_ws_select(&command),
        ["add"] => handle_add(&command),
        ["remove"] => handle_remove(&command),
        ["ls"] => handle_ls(),
        _ => Err(anyhow!("Command not found")),
    };

    if let Err(ref e) = cmd_result {
        eprintln!("Error: {:#}", e);
    }

    Ok(())
}

fn main() {
    if let Err(_) = handle_command() {
        std::process::exit(1)
    }

    std::process::exit(0)
}

fn get_path_from_str(val: &str) -> Result<PathBuf> {
    let path = match val {
        "" => std::env::current_dir()?,
        _ => PathBuf::from(val).canonicalize()?,
    };

    if !path.is_dir() {
        Err(anyhow!("path is not a directory"))?;
    }

    Ok(path)
}

fn handle_add(cmd: &Command) -> Result<()> {
    let positional = cmd.get_positional_string();
    let path = get_path_from_str(&positional)?;
    let mut config = Config::load()?;

    if config.has_ws(&path) {
        return Err(anyhow!("workspace already exists"));
    }

    config.add_ws(&path, None);
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

fn handle_remove(cmd: &Command) -> Result<()> {
    let positional = cmd.get_positional_string();
    let path = get_path_from_str(&positional)?;
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
fn handle_ws_select(cmd: &Command) -> Result<()> {
    let only_print_session_name = cmd.get_arg("print").is_some();
    println!(
        "Command: {:?}, print_session_name: {}",
        cmd, only_print_session_name
    );

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
