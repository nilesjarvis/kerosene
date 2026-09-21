use crate::network_activity::{ActivityEntry, ActivitySnapshot, Provider};
use iced::{widget::Id, window};
use std::fmt;

pub(crate) const PAGE_SIZE: usize = 150;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ConsoleFilter {
    #[default]
    All,
    Http,
    WebSocket,
    Errors,
}

impl ConsoleFilter {
    pub(crate) const ALL: [Self; 4] = [Self::All, Self::Http, Self::WebSocket, Self::Errors];

    fn matches(self, entry: &ActivityEntry) -> bool {
        match self {
            Self::All => true,
            Self::Http => entry.kind.is_http(),
            Self::WebSocket => !entry.kind.is_http(),
            Self::Errors => entry.kind.is_error(),
        }
    }
}

impl fmt::Display for ConsoleFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::All => "All activity",
            Self::Http => "HTTP",
            Self::WebSocket => "WebSocket",
            Self::Errors => "Errors",
        })
    }
}

#[derive(Debug, Default)]
pub(crate) struct ConsoleState {
    pub(crate) window_id: Option<window::Id>,
    pub(crate) snapshot: ActivitySnapshot,
    pub(crate) provider: Provider,
    pub(crate) filter: ConsoleFilter,
    pub(crate) paused: bool,
    pub(crate) page: usize,
    pub(crate) cleared_through: u64,
}

impl ConsoleState {
    pub(crate) fn entries(&self) -> impl Iterator<Item = &ActivityEntry> {
        self.snapshot.entries.iter().rev().filter(|entry| {
            entry.sequence > self.cleared_through
                && (self.provider == Provider::All || self.provider == entry.provider)
                && self.filter.matches(entry)
        })
    }

    pub(crate) fn refresh(&mut self) {
        if !self.paused {
            self.snapshot = crate::network_activity::snapshot();
        }
    }
}

pub(crate) fn scroll_id() -> Id {
    Id::new("network-console")
}

#[cfg(test)]
mod tests;
