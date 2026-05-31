use crate::app_state::TradingTerminal;
use crate::helpers::timeframe_button;
use crate::message::Message;
use crate::portfolio_state::{
    PORTFOLIO_WINDOWS, PnlValueDisplayMode, PortfolioScope, PortfolioWindow,
};
use iced::widget::{Space, button, column, container, responsive, row, rule, text, tooltip};
use iced::{Color, Element, Fill, Theme};

const PORTFOLIO_TIMEFRAME_STACK_WIDTH: f32 = 460.0;
const PORTFOLIO_TIMEFRAME_WRAP_WIDTH: f32 = 300.0;
const PORTFOLIO_COMPACT_WINDOW_ROW_SIZE: usize = 4;

impl TradingTerminal {
    pub(super) fn view_portfolio_title(&self) -> Element<'static, Message> {
        let value_mode = self.portfolio_pnl_value_display_mode();
        let usd_mode = timeframe_button(
            "$",
            value_mode == PnlValueDisplayMode::Usd,
            Message::SetPortfolioPnlValueMode(PnlValueDisplayMode::Usd),
        );
        let percent_mode = timeframe_button(
            "%",
            value_mode == PnlValueDisplayMode::Percent,
            Message::SetPortfolioPnlValueMode(PnlValueDisplayMode::Percent),
        );

        let refresh_button = button(
            text("\u{21bb}")
                .size(13)
                .center()
                .font(crate::app_fonts::monospace_font()),
        )
        .on_press(Message::RefreshPortfolio)
        .padding([2, 7])
        .style(subtle_portfolio_header_button);

        row![
            row![usd_mode, percent_mode].spacing(2),
            Space::new().width(Fill),
            tooltip(
                refresh_button,
                text("Refresh").size(10),
                tooltip::Position::Top
            ),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .width(Fill)
        .into()
    }

    pub(super) fn view_portfolio_window_controls(&self) -> Element<'static, Message> {
        let scope = self.portfolio.scope;
        let window = self.portfolio.window;

        responsive(move |size| portfolio_window_controls_for_width(scope, window, size.width))
            .width(Fill)
            .into()
    }
}

fn portfolio_window_controls_for_width(
    scope: PortfolioScope,
    selected_window: PortfolioWindow,
    available_width: f32,
) -> Element<'static, Message> {
    let scope_row = portfolio_scope_row(scope);

    if available_width < PORTFOLIO_TIMEFRAME_STACK_WIDTH {
        let mut controls = column![scope_row].spacing(4).width(Fill);
        if available_width < PORTFOLIO_TIMEFRAME_WRAP_WIDTH {
            for row in portfolio_window_rows(selected_window) {
                controls = controls.push(row);
            }
        } else {
            controls = controls.push(portfolio_window_row(selected_window, PORTFOLIO_WINDOWS));
        }
        controls.into()
    } else {
        row![
            scope_row,
            container(rule::vertical(1)).height(16).width(8),
            portfolio_window_row(selected_window, PORTFOLIO_WINDOWS),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center)
        .width(Fill)
        .into()
    }
}

fn portfolio_scope_row(scope: PortfolioScope) -> iced::widget::Row<'static, Message> {
    row![
        timeframe_button(
            "All PnL",
            scope == PortfolioScope::All,
            Message::SetPortfolioScope(PortfolioScope::All),
        ),
        timeframe_button(
            "Perp PnL",
            scope == PortfolioScope::Perp,
            Message::SetPortfolioScope(PortfolioScope::Perp),
        ),
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center)
}

fn portfolio_window_rows(
    selected_window: PortfolioWindow,
) -> Vec<iced::widget::Row<'static, Message>> {
    PORTFOLIO_WINDOWS
        .chunks(PORTFOLIO_COMPACT_WINDOW_ROW_SIZE)
        .map(|windows| portfolio_window_row(selected_window, windows))
        .collect()
}

fn portfolio_window_row(
    selected_window: PortfolioWindow,
    windows: &[PortfolioWindow],
) -> iced::widget::Row<'static, Message> {
    windows.iter().copied().fold(
        row![].spacing(4).align_y(iced::Alignment::Center),
        |row, window| {
            row.push(timeframe_button(
                window.label(),
                selected_window == window,
                Message::SetPortfolioWindow(window),
            ))
        },
    )
}

fn subtle_portfolio_header_button(theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => Some(
            Color {
                a: 0.06,
                ..theme.palette().text
            }
            .into(),
        ),
        _ => Some(Color::TRANSPARENT.into()),
    };

    button::Style {
        background,
        text_color: theme.extended_palette().background.weak.text,
        border: iced::Border {
            radius: 3.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}
