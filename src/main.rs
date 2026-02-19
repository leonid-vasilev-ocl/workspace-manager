#[macro_use]
mod commands;
#[macro_use]
mod log;
mod config;
mod fzf;
mod ns;
mod selectors;
mod sessions;
mod tmux;
mod workspace;

use crate::{
    commands::{ArgType, Command, CommandDef, ParseError},
    config::Config,
    log::init_logger,
    selectors::Selector,
    sessions::{SessionManager, SessionManagerImpl, get_formatted_session_name},
    workspace::{Workspace, WorkspaceName},
};
use anyhow::{Result, anyhow};
use std::{collections::HashSet, path::PathBuf};

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

    let notify = CommandDef::new(
        "notify",
        "mark workspace by name and move to the top of selector",
    );
    let command = command.add_subcommand(notify);

    command
}

fn handle_command() -> Result<()> {
    init_logger()?;

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
                error!("can't parse command: {:#}", err)
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
        ["notify"] => handle_notify(&command),
        _ => Err(anyhow!("Command not found")),
    };

    if let Err(e) = cmd_result {
        eprintln!("Error: {:#}", e);
        error!("command can't be executed: {:#}", e);
        return Err(e);
    }

    Ok(())
}

fn handle_notify(command: &Command) -> Result<()> {
    let path = get_path_from_str(&command.get_positional_string())?;
    ns::notify(path.as_path())?;
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

    let name = cmd.get_arg_value("name");

    let mut config = Config::load()?;

    if config.has_ws(&path) {
        return Err(anyhow!("workspace already exists"));
    }

    config.add_ws(&path, name.map(|s| s.to_string()));
    config.save()?;

    println!(
        "Added workspace: {} {}",
        path.display(),
        match name {
            Some(val) => format!("with name: {}", val),
            None => String::from(""),
        }
    );
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

fn handle_ws_select(cmd: &Command) -> Result<()> {
    let only_print_session_name = cmd.get_arg("print").is_some();

    let config = Config::load()?;

    let mut workspaces: Vec<Workspace> = config
        .take_ws_all()
        .into_iter()
        .map(Workspace::from)
        .collect();

    let sessions = SessionManager::Tmux(tmux::TmuxSessionManager);
    let active_sessions_names: HashSet<String> =
        sessions.list_active_sessions()?.into_iter().collect();
    for ws in workspaces.iter_mut() {
        let ws_name = ws.get_name_or_last_path()?;
        ws.is_open = active_sessions_names.contains(ws_name);
    }

    //TODO: move to the separate method
    workspaces.sort_by(|a, b| {
        let has_a = a.notification.is_some();
        let has_b = b.notification.is_some();

        let elapsed_a = a
            .notification
            .as_ref()
            .map(|n| n.elapsed)
            .unwrap_or(u64::MAX);

        let elapsed_b = b
            .notification
            .as_ref()
            .map(|n| n.elapsed)
            .unwrap_or(u64::MAX);

        let open_a = a.is_open;
        let open_b = b.is_open;

        has_b
            .cmp(&has_a)
            .then_with(|| elapsed_b.cmp(&elapsed_a))
            .then_with(|| open_b.cmp(&open_a))
    });

    let selector = Selector::Fzf(fzf::FzfSelector);

    let Some(ws) = selector.select(&workspaces)? else {
        return Ok(());
    };

    if let Some(ns) = &ws.notification {
        ns.remove()?;
    }

    let session_path = ws.as_ref();

    let session_name = get_formatted_session_name(ws.get_name_or_last_path()?);

    if sessions.is_same_session(&session_name) {
        return Ok(());
    }

    if !sessions.has_session(&session_name)? {
        sessions.new_session(&session_name, session_path)?;
    }

    if !only_print_session_name {
        sessions.switch_client(&session_name)?;
    }

    if only_print_session_name {
        println!("{}", &session_name)
    }

    Ok(())
}
