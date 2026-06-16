use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::session_data_state::{
    SessionDataId, SessionDataInstance, SessionDataLookback, SessionGroup, SessionStreak,
    SessionVerdict, average_abs_move_pct, current_streak, most_active_weekday,
    overall_win_rate_pct, session_verdict, total_return_pct, weekday_dispersions,
};

use iced::mouse;
use iced::widget::canvas::{self, Frame, Path, Stroke, Text};
use iced::widget::{
    Space, button, canvas as canvas_widget, column, container, responsive, row, scrollable, text,
    text_input,
};
use iced::{Alignment, Color, Element, Fill, Length, Point, Rectangle, Renderer, Size, Theme};

const STATUS_ERROR_CHARS: usize = 72;
const BODY_ERROR_CHARS: usize = 140;

// ---- Edge-lane geometry ----
const ROW_HEIGHT: f32 = 22.0;
const SECTION_HEADER_HEIGHT: f32 = 17.0;
const LANE_TOP_PAD: f32 = 6.0;
const LANE_BOTTOM_PAD: f32 = 8.0;
const SECTION_GAP: f32 = 10.0;
const LABEL_GUTTER: f32 = 50.0;
const VALUE_GUTTER: f32 = 56.0;
const BULLET_GUTTER: f32 = 56.0;
const N_GUTTER: f32 = 34.0;
const BULLET_WIDTH: f32 = 42.0;
const BULLET_HEIGHT: f32 = 5.0;
const BAR_THICKNESS: f32 = 12.0;
const BAR_RADIUS: f32 = 2.5;
/// Sample count at which a bucket reaches full confidence (opacity). Chosen so
/// the default 4-week lookback (~4 samples/weekday) reads at roughly half
/// confidence and fills in by ~8 weeks, while sample-rich session buckets stay
/// solid throughout.
const CONF_FULL: f32 = 8.0;
/// Floor opacity for any bucket with at least one sample, so a real-but-thin
/// edge never vanishes.
const MIN_CONF: f32 = 0.35;
const COMPACT_LANE_WIDTH: f32 = 360.0;
const KPI_WRAP_WIDTH: f32 = 520.0;
const VERDICT_TAIL_WIDTH: f32 = 300.0;
const TOOLTIP_WIDTH: f32 = 190.0;
const TOOLTIP_HEIGHT: f32 = 84.0;
const ZERO_LINE_ALPHA: f32 = 0.24;

