#[macro_use]
mod commands;
#[macro_use]
mod log;
mod config;
mod format;
mod fzf;
mod ns;
mod selectors;
mod sessions;
mod tmux;
mod workspace;

use crate::{
    commands::{ArgType, Command, CommandDef, ParseError},
    config::Config,
    format::get_workspace_display_items,
    log::init_logger,
    selectors::Selector,
    sessions::{SessionManager, SessionManagerImpl, get_formatted_session_name},
    workspace::{WorkspaceName, WorkspacesBuilder, order_by_notification, order_by_open_session},
};
use anyhow::{Result, anyhow};
use std::path::PathBuf;

fn define_command() -> CommandDef {
    let command = CommandDef::new(
        "wsm",
        "Command line workspace multiplexer, add workspaces to list and switch between them",
    );

    let select = CommandDef::new(
        "select",
        "Select a workspace and switch to tmux session(create + switch)",
    )
    .add_arg(
        "p",
        "print",
        ArgType::Flag,
        "creates workspace and prints name instead of switching",
    );
    let command = command.add_subcommand(select);

    let add = CommandDef::new("add", "Add a workspace to fzf")
        .add_arg(
            "n",
            "name",
            ArgType::Value,
            "Set specific custom name for the workspace",
        )
        .add_arg("o", "open", ArgType::Flag, "open workspace after adding");
    let command = command.add_subcommand(add);

    let remove = CommandDef::new("remove", "remove workspace from fzf by path as default").add_arg(
        "",
        "name",
        ArgType::Value,
        "delete by workspace name",
    );
    let command = command.add_subcommand(remove);

    let ls = CommandDef::new("ls", "list all workspaces added")
        .add_arg(
            "n",
            "notifications",
            ArgType::Flag,
            "toggle showing notifications",
        )
        .add_arg(
            "o",
            "open",
            ArgType::Flag,
            "toggle showing what workspace is open",
        )
        // only open workspaces
        .add_arg("O", "only-open", ArgType::Flag, "only open workspaces");
    let command = command.add_subcommand(ls);

    let notify = CommandDef::new(
        "notify",
        "mark workspace by name and move to the top of selector",
    );
    let command = command.add_subcommand(notify);

    let kill = CommandDef::new("kill", "kill session by workspace name");
    let command = command.add_subcommand(kill);

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
        ["ls"] => handle_ls(&command),
        ["notify"] => handle_notify(&command),
        ["kill"] => handle_kill(&command),
        _ => Err(anyhow!("Command not found")),
    };

    if let Err(e) = cmd_result {
        eprintln!("Error: {:#}", e);
        error!("command can't be executed: {:#}", e);
        return Err(e);
    }

    Ok(())
}

fn handle_kill(command: &Command) -> Result<()> {
    let positional = command.get_positional_string();
    if positional.is_empty() {
        return Err(anyhow!("No session by workspace name"));
    }

    let session_manager = SessionManager::Tmux(tmux::TmuxSessionManager);
    let session_name = get_formatted_session_name(&positional);
    session_manager.kill_session(&session_name)?;

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
    let open = cmd.get_arg("open").is_some();

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

    if open {
        let Some(name) = name.or_else(|| path.file_name().and_then(|os| os.to_str())) else {
            return Err(anyhow!(
                "can't get name from path: {}",
                path.to_string_lossy()
            ));
        };

        let session_manager = SessionManager::Tmux(tmux::TmuxSessionManager);
        let session_name = get_formatted_session_name(name);
        session_manager.new_session(&session_name, &path)?;
        session_manager.switch_client(&session_name)?;
    }
    Ok(())
}

fn handle_ls(command: &Command) -> Result<()> {
    let config = Config::load()?;
    let session_manager = SessionManager::Tmux(tmux::TmuxSessionManager);

    let show_notifications = command.get_arg("notifications").is_some();
    let show_open = command.get_arg("open").is_some();
    let only_open = command.get_arg("only-open").is_some();

    let mut builder = WorkspacesBuilder::new(&config);

    let mut current_session = None;

    if show_open || only_open {
        builder = builder.get_open_sessions(&session_manager);
        current_session = session_manager.get_current_session();
    }

    if show_notifications {
        builder = builder.collect_notifications();
    }

    let mut workspaces = builder.build()?;

    if only_open {
        workspaces = workspaces.into_iter().filter(|w| w.is_open).collect();
    }

    workspaces
        .sort_by(|a, b| order_by_notification(a, b).then_with(|| order_by_open_session(a, b)));

    let display_items = get_workspace_display_items(&workspaces, current_session.as_deref())?;
    for item in display_items {
        println!("{}", item)
    }
    Ok(())
}

fn handle_remove(cmd: &Command) -> Result<()> {
    let by_name = cmd.get_arg("name").is_some();
    let mut config = Config::load()?;

    if by_name {
        let Some(name) = cmd.get_arg_value("name") else {
            return Err(anyhow!("name is required, use --name <name>"));
        };
        config.remove_ws_by_name(name);
        println!("Removed workspace: {}", name);
    } else {
        let positional = cmd.get_positional_string();
        let path = get_path_from_str(&positional)?;
        if (config.has_ws(&path)) == false {
            return Err(anyhow!("workspace does not exist"));
        }
        config.remove_ws(&path);
        println!("Removed workspace: {}", path.display());
    }
    config.save()?;

    Ok(())
}

fn handle_ws_select(cmd: &Command) -> Result<()> {
    let only_print_session_name = cmd.get_arg("print").is_some();

    let config = Config::load()?;
    let sessions = SessionManager::Tmux(tmux::TmuxSessionManager);

    let mut workspaces = WorkspacesBuilder::new(&config)
        .get_open_sessions(&sessions)
        .collect_notifications()
        .build()?;

    workspaces
        .sort_by(|a, b| order_by_notification(a, b).then_with(|| order_by_open_session(a, b)));

    let selector = Selector::Fzf(fzf::FzfSelector);

    let current_session = sessions.get_current_session();

    let Some(ws) = selector.select(&workspaces, current_session.as_deref())? else {
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
