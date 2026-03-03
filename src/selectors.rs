use crate::fzf::FzfSelector;
use crate::workspace::Workspace;
use anyhow::Result;

pub enum Selector {
    Fzf(FzfSelector),
}

pub trait SelectorImpl {
    fn select<'a>(
        &self,
        workspaces: &'a [Workspace],
        current_session: Option<&'a str>,
    ) -> Result<Option<&'a Workspace>>;
}

impl Selector {
    pub fn select<'a>(
        &self,
        workspaces: &'a [Workspace],
        current_session: Option<&'a str>,
    ) -> Result<Option<&'a Workspace>> {
        match self {
            Selector::Fzf(sel) => sel.select(workspaces, current_session),
        }
    }
}
