use crate::config::{QuickTradeActionConfig, QuickTradeDenomination, QuickTradeSide};

use iced::window;

// ---------------------------------------------------------------------------
// Quick Trade Editor State
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(crate) struct QuickTradeActionDraft {
    pub(crate) side: QuickTradeSide,
    pub(crate) quantity: String,
    pub(crate) denomination: QuickTradeDenomination,
}

impl QuickTradeActionDraft {
    pub(crate) fn empty() -> Self {
        Self {
            side: QuickTradeSide::Buy,
            quantity: String::new(),
            denomination: QuickTradeDenomination::Usd,
        }
    }

    fn from_config(action: &QuickTradeActionConfig) -> Self {
        Self {
            side: action.side,
            quantity: format_quantity_input(action.quantity),
            denomination: action.denomination,
        }
    }
}

pub(crate) struct QuickTradeEditorState {
    pub(crate) window_id: window::Id,
    pub(crate) chart_id: u64,
    pub(crate) actions: Vec<QuickTradeActionDraft>,
    pub(crate) error: Option<String>,
}

impl QuickTradeEditorState {
    pub(crate) fn new(
        window_id: window::Id,
        chart_id: u64,
        actions: &[QuickTradeActionConfig],
    ) -> Self {
        Self {
            window_id,
            chart_id,
            actions: actions
                .iter()
                .map(QuickTradeActionDraft::from_config)
                .collect(),
            error: None,
        }
    }
}

fn format_quantity_input(quantity: f64) -> String {
    if quantity.fract().abs() <= f64::EPSILON {
        format!("{quantity:.0}")
    } else {
        quantity.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_quantity_keeps_whole_and_fractional_values_readable() {
        let whole = QuickTradeActionDraft::from_config(&QuickTradeActionConfig {
            side: QuickTradeSide::Buy,
            quantity: 10_000.0,
            denomination: QuickTradeDenomination::Usd,
        });
        let fractional = QuickTradeActionDraft::from_config(&QuickTradeActionConfig {
            side: QuickTradeSide::Sell,
            quantity: 0.25,
            denomination: QuickTradeDenomination::Coin,
        });

        assert_eq!(whole.quantity, "10000");
        assert_eq!(fractional.quantity, "0.25");
    }
}