// ---------------------------------------------------------------------------
// Session Data View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_session_data(&self, id: SessionDataId) -> Element<'_, Message> {
        responsive(move |size| self.view_session_data_sized(id, size.width)).into()
    }

    fn view_session_data_sized(
        &self,
        id: SessionDataId,
        available_width: f32,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let Some(instance) = self.session_data.get(&id) else {
            return container(
                text("Session Data instance missing")
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            )
            .width(Fill)
            .height(Fill)
            .center(Fill)
            .padding(10)
            .into();
        };

        let header = self.view_session_data_header(instance, &theme, available_width);
        let mut content = column![header].spacing(8).padding(8);
        if instance.symbol_picker_open {
            content = content.push(self.view_session_data_symbol_dropdown(instance, &theme));
        }
        content = content.push(self.view_session_data_body(instance, &theme, available_width));

        container(scrollable(content).height(Fill))
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn view_session_data_header<'a>(
        &'a self,
        instance: &'a SessionDataInstance,
        theme: &Theme,
        available_width: f32,
    ) -> Element<'a, Message> {
        let display = self.display_name_for_symbol(&instance.symbol);
        let mut symbol_content = row![].spacing(5).align_y(Alignment::Center);
        if let Some(icon) = helpers::symbol_icon(&instance.symbol, 15, theme.palette().text) {
            symbol_content = symbol_content.push(icon);
        }
        symbol_content = symbol_content
            .push(text(display).size(12).color(theme.palette().text))
            .push(
                text(if instance.symbol_picker_open {
                    "\u{25b2}"
                } else {
                    "\u{25be}"
                })
                .size(9)
                .color(theme.extended_palette().background.weak.text),
            );

        let symbol_button = button(symbol_content)
            .on_press(Message::ToggleSessionDataSymbolPicker(instance.id))
            .padding([3, 8])
            .style(move |theme: &Theme, status| compact_button_style(theme, status, true));

        let mut lookbacks = row![].spacing(3).align_y(Alignment::Center);
        for lookback in SessionDataLookback::ALL {
            lookbacks = lookbacks.push(lookback_button(
                lookback,
                instance.lookback == lookback,
                instance.id,
            ));
        }

        let refresh_label: Element<'_, Message> = if instance.loading {
            self.view_spinner(14)
        } else {
            text("\u{21bb}")
                .size(13)
                .center()
                .font(crate::app_fonts::monospace_font())
                .color(theme.extended_palette().background.weak.text)
                .into()
        };
        let refresh = button(refresh_label)
            .on_press_maybe((!instance.loading).then_some(Message::RefreshSessionData(instance.id)))
            .padding([3, 8])
            .style(move |theme: &Theme, status| compact_button_style(theme, status, false));

        let status = session_data_status(instance);
        let status_text = text(status)
            .size(10)
            .color(theme.extended_palette().background.weak.text);

        if available_width < 520.0 {
            column![
                row![symbol_button, refresh]
                    .spacing(6)
                    .align_y(Alignment::Center),
                lookbacks,
                status_text
            ]
            .spacing(6)
            .into()
        } else {
            row![
                symbol_button,
                lookbacks,
                Space::new().width(Fill),
                status_text,
                refresh
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        }
    }

    fn view_session_data_symbol_dropdown<'a>(
        &'a self,
        instance: &'a SessionDataInstance,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let search = text_input("Search perp or spot...", &instance.search_query)
            .style(helpers::text_input_style)
            .on_input(move |q| Message::SessionDataSearchChanged(instance.id, q))
            .size(12)
            .padding([5, 8]);

        let mut results = column![].spacing(2);
        let query = instance.search_query.trim().to_ascii_lowercase();
        for symbol in self
            .exchange_symbols
            .iter()
            .filter(|symbol| matches!(symbol.market_type, MarketType::Perp | MarketType::Spot))
            .filter(|symbol| symbol.is_user_selectable_market())
            .filter(|symbol| !self.exchange_symbol_is_hidden(symbol))
            .filter(|symbol| {
                if query.is_empty() {
                    return true;
                }
                let display = Self::exchange_symbol_display_name(symbol).to_ascii_lowercase();
                symbol.key.to_ascii_lowercase().contains(&query)
                    || symbol.ticker.to_ascii_lowercase().contains(&query)
                    || display.contains(&query)
                    || symbol.category.to_ascii_lowercase().contains(&query)
            })
            .take(12)
        {
            let display = Self::exchange_symbol_display_name(symbol);
            let market = match symbol.market_type {
                MarketType::Perp => "perp",
                MarketType::Spot => "spot",
                MarketType::Outcome => "outcome",
            };
            let row = row![
                text(display).size(11).width(Fill),
                text(symbol.key.clone())
                    .size(10)
                    .font(crate::app_fonts::monospace_font())
                    .color(theme.extended_palette().background.weak.text),
                text(market)
                    .size(9)
                    .color(theme.extended_palette().background.weak.text),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            results = results.push(
                button(row)
                    .on_press(Message::SessionDataSymbolSelected(
                        instance.id,
                        symbol.key.clone(),
                    ))
                    .padding([5, 8])
                    .width(Fill)
                    .style(|theme: &Theme, status| compact_button_style(theme, status, false)),
            );
        }

        if self.exchange_symbols.is_empty() {
            results = results.push(
                text("Symbols loading")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        container(column![search, results].spacing(6).padding(6))
            .width(Fill)
            .style(|theme: &Theme| container::Style {
                background: Some(theme.extended_palette().background.strong.color.into()),
                text_color: Some(theme.palette().text),
                border: iced::Border {
                    radius: 4.0.into(),
                    width: 1.0,
                    color: theme.extended_palette().background.weak.color,
                },
                ..Default::default()
            })
            .into()
    }

    fn view_session_data_body<'a>(
        &'a self,
        instance: &'a SessionDataInstance,
        theme: &Theme,
        available_width: f32,
    ) -> Element<'a, Message> {
        if instance.loading && instance.bars.is_empty() {
            return container(self.view_spinner(20))
                .width(Fill)
                .height(Fill)
                .center(Fill)
                .into();
        }

        if instance.bars.is_empty() {
            let message = instance
                .error
                .as_deref()
                .unwrap_or("No session history available");
            return container(
                text(message)
                    .size(12)
                    .color(theme.extended_palette().background.weak.text),
            )
            .width(Fill)
            .height(Fill)
            .center(Fill)
            .padding(10)
            .into();
        }

        let verdict = session_verdict(
            &instance.weekday_summaries,
            &instance.session_summaries,
            instance.bars.len(),
        );
        let best_key: Option<(SessionGroup, &str)> = match &verdict {
            SessionVerdict::Edge { strongest, .. } => {
                Some((strongest.group, strongest.label.as_str()))
            }
            SessionVerdict::Insufficient { .. } => None,
        };

        let dispersions = weekday_dispersions(&instance.bars);
        let weekday_rows: Vec<LaneRow> = instance
            .weekday_summaries
            .iter()
            .map(|summary| {
                let label = summary.weekday.label();
                LaneRow {
                    label: label.to_string(),
                    sample_count: summary.sample_count,
                    average_return_pct: summary.average_return_pct,
                    win_rate_pct: summary.win_rate_pct,
                    dispersion_pct: dispersions[summary.weekday.index()],
                    is_best: best_key == Some((SessionGroup::Weekday, label)),
                }
            })
            .collect();
        let session_rows: Vec<LaneRow> = instance
            .session_summaries
            .iter()
            .map(|summary| {
                let label = summary.session.short_label();
                LaneRow {
                    label: label.to_string(),
                    sample_count: summary.sample_count,
                    average_return_pct: summary.average_return_pct,
                    win_rate_pct: summary.win_rate_pct,
                    dispersion_pct: None,
                    is_best: best_key == Some((SessionGroup::Session, label)),
                }
            })
            .collect();

        let scale_max = weekday_rows
            .iter()
            .chain(session_rows.iter())
            .map(|row| row.average_return_pct.abs())
            .fold(0.0_f64, f64::max)
            .max(0.1) as f32;
        let lane_height = lane_height(weekday_rows.len(), session_rows.len());
        let lane = canvas_widget(SessionLane {
            weekday_rows,
            session_rows,
            scale_max,
            compact: available_width < COMPACT_LANE_WIDTH,
        })
        .width(Fill)
        .height(Length::Fixed(lane_height));

        let summary = self.view_session_data_summary(instance, theme, &verdict, available_width);
        let mut content = column![summary, lane].spacing(10);

        if let Some(error) = &instance.error {
            content = content.push(
                text(helpers::ellipsized_text(error, BODY_ERROR_CHARS))
                    .size(10)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        container(content).width(Fill).height(Fill).into()
    }

    fn view_session_data_summary(
        &self,
        instance: &SessionDataInstance,
        theme: &Theme,
        verdict: &SessionVerdict,
        available_width: f32,
    ) -> Element<'static, Message> {
        let verdict_line = view_verdict_line(verdict, theme, available_width);
        let kpis = view_kpi_strip(instance, theme, self.status_bar_now_ms, available_width);
        column![verdict_line, kpis].spacing(8).into()
    }
}

