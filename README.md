# Workspace Manager (wsm)

`wsm` is a small Rust CLI that keeps a list of project directories and lets you jump
into them with `fzf`, opening or switching to a matching `tmux` session.

## Requirements

- Rust toolchain with 2024 edition support
- `tmux`
- `fzf`

## Install

```sh
cargo install --path .
```

This installs the binary as `wsm` in your Cargo bin directory.

## Usage

`wsm` uses explicit subcommands (there is no implicit default action).

Select a workspace and jump to its tmux session:

```sh
wsm select
```

Select a workspace but only print the tmux session name:

```sh
wsm select --print
```

Add the current directory as a workspace:

```sh
wsm add
```

Add a specific directory:

```sh
wsm add /path/to/project
```

Add a workspace with a custom display name:

```sh
wsm add --name my-app /path/to/project
```

Remove a workspace (current directory by default):

```sh
wsm remove
```

List workspaces:

```sh
wsm ls
```

Mark a workspace as notified (current directory by default):

```sh
wsm notify
```

## How selection works

- `wsm select` launches `fzf` in reverse layout.
- Selector rows are built as `name + padded spacing + absolute path`.
- The internal item index is hidden from the UI and used only for selection mapping.
- Workspaces with notifications are highlighted in bold yellow.
- Notified workspaces are sorted to the top before `fzf` is shown.
- The session name is the selected directory's basename with `.` replaced by `_`.
- If you select a notified workspace, its notification marker is removed.
- If the selected session already matches the current tmux session, `wsm` exits.
- If the session does not exist, `wsm` creates it.
- Unless `--print` is used, `wsm` switches the current tmux client to the selected session.

## Notify workflow

- `wsm notify [path]` marks a workspace so it is prioritized in the selector.
- The target path must already exist in `wsm` config.
- Notification marker files are written under:

```
<temp_dir>/wsm/notify/<workspace-name>
```

`<workspace-name>` is the configured workspace name if present; otherwise it is the
workspace directory basename. `<temp_dir>` is the platform temp directory used by
Rust (`std::env::temp_dir()`).

## Logging

- `wsm` writes runtime logs to:

```
/tmp/wsm/log.txt
```

## Config file

Workspaces are stored in:

```
~/.config/wsm/config.json
```

Example:

```json
{
  "workspaces": [
    { "name": "my-app", "path": "/Users/you/projects/app" },
    { "path": "/Users/you/projects/another-app" }
  ]
}
```

You can edit this file by hand if needed.
