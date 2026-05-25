use super::*;

#[test]
fn position_columns_hide_one_group_at_a_time_as_width_shrinks() {
    assert_eq!(
        PositionColumnVisibility::for_width(HIDE_TOTAL_PNL_BELOW),
        PositionColumnVisibility {
            liquidation: true,
            funding: true,
            total_pnl: true,
            leverage: true,
        }
    );
    assert_eq!(
        PositionColumnVisibility::for_width(HIDE_TOTAL_PNL_BELOW - 1.0),
        PositionColumnVisibility {
            liquidation: true,
            funding: true,
            total_pnl: false,
            leverage: true,
        }
    );
    assert_eq!(
        PositionColumnVisibility::for_width(HIDE_LEVERAGE_BELOW - 1.0),
        PositionColumnVisibility {
            liquidation: true,
            funding: true,
            total_pnl: false,
            leverage: false,
        }
    );
    assert_eq!(
        PositionColumnVisibility::for_width(HIDE_FUNDING_BELOW - 1.0),
        PositionColumnVisibility {
            liquidation: true,
            funding: false,
            total_pnl: false,
            leverage: false,
        }
    );
    assert_eq!(
        PositionColumnVisibility::for_width(HIDE_LIQUIDATION_BELOW - 1.0),
        PositionColumnVisibility {
            liquidation: false,
            funding: false,
            total_pnl: false,
            leverage: false,
        }
    );
}

#[test]
fn position_numbers_compact_after_optional_columns_are_hidden() {
    assert_eq!(
        PositionNumberMode::for_width(COMPACT_NUMBERS_BELOW),
        PositionNumberMode::Full
    );
    assert_eq!(
        PositionNumberMode::for_width(COMPACT_NUMBERS_BELOW - 1.0),
        PositionNumberMode::Compact
    );
}
