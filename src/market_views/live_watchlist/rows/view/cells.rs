use crate::config;
use crate::denomination::DisplayDenominationContext;
use crate::helpers;
use crate::market_state::{LiveWatchlistId, LiveWatchlistRowData};
use crate::message::Message;
use iced::widget::{Row, Space, button, row, text};
use iced::{Color, Element, Fill, Theme};

pub(super) fn live_watchlist_symbol_cell(
    sym_key: &str,
    display: String,
    theme: &Theme,
) -> Row<'static, Message> {
    let mut coin_content = row![];
    if let Some(icon) = helpers::symbol_icon(sym_key, 14, theme.palette().text) {
        coin_content = coin_content.push(icon).push(Space::new().width(4.0));
    }
    coin_content.push(
        text(display)
            .size(12)
            .color(theme.palette().text)
            .width(Fill),
    )
}

pub(super) fn live_watchlist_column_value(
    column: &config::LiveWatchlistColumn,
    data: &LiveWatchlistRowData,
    denomination: &DisplayDenominationContext,
    price_color: Color,
    theme: &Theme,
) -> (String, Color) {
    match column {
        config::LiveWatchlistColumn::Price => data
            .mid_px
            .map(|mid_px| (denomination.format_price(mid_px), price_color))
            .unwrap_or_else(|| {
                (
                    "-".to_string(),
                    theme.extended_palette().background.weak.text,
                )
            }),
        config::LiveWatchlistColumn::Change5m => format_pct(data.pct_5m, theme),
        config::LiveWatchlistColumn::Change30m => format_pct(data.pct_30m, theme),
        config::LiveWatchlistColumn::Change1h => format_pct(data.pct_1h, theme),
        config::LiveWatchlistColumn::Change24h => format_pct(data.pct_24h, theme),
        config::LiveWatchlistColumn::Funding => (
            data.funding
                .map(|f| format!("{:.4}%", f * 100.0))
                .unwrap_or_else(|| "-".to_string()),
            theme.palette().text,
        ),
    }
}

pub(super) fn live_watchlist_column_cell(
    column: &config::LiveWatchlistColumn,
    value: String,
    color: Color,
) -> Element<'static, Message> {
    text(value)
        .size(11)
        .font(iced::Font::MONOSPACE)
        .color(color)
        .width(column.width())
        .into()
}

pub(super) fn live_watchlist_remove_button(
    id: LiveWatchlistId,
    sym_key: String,
    theme: &Theme,
) -> Element<'static, Message> {
    button(text("x").size(10).color(theme.palette().danger))
        .on_press(Message::LiveWatchlistRemoveSymbol(id, sym_key))
        .padding([0, 4])
        .style(|_theme: &Theme, _status| button::Style {
            background: None,
            ..Default::default()
        })
        .into()
}

pub(super) fn live_watchlist_row_button(
    sym_key: String,
    row_content: Row<'static, Message>,
) -> Element<'static, Message> {
    button(row_content)
        .on_press(Message::SymbolSelected(sym_key))
        .padding([6, 8])
        .width(Fill)
        .style(move |theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.base.color,
            };
            button::Style {
                background: Some(bg.into()),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn format_pct(pct: Option<f64>, theme: &Theme) -> (String, Color) {
    if let Some(p) = pct {
        (
            format!("{p:+.2}%"),
            if p >= 0.0 {
                theme.palette().success
            } else {
                theme.palette().danger
            },
        )
    } else {
        ("-".to_string(), theme.palette().text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row_data(mid_px: Option<f64>) -> LiveWatchlistRowData {
        LiveWatchlistRowData {
            sym_key: "BTC".to_string(),
            display: "BTC".to_string(),
            mid_px,
            pct_5m: None,
            pct_30m: None,
            pct_1h: None,
            pct_24h: None,
            funding: None,
        }
    }

    #[test]
    fn watchlist_price_cell_marks_missing_mid_unavailable() {
        let (value, _) = live_watchlist_column_value(
            &config::LiveWatchlistColumn::Price,
            &row_data(None),
            &DisplayDenominationContext::default(),
            Color::WHITE,
            &Theme::Dark,
        );
        assert_eq!(value, "-");

        let (value, _) = live_watchlist_column_value(
            &config::LiveWatchlistColumn::Price,
            &row_data(Some(123.45)),
            &DisplayDenominationContext::default(),
            Color::WHITE,
            &Theme::Dark,
        );
        assert_eq!(value, "123.45");
    }
}
