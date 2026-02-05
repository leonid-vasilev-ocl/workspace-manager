use crate::config::Workspace;
use crate::fzf::FzfSelector;
use anyhow::Result;

pub enum Selector {
    Fzf(FzfSelector),
}

pub trait SelectorImpl {
    fn select<'a>(&self, workspaces: &'a [Workspace]) -> Result<Option<&'a Workspace>>;
}

impl Selector {
    pub fn select<'a>(&self, workspaces: &'a [Workspace]) -> Result<Option<&'a Workspace>> {
        match self {
            Selector::Fzf(sel) => sel.select(workspaces),
        }
    }
}
