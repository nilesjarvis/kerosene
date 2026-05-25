#[path = "pnl_card/text.rs"]
mod display_text;
mod image;
mod metrics;
mod model;
mod style;
mod update;
mod view;

#[cfg(test)]
use display_text::*;
#[cfg(test)]
use image::*;
#[cfg(test)]
use metrics::*;
pub(crate) use model::{PnlCardDisplayMode, PnlCardPercentMode, PnlCardTarget, PnlCardWindowState};
#[cfg(test)]
use style::*;
#[cfg(test)]
use update::pnl_card_account_matches;
pub(crate) use view::pnl_card_icon_button;

#[cfg(test)]
mod tests;
