use std::fmt;

// ---------------------------------------------------------------------------
// PnL Card State
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum PnlCardTarget {
    Position(String),
    Summary,
}

impl fmt::Debug for PnlCardTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Position(_) => f.write_str("Position(<redacted>)"),
            Self::Summary => f.write_str("Summary"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PnlCardDisplayMode {
    PercentOnly,
    UsdOnly,
    Both,
}

impl PnlCardDisplayMode {
    pub(super) const ALL: [Self; 3] = [Self::PercentOnly, Self::UsdOnly, Self::Both];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::PercentOnly => "% only",
            Self::UsdOnly => "$ only",
            Self::Both => "% + $",
        }
    }
}

impl std::fmt::Display for PnlCardDisplayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PnlCardPercentMode {
    AssetMove,
    Leveraged,
}

impl PnlCardPercentMode {
    pub(super) const ALL: [Self; 2] = [Self::AssetMove, Self::Leveraged];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::AssetMove => "Asset move",
            Self::Leveraged => "By leverage",
        }
    }
}

impl std::fmt::Display for PnlCardPercentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Clone)]
pub(crate) struct PnlCardWindowState {
    pub(crate) target: PnlCardTarget,
    pub(crate) account_address: String,
    pub(crate) display_mode: PnlCardDisplayMode,
    pub(crate) percent_mode: PnlCardPercentMode,
    pub(crate) obscure_prices: bool,
    pub(crate) show_position_size: bool,
}

impl PnlCardWindowState {
    pub(crate) fn new(target: PnlCardTarget, account_address: String) -> Self {
        Self {
            target,
            account_address,
            display_mode: PnlCardDisplayMode::Both,
            percent_mode: PnlCardPercentMode::Leveraged,
            obscure_prices: true,
            show_position_size: false,
        }
    }
}

impl fmt::Debug for PnlCardWindowState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PnlCardWindowState")
            .field("target", &self.target)
            .field("account_address", &"<redacted>")
            .field("display_mode", &self.display_mode)
            .field("percent_mode", &self.percent_mode)
            .field("obscure_prices", &self.obscure_prices)
            .field("show_position_size", &self.show_position_size)
            .finish()
    }
}
