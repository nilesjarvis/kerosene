use crate::app_state::TradingTerminal;
use crate::message::Message;
use crate::portfolio_state::PnlValueDisplayMode;
use chrono::{Datelike, NaiveDate};
use iced::widget::{Space, column, container, row, text};
use iced::{Border, Element, Fill, Length, Theme};

use super::tokens;
use super::totals::format_signed_percent_value;

// ---------------------------------------------------------------------------
// Daily PnL — Diverging Bars
// ---------------------------------------------------------------------------

const DAILY_ROWS: usize = 7;
const DATE_COLUMN: f32 = 52.0;
const VALUE_COLUMN: f32 = 86.0;
const BAR_HEIGHT: f32 = 7.0;
const AXIS_HEIGHT: f32 = 13.0;
const BAR_PORTION_SCALE: f32 = 1000.0;

impl TradingTerminal {
    pub(super) fn view_portfolio_daily_section(&self, theme: &Theme) -> Element<'static, Message> {
        let value_mode = self.portfolio_pnl_value_display_mode();
        let rows = self.daily_pnl_rows(value_mode);

        let label = section_label(
            if value_mode == PnlValueDisplayMode::Percent {
                "Daily Performance · last 7 days"
            } else {
                "Daily PnL · last 7 days"
            },
            theme,
        );

        if rows.is_empty() {
            let empty = if value_mode == PnlValueDisplayMode::Percent {
                "No daily performance data"
            } else {
                "No daily PnL data"
            };
            return column![
                label,
                text(empty)
                    .size(11)
                    .font(tokens::mono())
                    .color(tokens::dim(theme)),
            ]
            .spacing(5)
            .width(Fill)
            .into();
        }

        let denomination = self.display_denomination_context();
        let max_abs = rows
            .iter()
            .map(|(_, value)| value.abs())
            .fold(0.0_f64, f64::max);

        let bars = rows
            .into_iter()
            .fold(column![].spacing(1).width(Fill), |list, (day, value)| {
                let value_text = if value_mode == PnlValueDisplayMode::Percent {
                    format_signed_percent_value(value)
                } else {
                    denomination.format_signed_value(value, 2)
                };
                list.push(daily_bar_row(
                    theme,
                    format_day_label(&day),
                    value,
                    daily_bar_fraction(value, max_abs),
                    value_text,
                ))
            });

        column![label, bars].spacing(5).width(Fill).into()
    }

    fn daily_pnl_rows(&self, value_mode: PnlValueDisplayMode) -> Vec<(String, f64)> {
        match value_mode {
            PnlValueDisplayMode::Percent => {
                let (pnl_history, account_value_history) = self
                    .daily_source_portfolio_bucket()
                    .map(|bucket| {
                        (
                            bucket.pnl_history.clone(),
                            bucket.account_value_history.clone(),
                        )
                    })
                    .unwrap_or_default();
                Self::compute_daily_percent_rows_from_cumulative(
                    &pnl_history,
                    &account_value_history,
                    DAILY_ROWS,
                )
            }
            PnlValueDisplayMode::Usd => {
                let pnl_history = self
                    .daily_source_portfolio_bucket()
                    .map(|bucket| bucket.pnl_history.clone())
                    .unwrap_or_default();
                Self::compute_daily_pnl_rows_from_cumulative(&pnl_history, DAILY_ROWS)
            }
        }
    }
}

fn section_label(label: &str, theme: &Theme) -> Element<'static, Message> {
    text(label.to_uppercase())
        .size(9)
        .font(tokens::mono())
        .color(tokens::dim(theme))
        .into()
}

