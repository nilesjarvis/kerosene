use super::analytics::{journal_effective_pnl, journal_is_non_perp, journal_trade_r_multiple};
use super::trade_card::journal_chip;
use super::trades::journal_pnl_color;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::journal::AggregatedTrade;
use crate::journal_views::style::{
    journal_accent_focus, journal_accent_soft, journal_dim, journal_monogram_style, journal_muted,
    journal_rule_style,
};
use crate::message::Message;
use iced::widget::container as container_style;
use iced::widget::{Column, Space, button, canvas, column, container, row, rule, scrollable, text};
use iced::{
    Alignment, Border, Color, Element, Fill, Length, Point, Rectangle, Renderer, Size, Theme,
};

const SPARKLINE_WIDTH: f32 = 60.0;
const SPARKLINE_HEIGHT: f32 = 26.0;
const MONOGRAM_SIZE: f32 = 30.0;

impl TradingTerminal {
    pub(super) fn view_journal_trade_list<'a>(
        &'a self,
        trades: &[&'a AggregatedTrade],
        r_unit: Option<f64>,
    ) -> Element<'a, Message> {
        let theme = self.theme();

        let header = container(
            row![
                text("ASSET · POSITION")
                    .size(9)
                    .font(crate::app_fonts::monospace_font())
                    .color(journal_muted(&theme)),
                Space::new().width(Fill),
                text("NET PNL")
                    .size(9)
                    .font(crate::app_fonts::monospace_font())
                    .color(journal_muted(&theme)),
            ]
            .align_y(Alignment::Center),
        )
        .width(Fill)
        .padding([10, 14]);

        let mut list = Column::new();
        for trade in trades.iter().copied() {
            list = list.push(self.view_journal_trade_row(trade, r_unit, &theme));
            list = list.push(rule::horizontal(1).style(journal_rule_style));
        }
        if self.journal.loading {
            list = list.push(self.view_journal_fetching_history_row(&theme));
        }

        column![
            header,
            rule::horizontal(1).style(journal_rule_style),
            scrollable(list)
                .direction(iced::widget::scrollable::Direction::Vertical(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(4)
                        .margin(0)
                        .scroller_width(4),
                ))
                .width(Fill)
                .height(Fill),
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }

    fn view_journal_trade_row<'a>(
        &'a self,
        trade: &'a AggregatedTrade,
        r_unit: Option<f64>,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let selected = self.journal.selected_trade_id.as_deref() == Some(trade.id.as_str());
        // The column is headed NET PNL, so honor the fee toggle exactly like
        // the KPI strip and the detail pane do.
        let net_pnl = journal_effective_pnl(trade, self.journal.include_fees_in_pnl);
        let pnl_color = journal_pnl_color(net_pnl, theme);
        let display_coin = self.display_coin_for_journal(&trade.coin);

        let monogram = journal_asset_badge(&display_coin, MONOGRAM_SIZE, 18, theme);

        let side_chip = journal_chip(side_label(trade), side_tint(trade, theme));
        let mut ticker = row![
            text(display_coin)
                .size(13)
                .font(crate::app_fonts::monospace_font())
                .color(journal_accent_soft(theme)),
            side_chip,
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        // Flag positions that are still open with an accent chip.
        if trade.status == "OPEN" {
            ticker = ticker.push(journal_chip("OPEN", theme.palette().primary));
        }

        let subline = text(format!(
            "{} · {}",
            self.journal_max_position_label(trade),
            short_timestamp(trade.start_time)
        ))
        .size(10)
        .font(crate::app_fonts::monospace_font())
        .color(journal_dim(theme));

        let info = column![ticker, subline].spacing(3);

        let sparkline = self.journal_sparkline(trade, pnl_color);

        let denomination = self.display_denomination_context();
        let r_label = journal_trade_r_multiple(trade, r_unit, self.journal.include_fees_in_pnl)
            .map(|r| format!("{r:+.1}R"))
            .unwrap_or_default();
        let metrics = column![
            text(denomination.format_signed_value(net_pnl, 2))
                .size(13)
                .font(crate::app_fonts::monospace_font())
                .color(pnl_color),
            text(r_label)
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(journal_dim(theme)),
        ]
        .spacing(3)
        .align_x(Alignment::End);

        let left_bar = container(Space::new().width(Length::Fixed(3.0)).height(Fill)).style(
            move |theme: &Theme| container_style::Style {
                background: Some(
                    if selected {
                        theme.palette().primary
                    } else {
                        Color::TRANSPARENT
                    }
                    .into(),
                ),
                ..Default::default()
            },
        );

        let content = row![
            left_bar,
            monogram,
            info,
            Space::new().width(Fill),
            sparkline,
            metrics,
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        button(content)
            .on_press(Message::JournalSelectTrade(trade.id.clone()))
            .padding([8, 11])
            .width(Fill)
            .style(move |theme: &Theme, status| journal_row_style(theme, status, selected))
            .into()
    }

    fn journal_sparkline(&self, trade: &AggregatedTrade, color: Color) -> Element<'_, Message> {
        let values: Vec<f32> = self
            .journal
            .trade_details
            .get(&trade.id)
            .map(|details| {
                details
                    .attributed_fills
                    .iter()
                    .filter(|fill| fill.price.is_finite() && fill.price > 0.0)
                    .map(|fill| fill.price as f32)
                    .collect()
            })
            .unwrap_or_default();

        canvas(JournalSparkline { values, color })
            .width(Length::Fixed(SPARKLINE_WIDTH))
            .height(Length::Fixed(SPARKLINE_HEIGHT))
            .into()
    }
}