// ---------------------------------------------------------------------------
// Verdict line + KPI strip
// ---------------------------------------------------------------------------

fn view_verdict_line(
    verdict: &SessionVerdict,
    theme: &Theme,
    available_width: f32,
) -> Element<'static, Message> {
    let weak = theme.extended_palette().background.weak.text;
    let accent = text("\u{258e}").size(14).color(theme.palette().primary);

    match verdict {
        SessionVerdict::Insufficient {
            total_samples,
            min_required,
        } => row![
            accent,
            text(format!(
                "Not enough completed sessions to call a trend ({total_samples} so far, need {min_required}+ per bucket)"
            ))
            .size(11)
            .color(weak),
        ]
        .spacing(5)
        .align_y(Alignment::Center)
        .into(),
        SessionVerdict::Edge { strongest, weakest } => {
            let mut line = row![
                accent,
                text("Strongest").size(11).color(weak),
                text(strongest.label.clone())
                    .size(11)
                    .color(theme.palette().text),
                text(helpers::format_signed_percent_value(strongest.average_return_pct))
                    .size(11)
                    .font(crate::app_fonts::monospace_font())
                    .color(helpers::signed_number_color(
                        strongest.average_return_pct,
                        theme
                    )),
                text("\u{00b7}").size(11).color(weak),
                text(format!("{:.0}% win", strongest.win_rate_pct))
                    .size(11)
                    .color(theme.palette().text),
                text("\u{00b7}").size(11).color(weak),
                text(format!("n{}", strongest.sample_count))
                    .size(10)
                    .color(weak),
            ]
            .spacing(5)
            .align_y(Alignment::Center);

            if available_width >= VERDICT_TAIL_WIDTH
                && let Some(weakest) = weakest
            {
                line = line
                    .push(Space::new().width(Fill))
                    .push(text("Weakest").size(11).color(weak))
                    .push(
                        text(weakest.label.clone())
                            .size(11)
                            .color(theme.palette().text),
                    )
                    .push(
                        text(helpers::format_signed_percent_value(weakest.average_return_pct))
                            .size(11)
                            .font(crate::app_fonts::monospace_font())
                            .color(helpers::signed_number_color(
                                weakest.average_return_pct,
                                theme,
                            )),
                    );
            }
            line.into()
        }
    }
}

