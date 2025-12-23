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

Add the current directory as a workspace:

```sh
wsm add
```

Add a specific directory:

```sh
wsm add /path/to/project
```

Remove a workspace (current directory by default):

```sh
wsm remove
```

List workspaces:

```sh
wsm ls
```

Select a workspace and jump to its tmux session:

```sh
wsm
```

## How selection works

- `wsm` launches `fzf` with a preview pane.
- If a tmux session for the selected directory already exists, the preview shows
  recent window output for each window.
- If no session exists, the preview shows a colored directory listing.
- The session name is the selected directory's basename with `.` replaced by `_`.
- If you're already inside tmux, `wsm` switches the current client to the session.
  Otherwise it starts the session in the foreground.

## Config file

Workspaces are stored in:

```
~/.config/wsm/config.json
```

Example:

```json
{
  "workspaces": [
    { "path": "/Users/you/projects/app" }
  ]
}
```

You can edit this file by hand if needed.