fn journal_row_style(theme: &Theme, status: button::Status, selected: bool) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let background = if selected {
        Color {
            a: 0.08,
            ..theme.palette().primary
        }
    } else if hovered {
        Color {
            a: 0.04,
            ..theme.palette().text
        }
    } else {
        Color::TRANSPARENT
    };
    button::Style {
        background: Some(background.into()),
        text_color: theme.palette().text,
        border: Border {
            radius: 0.0.into(),
            width: 0.0,
            color: if selected {
                journal_accent_focus(theme)
            } else {
                Color::TRANSPARENT
            },
        },
        ..Default::default()
    }
}

fn side_label(trade: &AggregatedTrade) -> &'static str {
    if journal_is_non_perp(&trade.coin) {
        "SPOT"
    } else if trade.is_long {
        "LONG"
    } else {
        "SHORT"
    }
}

fn side_tint(trade: &AggregatedTrade, theme: &Theme) -> Color {
    if journal_is_non_perp(&trade.coin) {
        journal_muted(theme)
    } else if trade.is_long {
        theme.palette().success
    } else {
        theme.palette().danger
    }
}

/// Square asset badge: the embedded SVG logo when one exists for the symbol,
/// otherwise a 2-letter monogram. Used by both the list rows and the detail
/// header.
pub(super) fn journal_asset_badge(
    display_coin: &str,
    box_size: f32,
    icon_size: u16,
    theme: &Theme,
) -> Element<'static, Message> {
    let inner: Element<'static, Message> =
        match crate::helpers::symbol_icon(display_coin, icon_size, theme.palette().text) {
            Some(icon) => icon.into(),
            None => text(journal_monogram(display_coin))
                .size((box_size * 0.4).round().max(10.0))
                .font(crate::app_fonts::monospace_font())
                .into(),
        };
    container(inner)
        .center(Length::Fixed(box_size))
        .style(journal_monogram_style)
        .into()
}

pub(super) fn journal_monogram(display_coin: &str) -> String {
    let after_provider = display_coin.split(':').next_back().unwrap_or(display_coin);
    let base = after_provider.split('/').next().unwrap_or(after_provider);
    let letters: String = base
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .take(2)
        .collect();
    if letters.is_empty() {
        "??".to_string()
    } else {
        letters.to_uppercase()
    }
}

fn short_timestamp(time_ms: u64) -> String {
    helpers::format_timestamp_exact(time_ms)
}

// ---- Sparkline canvas ----

#[derive(Debug, Clone)]
struct JournalSparkline {
    values: Vec<f32>,
    color: Color,
}

impl canvas::Program<Message> for JournalSparkline {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        draw_sparkline(&mut frame, bounds.size(), &self.values, self.color);
        vec![frame.into_geometry()]
    }
}

fn draw_sparkline(frame: &mut canvas::Frame, size: Size, values: &[f32], color: Color) {
    if size.width <= 1.0 || size.height <= 1.0 {
        return;
    }
    let pad = 3.0_f32;
    let plot_w = (size.width - pad * 2.0).max(1.0);
    let plot_h = (size.height - pad * 2.0).max(1.0);

    if values.len() < 2 {
        // Flat reference line for trades without enough fill points.
        let y = pad + plot_h / 2.0;
        let path = canvas::Path::line(Point::new(pad, y), Point::new(pad + plot_w, y));
        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_color(Color { a: 0.4, ..color })
                .with_width(1.2),
        );
        return;
    }

    let (min, max) = values
        .iter()
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), value| {
            (lo.min(*value), hi.max(*value))
        });
    let span = (max - min).max(f32::EPSILON);
    let step = plot_w / (values.len() - 1) as f32;

    let points: Vec<Point> = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let x = pad + step * index as f32;
            let y = pad + (1.0 - (value - min) / span) * plot_h;
            Point::new(x, y)
        })
        .collect();

    let line = canvas::Path::new(|path| {
        path.move_to(points[0]);
        for point in &points[1..] {
            path.line_to(*point);
        }
    });
    frame.stroke(
        &line,
        canvas::Stroke::default().with_color(color).with_width(1.3),
    );
}