fn view_kpi_strip(
    instance: &SessionDataInstance,
    theme: &Theme,
    now_ms: u64,
    available_width: f32,
) -> Element<'static, Message> {
    let weak = theme.extended_palette().background.weak.text;

    let win = overall_win_rate_pct(&instance.weekday_summaries);
    let avg_move = average_abs_move_pct(&instance.bars);
    let total = total_return_pct(&instance.bars);
    let streak = current_streak(&instance.bars);
    let active = most_active_weekday(&instance.bars);
    let freshness = instance
        .last_fetch_ms
        .map(|ms| format!("{} ago", helpers::format_relative_time(ms, now_ms)));

    let tiles: Vec<Element<'static, Message>> = vec![
        view_kpi_tile_winrate(win, theme),
        view_kpi_tile(
            "AVG MOVE",
            avg_move.map(|value| format!("{value:.2}%")),
            theme.palette().text,
            theme,
        ),
        view_kpi_tile_signed("TOTAL", total, theme),
        view_kpi_tile_streak(streak, theme),
        view_kpi_tile(
            "BUSIEST",
            active.map(|weekday| weekday.label().to_string()),
            theme.palette().text,
            theme,
        ),
        view_kpi_tile("UPDATED", freshness, weak, theme),
    ];

    if available_width >= KPI_WRAP_WIDTH {
        let mut strip = row![].spacing(6);
        for tile in tiles {
            strip = strip.push(tile);
        }
        strip.into()
    } else {
        let mut top = row![].spacing(6);
        let mut bottom = row![].spacing(6);
        for (idx, tile) in tiles.into_iter().enumerate() {
            if idx < 3 {
                top = top.push(tile);
            } else {
                bottom = bottom.push(tile);
            }
        }
        column![top, bottom].spacing(6).into()
    }
}

fn view_kpi_tile(
    label: &str,
    value: Option<String>,
    value_color: Color,
    theme: &Theme,
) -> Element<'static, Message> {
    let (value, value_color) = match value {
        Some(value) => (value, value_color),
        None => (
            "\u{2014}".to_string(),
            theme.extended_palette().background.weak.text,
        ),
    };
    let body = column![
        text(label.to_string())
            .size(9)
            .color(theme.extended_palette().background.weak.text),
        text(value)
            .size(13)
            .font(crate::app_fonts::monospace_font())
            .color(value_color),
    ]
    .spacing(2);
    container(body)
        .padding([4, 7])
        .width(Fill)
        .style(kpi_tile_style)
        .into()
}

fn view_kpi_tile_signed(
    label: &str,
    value: Option<f64>,
    theme: &Theme,
) -> Element<'static, Message> {
    match value {
        Some(value) => view_kpi_tile(
            label,
            Some(helpers::format_signed_percent_value(value)),
            helpers::signed_number_color(value, theme),
            theme,
        ),
        None => view_kpi_tile(label, None, theme.palette().text, theme),
    }
}

fn view_kpi_tile_streak(streak: Option<SessionStreak>, theme: &Theme) -> Element<'static, Message> {
    match streak {
        Some(streak) if streak.positive => view_kpi_tile(
            "STREAK",
            Some(format!("\u{25b2} {}", streak.length)),
            theme.palette().success,
            theme,
        ),
        Some(streak) => view_kpi_tile(
            "STREAK",
            Some(format!("\u{25bc} {}", streak.length)),
            theme.palette().danger,
            theme,
        ),
        None => view_kpi_tile("STREAK", None, theme.palette().text, theme),
    }
}

fn view_kpi_tile_winrate(rate: Option<f64>, theme: &Theme) -> Element<'static, Message> {
    let weak = theme.extended_palette().background.weak.text;
    let label = text("WIN RATE").size(9).color(weak);
    let value: Element<'static, Message> = match rate {
        Some(rate) => row![
            text(format!("{rate:.0}%"))
                .size(13)
                .font(crate::app_fonts::monospace_font())
                .color(theme.palette().text),
            canvas_widget(WinRateBullet {
                ratio: (rate / 100.0) as f32,
            })
            .width(Length::Fixed(34.0))
            .height(Length::Fixed(8.0)),
        ]
        .spacing(5)
        .align_y(Alignment::Center)
        .into(),
        None => text("\u{2014}")
            .size(13)
            .font(crate::app_fonts::monospace_font())
            .color(weak)
            .into(),
    };
    container(column![label, value].spacing(2))
        .padding([4, 7])
        .width(Fill)
        .style(kpi_tile_style)
        .into()
}

