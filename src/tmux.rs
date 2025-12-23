use std::process::Command;

use anyhow::Result;

pub fn go_to_tmux(session_name: &str, session_path: &str) -> Result<()> {
    let is_in_tmux = is_in_tmux();

    let mut tmux_command = Command::new("tmux");
    tmux_command
        .arg("new-session")
        .arg("-A")
        .arg("-s")
        .arg(&session_name)
        .arg("-c")
        .arg(&session_path);

    if is_in_tmux {
        if is_same_tmux_session(session_name) {
            println!("same session. return");
            return Ok(());
        }

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

fn is_same_tmux_session(session_name: &str) -> bool {
    let current_session = Command::new("tmux")
        .arg("display-message")
        .arg("-p")
        .arg("#S")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    Some(session_name) == current_session.as_deref()
}

fn is_in_tmux() -> bool {
    std::env::var("TMUX").is_ok()
}
