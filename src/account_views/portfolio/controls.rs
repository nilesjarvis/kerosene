use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::portfolio_state::{PORTFOLIO_WINDOWS, PnlValueDisplayMode, PortfolioScope};
use iced::widget::{button, container, row, text};
use iced::{Border, Color, Element, Theme};

use super::tokens;

// ---------------------------------------------------------------------------
// Segmented Controls
// ---------------------------------------------------------------------------

const SEGMENT_TEXT_SIZE: f32 = 11.0;

impl TradingTerminal {
    /// Top control row: `$ | %` on the left, `All | Perp` on the right.
    pub(super) fn view_portfolio_control_row(&self) -> Element<'static, Message> {
        row![
            self.view_portfolio_value_mode_track(),
            iced::widget::Space::new().width(iced::Fill),
            self.view_portfolio_scope_track(),
        ]
        .align_y(iced::Alignment::Center)
        .width(iced::Fill)
        .into()
    }

    fn view_portfolio_value_mode_track(&self) -> Element<'static, Message> {
        let mode = self.portfolio.pnl_value_display_mode;
        segmented_track(vec![
            segment(
                "$",
                mode == PnlValueDisplayMode::Usd,
                Message::SetPortfolioPnlValueMode(PnlValueDisplayMode::Usd),
            ),
            segment(
                "%",
                mode == PnlValueDisplayMode::Percent,
                Message::SetPortfolioPnlValueMode(PnlValueDisplayMode::Percent),
            ),
        ])
    }

    fn view_portfolio_scope_track(&self) -> Element<'static, Message> {
        let scope = self.portfolio.scope;
        segmented_track(vec![
            segment(
                "All",
                scope == PortfolioScope::All,
                Message::SetPortfolioScope(PortfolioScope::All),
            ),
            segment(
                "Perp",
                scope == PortfolioScope::Perp,
                Message::SetPortfolioScope(PortfolioScope::Perp),
            ),
        ])
    }

    /// Timeframe track under the chart: one segment per portfolio window,
    /// left-aligned and only as wide as its contents.
    pub(super) fn view_portfolio_timeframe_track(&self) -> Element<'static, Message> {
        let selected = self.portfolio.window;
        let segments = PORTFOLIO_WINDOWS
            .iter()
            .copied()
            .map(|window| {
                segment(
                    window.label(),
                    selected == window,
                    Message::SetPortfolioWindow(window),
                )
            })
            .collect();
        container(segmented_track(segments))
            .align_x(iced::alignment::Horizontal::Left)
            .into()
    }
}

fn segmented_track(segments: Vec<Element<'static, Message>>) -> Element<'static, Message> {
    let track = segments
        .into_iter()
        .fold(row![].spacing(3), |row, segment| row.push(segment));

    container(track)
        .padding(3)
        .style(|theme: &Theme| container::Style {
            background: Some(tokens::track(theme).into()),
            border: Border {
                color: tokens::border(theme),
                width: 1.0,
                radius: 4.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

fn segment(label: &str, active: bool, msg: Message) -> Element<'static, Message> {
    button(
        text(label.to_string())
            .size(SEGMENT_TEXT_SIZE)
            .font(tokens::mono_semibold()),
    )
    .on_press(msg)
    .padding([3, 11])
    .style(move |theme: &Theme, status| segment_style(theme, active, status))
    .into()
}

fn segment_style(theme: &Theme, active: bool, status: button::Status) -> button::Style {
    let (background, border_color, text_color) = if active {
        (
            tokens::accent_wash(theme),
            tokens::accent_border(theme),
            tokens::accent_soft(theme),
        )
    } else {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let background = if hovered {
            Color {
                a: 0.06,
                ..tokens::text(theme)
            }
        } else {
            Color::TRANSPARENT
        };
        (background, Color::TRANSPARENT, tokens::muted(theme))
    };

    button::Style {
        background: Some(background.into()),
        text_color,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 3.0.into(),
        },
        ..button::Style::default()
    }
}