fn kpi_tile_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: iced::Border {
            radius: 3.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    }
}

fn lookback_button(
    lookback: SessionDataLookback,
    active: bool,
    id: SessionDataId,
) -> Element<'static, Message> {
    button(text(lookback.label()).size(10).center())
        .on_press(Message::SessionDataLookbackChanged(id, lookback))
        .padding([3, 7])
        .style(move |theme: &Theme, status| compact_button_style(theme, status, active))
        .into()
}

fn compact_button_style(theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let bg = match (active, status) {
        (true, _) => theme.extended_palette().background.strong.color,
        (false, button::Status::Hovered) | (false, button::Status::Pressed) => {
            theme.extended_palette().background.weak.color
        }
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: if active {
            theme.palette().primary
        } else {
            theme.palette().text
        },
        border: iced::Border {
            radius: 3.0.into(),
            width: if active { 1.0 } else { 0.0 },
            color: if active {
                Color {
                    a: 0.42,
                    ..theme.palette().primary
                }
            } else {
                Color::TRANSPARENT
            },
        },
        ..Default::default()
    }
}

fn session_data_status(instance: &SessionDataInstance) -> String {
    if instance.loading {
        return "Loading".to_string();
    }
    if let Some(error) = &instance.error
        && instance.bars.is_empty()
    {
        return helpers::ellipsized_text(error, STATUS_ERROR_CHARS);
    }
    format!("{} sessions", instance.bars.len())
}

// ---------------------------------------------------------------------------
// Win-rate bullet (KPI tile)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct WinRateBullet {
    /// Win rate in 0..1.
    ratio: f32,
}

impl canvas::Program<Message> for WinRateBullet {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return vec![frame.into_geometry()];
        }
        let track_h = (bounds.height * 0.5).clamp(3.0, 5.0);
        let y = (bounds.height - track_h) * 0.5;
        frame.fill_rectangle(
            Point::new(0.0, y),
            Size::new(bounds.width, track_h),
            Color {
                a: 0.9,
                ..theme.extended_palette().background.strong.color
            },
        );
        let fill_w = (self.ratio.clamp(0.0, 1.0) * bounds.width).max(0.0);
        frame.fill_rectangle(
            Point::new(0.0, y),
            Size::new(fill_w, track_h),
            theme.palette().primary,
        );
        let tick_x = bounds.width * 0.5;
        frame.fill_rectangle(
            Point::new(tick_x - 0.5, y - 1.0),
            Size::new(1.0, track_h + 2.0),
            Color {
                a: 0.6,
                ..theme.palette().text
            },
        );
        vec![frame.into_geometry()]
    }
}

// ---------------------------------------------------------------------------
// Edge lane (weekday + session breakdown)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct LaneRow {
    label: String,
    sample_count: usize,
    average_return_pct: f64,
    win_rate_pct: f64,
    dispersion_pct: Option<f64>,
    is_best: bool,
}

#[derive(Debug, Clone)]
struct SessionLane {
    weekday_rows: Vec<LaneRow>,
    session_rows: Vec<LaneRow>,
    scale_max: f32,
    compact: bool,
}

#[derive(Default)]
struct SessionLaneState {
    hovered: Option<usize>,
}

fn lane_height(weekday_rows: usize, session_rows: usize) -> f32 {
    LANE_TOP_PAD
        + SECTION_HEADER_HEIGHT
        + weekday_rows as f32 * ROW_HEIGHT
        + SECTION_GAP
        + SECTION_HEADER_HEIGHT
        + session_rows as f32 * ROW_HEIGHT
        + LANE_BOTTOM_PAD
}

impl SessionLane {
    fn total_rows(&self) -> usize {
        self.weekday_rows.len() + self.session_rows.len()
    }

    fn row_at(&self, index: usize) -> Option<&LaneRow> {
        let weekday = self.weekday_rows.len();
        if index < weekday {
            self.weekday_rows.get(index)
        } else {
            self.session_rows.get(index - weekday)
        }
    }

    fn row_y_center(&self, index: usize) -> f32 {
        let weekday = self.weekday_rows.len();
        if index < weekday {
            LANE_TOP_PAD + SECTION_HEADER_HEIGHT + index as f32 * ROW_HEIGHT + ROW_HEIGHT * 0.5
        } else {
            let session_index = index - weekday;
            LANE_TOP_PAD
                + SECTION_HEADER_HEIGHT
                + weekday as f32 * ROW_HEIGHT
                + SECTION_GAP
                + SECTION_HEADER_HEIGHT
                + session_index as f32 * ROW_HEIGHT
                + ROW_HEIGHT * 0.5
        }
    }

