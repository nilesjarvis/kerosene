use crate::app_state::TradingTerminal;
use crate::config::SortDirection;
use crate::denomination::format_compact_usd;
use crate::helpers;
use crate::message::Message;
use crate::screener_state::{ScreenerRow, ScreenerSortColumn};

use iced::widget::{
    Column, Space, button, column, container, pick_list, row, rule, scrollable, text,
};
use iced::{Alignment, Color, Element, Fill, Length, Theme};

// ---------------------------------------------------------------------------
// Screener Views
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_screener_window(&self) -> Element<'_, Message> {
        let theme = self.theme();
        let rows = self.screener_rows();

        let top_bar = row![
            text("Screener").size(16).color(theme.palette().text),
            pick_list(
                self.screener_exchange_filter_options(),
                Some(self.screener.exchange_filter.clone()),
                Message::ScreenerExchangeFilterChanged,
            )
            .padding([4, 8])
            .text_size(11)
            .width(Length::Fixed(150.0)),
            iced::widget::Space::new().width(Fill),
            button(text("Refresh").size(11))
                .on_press(Message::RefreshScreener)
                .padding([4, 10])
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let mut table = Column::new()
            .spacing(3)
            .push(screener_header(
                &theme,
                self.screener.sort_column,
                self.screener.sort_direction,
            ))
            .push(rule::horizontal(1));

        if self.symbols_loading {
            table = table.push(screener_placeholder_row("Loading symbols", &theme));
        } else if rows.is_empty() {
            table = table.push(screener_placeholder_row("No tickers available", &theme));
        } else {
            for (row_index, row_data) in rows.into_iter().enumerate() {
                table = table.push(screener_row(row_index, row_data, &theme, self));
            }
        }

        let content = column![top_bar, scrollable(table).height(Fill)].spacing(8);

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(12)
            .style(|theme: &Theme| iced::widget::container::Style {
                background: Some(theme.extended_palette().background.base.color.into()),
                ..Default::default()
            })
            .into()
    }
}

fn screener_header(
    theme: &Theme,
    sort_column: ScreenerSortColumn,
    sort_direction: SortDirection,
) -> Element<'static, Message> {
    let color = theme.extended_palette().background.weak.text;
    row![
        screener_header_cell(
            "Ticker",
            ScreenerSortColumn::Symbol,
            Length::Fill,
            sort_column,
            sort_direction,
            color
        ),
        screener_header_cell(
            "Price",
            ScreenerSortColumn::Price,
            Length::Fixed(112.0),
            sort_column,
            sort_direction,
            color
        ),
        screener_header_cell(
            "24h",
            ScreenerSortColumn::Change24h,
            Length::Fixed(76.0),
            sort_column,
            sort_direction,
            color
        ),
        screener_header_cell(
            "1h",
            ScreenerSortColumn::Change1h,
            Length::Fixed(76.0),
            sort_column,
            sort_direction,
            color
        ),
        screener_header_cell(
            "15m",
            ScreenerSortColumn::Change15m,
            Length::Fixed(76.0),
            sort_column,
            sort_direction,
            color
        ),
        screener_header_cell(
            "Volume",
            ScreenerSortColumn::Volume,
            Length::Fixed(84.0),
            sort_column,
            sort_direction,
            color
        ),
        screener_header_cell(
            "Funding",
            ScreenerSortColumn::Funding,
            Length::Fixed(88.0),
            sort_column,
            sort_direction,
            color
        ),
    ]
    .spacing(10)
    .padding([0, 8])
    .align_y(Alignment::Center)
    .into()
}

fn screener_header_cell<'a>(
    label: &'static str,
    column: ScreenerSortColumn,
    width: Length,
    sort_column: ScreenerSortColumn,
    sort_direction: SortDirection,
    color: Color,
) -> Element<'a, Message> {
    let mut content = row![
        text(label)
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(color)
    ];
    if sort_column == column {
        let icon = if sort_direction == SortDirection::Ascending {
            "\u{2191}"
        } else {
            "\u{2193}"
        };
        content = content.push(
            text(icon)
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(color),
        );
    }

    button(content.spacing(2))
        .on_press(Message::ScreenerSortChanged(column))
        .style(|_theme: &Theme, _status| button::Style {
            background: None,
            ..Default::default()
        })
        .padding(0)
        .width(width)
        .into()
}

fn screener_row(
    row_index: usize,
    row_data: ScreenerRow,
    theme: &Theme,
    terminal: &TradingTerminal,
) -> Element<'static, Message> {
    let price = row_data
        .price
        .map(|price| terminal.format_display_price(price))
        .unwrap_or_else(|| "-".to_string());
    let (pct_24h, pct_24h_color) = format_pct(row_data.pct_24h, theme);
    let (pct_1h, pct_1h_color) = format_pct(row_data.pct_1h, theme);
    let (pct_15m, pct_15m_color) = format_pct(row_data.pct_15m, theme);
    let volume = row_data
        .volume_24h
        .map(format_compact_usd)
        .unwrap_or_else(|| "-".to_string());
    let funding = row_data
        .funding
        .map(|funding| format!("{:.4}%", funding * 100.0))
        .unwrap_or_else(|| "-".to_string());

    let row_content = row![
        screener_symbol_cell(&row_data.symbol_key, row_data.display.clone(), theme),
        screener_value_cell(price, theme.palette().text, 112.0),
        screener_value_cell(pct_24h, pct_24h_color, 76.0),
        screener_value_cell(pct_1h, pct_1h_color, 76.0),
        screener_value_cell(pct_15m, pct_15m_color, 76.0),
        screener_value_cell(volume, theme.palette().text, 84.0),
        screener_value_cell(funding, theme.palette().text, 88.0),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    button(row_content)
        .on_press(Message::SymbolSelected(row_data.symbol_key))
        .padding([6, 8])
        .width(Fill)
        .style(move |theme: &Theme, status| {
            let background = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ if row_index % 2 == 1 => Color {
                    a: 0.34,
                    ..theme.extended_palette().background.weak.color
                },
                _ => theme.extended_palette().background.base.color,
            };
            button::Style {
                background: Some(background.into()),
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn screener_symbol_cell(
    sym_key: &str,
    display: String,
    theme: &Theme,
) -> Element<'static, Message> {
    let mut content = row![];
    if let Some(icon) = helpers::symbol_icon(sym_key, 14, theme.palette().text) {
        content = content.push(icon).push(Space::new().width(4.0));
    }

    content
        .push(
            text(display)
                .size(12)
                .color(theme.palette().text)
                .width(Fill),
        )
        .width(Fill)
        .align_y(Alignment::Center)
        .into()
}

fn screener_value_cell(
    value: String,
    color: Color,
    width: impl Into<iced::Length>,
) -> Element<'static, Message> {
    text(value)
        .size(11)
        .font(crate::app_fonts::monospace_font())
        .color(color)
        .width(width)
        .into()
}

fn screener_placeholder_row(label: &'static str, theme: &Theme) -> Element<'static, Message> {
    container(
        text(label)
            .size(12)
            .color(theme.extended_palette().background.weak.text),
    )
    .width(Fill)
    .padding([10, 8])
    .into()
}

fn format_pct(pct: Option<f64>, theme: &Theme) -> (String, Color) {
    pct.map(|value| {
        (
            format!("{value:+.2}%"),
            if value >= 0.0 {
                theme.palette().success
            } else {
                theme.palette().danger
            },
        )
    })
    .unwrap_or_else(|| {
        (
            "-".to_string(),
            theme.extended_palette().background.weak.text,
        )
    })
}