fn daily_bar_row(
    theme: &Theme,
    date: String,
    value: f64,
    fraction: f64,
    value_text: String,
) -> Element<'static, Message> {
    let gain = value >= 0.0;
    let value_color = tokens::pnl_color(theme, Some(value));
    let portion = (fraction as f32 * BAR_PORTION_SCALE).round().max(0.0) as u16;
    let rest = BAR_PORTION_SCALE as u16 - portion.min(BAR_PORTION_SCALE as u16);

    let loss_cell: Element<'static, Message> = if !gain && portion > 0 {
        row![
            Space::new().width(Length::FillPortion(rest)),
            bar(tokens::down(theme), portion),
        ]
        .width(Fill)
        .into()
    } else {
        Space::new().width(Fill).into()
    };

    let gain_cell: Element<'static, Message> = if gain && portion > 0 {
        row![
            bar(tokens::up(theme), portion),
            Space::new().width(Length::FillPortion(rest)),
        ]
        .width(Fill)
        .into()
    } else {
        Space::new().width(Fill).into()
    };

    row![
        text(date)
            .size(10)
            .font(tokens::mono())
            .color(tokens::dim(theme))
            .width(DATE_COLUMN),
        loss_cell,
        center_axis(theme),
        gain_cell,
        container(
            text(value_text)
                .size(12)
                .font(tokens::mono_semibold())
                .color(value_color),
        )
        .width(VALUE_COLUMN)
        .align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(7)
    .align_y(iced::Alignment::Center)
    .width(Fill)
    .padding([3, 0])
    .into()
}

fn bar(color: iced::Color, portion: u16) -> Element<'static, Message> {
    container(Space::new().height(BAR_HEIGHT))
        .height(BAR_HEIGHT)
        .width(Length::FillPortion(portion))
        .style(move |_theme: &Theme| container::Style {
            background: Some(color.into()),
            border: Border {
                radius: 2.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

fn center_axis(theme: &Theme) -> Element<'static, Message> {
    let color = tokens::border(theme);
    container(Space::new().height(AXIS_HEIGHT))
        .width(1)
        .height(AXIS_HEIGHT)
        .style(move |_theme: &Theme| container::Style {
            background: Some(color.into()),
            ..container::Style::default()
        })
        .into()
}

/// Reformat a `YYYY-MM-DD` key to the compact `Jun 20` label, falling back to
/// the raw key when it cannot be parsed.
fn format_day_label(day: &str) -> String {
    match NaiveDate::parse_from_str(day, "%Y-%m-%d") {
        Ok(date) => format!("{} {}", date.format("%b"), date.day()),
        Err(_) => day.to_string(),
    }
}

/// Compressed log scale so small days stay visible while the largest day pins
/// to 100% (spec §5).
fn daily_bar_fraction(value: f64, max_abs: f64) -> f64 {
    if value == 0.0 || max_abs <= 0.0 {
        return 0.0;
    }
    0.16 + 0.84 * (1.0 + value.abs()).ln() / (1.0 + max_abs).ln()
}

#[cfg(test)]
mod tests {
    use super::{daily_bar_fraction, format_day_label};
    use crate::helpers::assert_close_loose as assert_near;

    #[test]
    fn zero_value_has_no_bar() {
        assert_eq!(daily_bar_fraction(0.0, 100.0), 0.0);
    }

    #[test]
    fn degenerate_max_abs_yields_zero_fraction() {
        assert_eq!(daily_bar_fraction(5.0, 0.0), 0.0);
    }

    #[test]
    fn largest_day_pins_to_full_width() {
        assert_near(daily_bar_fraction(14_395.72, 14_395.72), 1.0);
    }

    #[test]
    fn small_day_stays_visible() {
        // A $15 day against a ~$14k max must not collapse to an invisible sliver.
        let fraction = daily_bar_fraction(15.0, 14_395.72);
        assert!(fraction > 0.3, "expected visible bar, got {fraction}");
        assert!(fraction < 0.5, "expected sub-half bar, got {fraction}");
    }

    #[test]
    fn loss_and_gain_of_equal_magnitude_match() {
        assert_near(
            daily_bar_fraction(-2_691.99, 14_395.72),
            daily_bar_fraction(2_691.99, 14_395.72),
        );
    }

    #[test]
    fn day_label_is_compact() {
        assert_eq!(format_day_label("2026-06-20"), "Jun 20");
        assert_eq!(format_day_label("2026-06-06"), "Jun 6");
    }

    #[test]
    fn day_label_falls_back_on_bad_input() {
        assert_eq!(format_day_label("not-a-date"), "not-a-date");
    }
}