    fn row_at_cursor(&self, bounds: Rectangle, cursor: mouse::Cursor) -> Option<usize> {
        let pos = cursor.position_in(bounds)?;
        if pos.x < 0.0 || pos.x > bounds.width {
            return None;
        }
        (0..self.total_rows())
            .find(|&index| (pos.y - self.row_y_center(index)).abs() <= ROW_HEIGHT * 0.5)
    }
}

impl canvas::Program<Message> for SessionLane {
    type State = SessionLaneState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let next = match event {
            iced::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                self.row_at_cursor(bounds, cursor)
            }
            iced::Event::Mouse(mouse::Event::CursorLeft) => None,
            _ => return None,
        };
        if state.hovered != next {
            state.hovered = next;
            return Some(canvas::Action::request_redraw());
        }
        None
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), Color::TRANSPARENT);

        if self.total_rows() == 0 || bounds.width <= 0.0 || bounds.height <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let layout = LaneLayout::new(bounds.size(), self.compact);

        // Shared zero baseline running through both sections.
        let baseline = Path::line(
            Point::new(layout.mid_x, LANE_TOP_PAD + SECTION_HEADER_HEIGHT),
            Point::new(layout.mid_x, bounds.height - LANE_BOTTOM_PAD),
        );
        frame.stroke(
            &baseline,
            Stroke::default()
                .with_color(Color {
                    a: ZERO_LINE_ALPHA,
                    ..theme.palette().text
                })
                .with_width(1.0),
        );

        let mut y = LANE_TOP_PAD;
        draw_section_header(&mut frame, theme, "WEEKDAY", &layout, y);
        y += SECTION_HEADER_HEIGHT;
        for (index, row) in self.weekday_rows.iter().enumerate() {
            draw_lane_row(
                &mut frame,
                theme,
                &layout,
                row,
                self.scale_max,
                y,
                state.hovered == Some(index),
            );
            y += ROW_HEIGHT;
        }

        y += SECTION_GAP;
        draw_section_header(&mut frame, theme, "SESSION", &layout, y);
        y += SECTION_HEADER_HEIGHT;
        let weekday = self.weekday_rows.len();
        for (index, row) in self.session_rows.iter().enumerate() {
            draw_lane_row(
                &mut frame,
                theme,
                &layout,
                row,
                self.scale_max,
                y,
                state.hovered == Some(weekday + index),
            );
            y += ROW_HEIGHT;
        }

        if let Some(index) = state.hovered
            && let Some(row) = self.row_at(index)
        {
            draw_lane_tooltip(
                &mut frame,
                theme,
                bounds.size(),
                &layout,
                row,
                self.row_y_center(index),
            );
        }

        vec![frame.into_geometry()]
    }
}

struct LaneLayout {
    width: f32,
    bar_left: f32,
    mid_x: f32,
    half_w: f32,
    value_right: Option<f32>,
    bullet_left: f32,
    n_right: Option<f32>,
}

impl LaneLayout {
    fn new(size: Size, compact: bool) -> Self {
        let width = size.width;
        let bar_left = LABEL_GUTTER;
        let (right_block, value_right, n_right, bullet_left) = if compact {
            (BULLET_GUTTER, None, None, width - BULLET_WIDTH - 8.0)
        } else {
            (
                VALUE_GUTTER + BULLET_GUTTER + N_GUTTER,
                Some(width - N_GUTTER - BULLET_GUTTER - 6.0),
                Some(width - 6.0),
                width - N_GUTTER - BULLET_GUTTER + (BULLET_GUTTER - BULLET_WIDTH) * 0.5,
            )
        };
        let bar_right = (width - right_block).max(bar_left + 8.0);
        let mid_x = (bar_left + bar_right) * 0.5;
        let half_w = ((bar_right - bar_left) * 0.5 - 3.0).max(1.0);
        Self {
            width,
            bar_left,
            mid_x,
            half_w,
            value_right,
            bullet_left: bullet_left.max(bar_left),
            n_right,
        }
    }
}

fn draw_section_header(frame: &mut Frame, theme: &Theme, label: &str, layout: &LaneLayout, y: f32) {
    let weak = theme.extended_palette().background.weak.text;
    let center_y = y + SECTION_HEADER_HEIGHT * 0.5;
    frame.fill_text(Text {
        content: label.to_string(),
        position: Point::new(6.0, center_y),
        color: weak,
        size: iced::Pixels(9.0),
        align_y: iced::alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..Default::default()
    });
    let rule = Path::line(
        Point::new(layout.bar_left, center_y + 0.5),
        Point::new(layout.width - 6.0, center_y + 0.5),
    );
    frame.stroke(
        &rule,
        Stroke::default()
            .with_color(Color {
                a: 0.12,
                ..theme.palette().text
            })
            .with_width(1.0),
    );
}

fn draw_lane_row(
    frame: &mut Frame,
    theme: &Theme,
    layout: &LaneLayout,
    row: &LaneRow,
    scale_max: f32,
    y: f32,
    hovered: bool,
) {
    let weak = theme.extended_palette().background.weak.text;
    let center_y = y + ROW_HEIGHT * 0.5;

    if row.is_best {
        let marker = Path::rounded_rectangle(
            Point::new(2.0, center_y - BAR_THICKNESS * 0.5),
            Size::new(2.0, BAR_THICKNESS),
            1.0.into(),
        );
        frame.fill(&marker, theme.palette().primary);
    }

    let label_color = if row.is_best {
        theme.palette().text
    } else if row.sample_count > 0 {
        Color {
            a: 0.85,
            ..theme.palette().text
        }
    } else {
        weak
    };
    frame.fill_text(Text {
        content: row.label.clone(),
        position: Point::new(8.0, center_y),
        color: label_color,
        size: iced::Pixels(10.0),
        align_y: iced::alignment::Vertical::Center,
        font: crate::app_fonts::monospace_font(),
        ..Default::default()
    });

    if row.sample_count == 0 {
        draw_absent_dots(frame, theme, layout, center_y);
        if let Some(value_right) = layout.value_right {
            frame.fill_text(Text {
                content: "\u{2014}".to_string(),
                position: Point::new(value_right, center_y),
                color: weak,
                size: iced::Pixels(9.0),
                align_x: iced::alignment::Horizontal::Right.into(),
                align_y: iced::alignment::Vertical::Center,
                font: crate::app_fonts::monospace_font(),
                ..Default::default()
            });
        }
        if let Some(n_right) = layout.n_right {
            frame.fill_text(Text {
                content: "n0".to_string(),
                position: Point::new(n_right, center_y),
                color: weak,
                size: iced::Pixels(9.0),
                align_x: iced::alignment::Horizontal::Right.into(),
                align_y: iced::alignment::Vertical::Center,
                font: crate::app_fonts::monospace_font(),
                ..Default::default()
            });
        }
        return;
    }

    let confidence = if hovered {
        1.0
    } else {
        (row.sample_count as f32 / CONF_FULL).clamp(MIN_CONF, 1.0)
    };

    // Return bar — center-anchored, growing right for gains / left for losses.
    let base = helpers::signed_number_color(row.average_return_pct, theme);
    let magnitude = (row.average_return_pct.abs() as f32 / scale_max).clamp(0.0, 1.0);
    let bar_len = (magnitude * layout.half_w).max(2.0);
    let bar_x = if row.average_return_pct >= 0.0 {
        layout.mid_x
    } else {
        layout.mid_x - bar_len
    };
    let bar_top = center_y - BAR_THICKNESS * 0.5;
    let bar = Path::rounded_rectangle(
        Point::new(bar_x, bar_top),
        Size::new(bar_len, BAR_THICKNESS),
        BAR_RADIUS.into(),
    );
    frame.fill(
        &bar,
        lane_bar_gradient(base, bar_top, BAR_THICKNESS, confidence),
    );

    draw_win_bullet(frame, theme, layout, row.win_rate_pct, center_y, confidence);

    if let Some(value_right) = layout.value_right {
        frame.fill_text(Text {
            content: helpers::format_signed_percent_value(row.average_return_pct),
            position: Point::new(value_right, center_y),
            color: Color {
                a: if hovered { 1.0 } else { 0.9 },
                ..base
            },
            size: iced::Pixels(9.0),
            align_x: iced::alignment::Horizontal::Right.into(),
            align_y: iced::alignment::Vertical::Center,
            font: crate::app_fonts::monospace_font(),
            ..Default::default()
        });
    }
    if let Some(n_right) = layout.n_right {
        frame.fill_text(Text {
            content: format!("n{}", row.sample_count),
            position: Point::new(n_right, center_y),
            color: weak,
            size: iced::Pixels(9.0),
            align_x: iced::alignment::Horizontal::Right.into(),
            align_y: iced::alignment::Vertical::Center,
            font: crate::app_fonts::monospace_font(),
            ..Default::default()
        });
    }
}

fn lane_bar_gradient(
    color: Color,
    top_y: f32,
    height: f32,
    confidence: f32,
) -> canvas::gradient::Linear {
    canvas::gradient::Linear::new(Point::new(0.0, top_y), Point::new(0.0, top_y + height))
        .add_stop(
            0.0,
            Color {
                a: 0.95 * confidence,
                ..color
            },
        )
        .add_stop(
            1.0,
            Color {
                a: 0.60 * confidence,
                ..color
            },
        )
}

fn draw_win_bullet(
    frame: &mut Frame,
    theme: &Theme,
    layout: &LaneLayout,
    win_rate_pct: f64,
    center_y: f32,
    confidence: f32,
) {
    let x = layout.bullet_left;
    let top = center_y - BULLET_HEIGHT * 0.5;
    frame.fill_rectangle(
        Point::new(x, top),
        Size::new(BULLET_WIDTH, BULLET_HEIGHT),
        Color {
            a: 0.9,
            ..theme.extended_palette().background.strong.color
        },
    );
    let fill_w = (win_rate_pct.clamp(0.0, 100.0) as f32 / 100.0 * BULLET_WIDTH).max(0.0);
    frame.fill_rectangle(
        Point::new(x, top),
        Size::new(fill_w, BULLET_HEIGHT),
        Color {
            a: (0.55 + 0.45 * confidence).min(1.0),
            ..theme.palette().primary
        },
    );
    let tick_x = x + BULLET_WIDTH * 0.5;
    frame.fill_rectangle(
        Point::new(tick_x - 0.5, top - 1.0),
        Size::new(1.0, BULLET_HEIGHT + 2.0),
        Color {
            a: 0.6,
            ..theme.palette().text
        },
    );
}

fn draw_absent_dots(frame: &mut Frame, theme: &Theme, layout: &LaneLayout, center_y: f32) {
    let color = Color {
        a: 0.12,
        ..theme.palette().text
    };
    for k in 0..4 {
        let x = layout.mid_x + (k as f32 - 1.5) * 7.0;
        frame.fill_rectangle(
            Point::new(x - 1.0, center_y - 1.0),
            Size::new(2.0, 2.0),
            color,
        );
    }
}

fn draw_lane_tooltip(
    frame: &mut Frame,
    theme: &Theme,
    size: Size,
    layout: &LaneLayout,
    row: &LaneRow,
    center_y: f32,
) {
    let origin = tooltip_origin(Point::new(layout.mid_x, center_y), size);
    frame.fill_rectangle(
        origin,
        Size::new(TOOLTIP_WIDTH, TOOLTIP_HEIGHT),
        Color {
            a: 0.96,
            ..theme.extended_palette().background.strong.color
        },
    );
    let border = Path::rectangle(origin, Size::new(TOOLTIP_WIDTH, TOOLTIP_HEIGHT));
    frame.stroke(
        &border,
        Stroke::default()
            .with_color(theme.extended_palette().background.weak.color)
            .with_width(1.0),
    );

    let content = if row.sample_count > 0 {
        let mut text = format!(
            "{}\nAvg {}\nWin {:.0}%   n{}",
            row.label,
            helpers::format_signed_percent_value(row.average_return_pct),
            row.win_rate_pct,
            row.sample_count,
        );
        if let Some(dispersion) = row.dispersion_pct {
            text.push_str(&format!("\n\u{00b1}{dispersion:.2}% per session"));
        }
        text
    } else {
        format!("{}\nNo completed sessions", row.label)
    };
    frame.fill_text(Text {
        content,
        position: Point::new(origin.x + 8.0, origin.y + 9.0),
        color: theme.palette().text,
        size: iced::Pixels(10.0),
        font: crate::app_fonts::monospace_font(),
        ..Default::default()
    });
}

fn tooltip_origin(point: Point, size: Size) -> Point {
    let x = if point.x + TOOLTIP_WIDTH + 10.0 > size.width {
        point.x - TOOLTIP_WIDTH - 8.0
    } else {
        point.x + 8.0
    }
    .clamp(0.0, (size.width - TOOLTIP_WIDTH).max(0.0));
    let y = if point.y + TOOLTIP_HEIGHT + 10.0 > size.height {
        point.y - TOOLTIP_HEIGHT - 8.0
    } else {
        point.y + 8.0
    }
    .clamp(0.0, (size.height - TOOLTIP_HEIGHT).max(0.0));
    Point::new(x, y)
}
