use crate::account;
use crate::app_state::TradingTerminal;
use crate::chart_screenshot::{
    PixelPoint, Rect, bitmap_text_width, color_to_rgba, draw_bitmap_text, encode_png_rgba,
    fill_rect,
};
use crate::denomination::DisplayDenominationContext;
use crate::message::Message;

use arboard::{Clipboard, ImageData};
use chrono::Local;
use iced::gradient;
use iced::widget::container as container_style;
use iced::widget::{
    Column, Space, button, checkbox, column, container, radio, row, rule, scrollable, text, tooltip,
};
use iced::{Alignment, Color, Degrees, Element, Fill, Length, Size, Task, Theme, window};
use std::borrow::Cow;
use std::path::PathBuf;

const PNL_CARD_MIN_TEXT_CONTRAST: f32 = 4.5;

// ---------------------------------------------------------------------------
// PnL Card State
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PnlCardTarget {
    Position(String),
    Summary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PnlCardDisplayMode {
    PercentOnly,
    UsdOnly,
    Both,
}

impl PnlCardDisplayMode {
    const ALL: [Self; 3] = [Self::PercentOnly, Self::UsdOnly, Self::Both];

    fn label(self) -> &'static str {
        match self {
            Self::PercentOnly => "% only",
            Self::UsdOnly => "$ only",
            Self::Both => "% + $",
        }
    }
}

impl std::fmt::Display for PnlCardDisplayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PnlCardPercentMode {
    AssetMove,
    Leveraged,
}

impl PnlCardPercentMode {
    const ALL: [Self; 2] = [Self::AssetMove, Self::Leveraged];

    fn label(self) -> &'static str {
        match self {
            Self::AssetMove => "Asset move",
            Self::Leveraged => "By leverage",
        }
    }
}

impl std::fmt::Display for PnlCardPercentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PnlCardWindowState {
    pub(crate) target: PnlCardTarget,
    pub(crate) account_address: String,
    pub(crate) display_mode: PnlCardDisplayMode,
    pub(crate) percent_mode: PnlCardPercentMode,
    pub(crate) obscure_prices: bool,
    pub(crate) show_position_size: bool,
}

impl PnlCardWindowState {
    pub(crate) fn new(target: PnlCardTarget, account_address: String) -> Self {
        Self {
            target,
            account_address,
            display_mode: PnlCardDisplayMode::Both,
            percent_mode: PnlCardPercentMode::Leveraged,
            obscure_prices: true,
            show_position_size: false,
        }
    }
}

// ---------------------------------------------------------------------------
// PnL Card Update
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn open_pnl_card_window(&mut self, target: PnlCardTarget) -> Task<Message> {
        let Some(account_address) = self.current_pnl_card_account_address() else {
            self.push_toast(
                "Connect an account before opening a PnL card".to_string(),
                true,
            );
            return Task::none();
        };

        if let Some(window_id) = self.pnl_card_windows.iter().find_map(|(id, state)| {
            (state.target == target && state.account_address == account_address).then_some(*id)
        }) {
            return window::gain_focus(window_id);
        }

        if !self.pnl_card_target_available(&target) {
            return Task::none();
        }

        let settings = window::Settings {
            size: Size::new(480.0, 640.0),
            ..crate::window_chrome::settings()
        };
        let (window_id, task) = window::open(settings);
        self.pnl_card_windows
            .insert(window_id, PnlCardWindowState::new(target, account_address));

        task.map(Message::WindowOpened)
    }

    pub(crate) fn set_pnl_card_display_mode(
        &mut self,
        window_id: window::Id,
        mode: PnlCardDisplayMode,
    ) -> Task<Message> {
        if let Some(state) = self.pnl_card_windows.get_mut(&window_id) {
            state.display_mode = mode;
        }
        Task::none()
    }

    pub(crate) fn set_pnl_card_percent_mode(
        &mut self,
        window_id: window::Id,
        mode: PnlCardPercentMode,
    ) -> Task<Message> {
        if let Some(state) = self.pnl_card_windows.get_mut(&window_id) {
            state.percent_mode = mode;
        }
        Task::none()
    }

    pub(crate) fn toggle_pnl_card_price_privacy(
        &mut self,
        window_id: window::Id,
        obscure: bool,
    ) -> Task<Message> {
        if let Some(state) = self.pnl_card_windows.get_mut(&window_id) {
            state.obscure_prices = obscure;
        }
        Task::none()
    }

    pub(crate) fn toggle_pnl_card_position_size(
        &mut self,
        window_id: window::Id,
        show: bool,
    ) -> Task<Message> {
        if let Some(state) = self.pnl_card_windows.get_mut(&window_id) {
            state.show_position_size = show;
        }
        Task::none()
    }

    pub(crate) fn copy_pnl_card_image(&mut self, window_id: window::Id) -> Task<Message> {
        let image = match self.pnl_card_export_image(window_id) {
            Ok(image) => image,
            Err(err) => {
                self.push_toast(err, true);
                return Task::none();
            }
        };

        Task::perform(
            async move { copy_pnl_card_to_clipboard(image).map_err(|err| err.to_string()) },
            Message::PnlCardCopied,
        )
    }

    pub(crate) fn save_pnl_card_image(&mut self, window_id: window::Id) -> Task<Message> {
        let image = match self.pnl_card_export_image(window_id) {
            Ok(image) => image,
            Err(err) => {
                self.push_toast(err, true);
                return Task::none();
            }
        };

        Task::perform(save_pnl_card_png(image), Message::PnlCardSaved)
    }

    pub(crate) fn handle_pnl_card_copied(&mut self, result: Result<(), String>) -> Task<Message> {
        match result {
            Ok(()) => self.push_toast("PnL card copied to clipboard".to_string(), false),
            Err(err) => self.push_toast(format!("PnL card copy failed: {err}"), true),
        }
        Task::none()
    }

    pub(crate) fn handle_pnl_card_saved(
        &mut self,
        result: Result<Option<PathBuf>, String>,
    ) -> Task<Message> {
        match result {
            Ok(Some(path)) => {
                self.push_toast(format!("PnL card saved to {}", path.display()), false)
            }
            Ok(None) => {}
            Err(err) => self.push_toast(format!("PnL card save failed: {err}"), true),
        }
        Task::none()
    }

    fn pnl_card_target_available(&self, target: &PnlCardTarget) -> bool {
        match target {
            PnlCardTarget::Position(coin) => self
                .account_data
                .as_ref()
                .is_some_and(|data| position_for_coin(data, coin).is_some()),
            PnlCardTarget::Summary => self.visible_pnl_card_positions().next().is_some(),
        }
    }

    fn current_pnl_card_account_address(&self) -> Option<String> {
        self.connected_address
            .as_deref()
            .and_then(Self::normalize_wallet_address)
    }

    fn pnl_card_account_is_current(&self, state: &PnlCardWindowState) -> bool {
        pnl_card_account_matches(self.connected_address.as_deref(), state)
    }

    fn stale_pnl_card_message(&self, state: &PnlCardWindowState) -> String {
        format!(
            "PnL card was opened for {}. Reopen it for the current account.",
            Self::short_address(&state.account_address)
        )
    }

    fn pnl_card_metrics_for_state(
        &self,
        state: &PnlCardWindowState,
    ) -> Result<PnlCardMetrics, String> {
        if !self.pnl_card_account_is_current(state) {
            return Err(self.stale_pnl_card_message(state));
        }

        match &state.target {
            PnlCardTarget::Position(coin) => self
                .position_pnl_card_metrics(coin)
                .ok_or_else(|| "Position is no longer open".to_string()),
            PnlCardTarget::Summary => self
                .summary_pnl_card_metrics()
                .ok_or_else(|| "No open positions".to_string()),
        }
    }

    fn pnl_card_export_image(&self, window_id: window::Id) -> Result<PnlCardImage, String> {
        let state = self
            .pnl_card_windows
            .get(&window_id)
            .cloned()
            .ok_or_else(|| "PnL card not found".to_string())?;
        let metrics = self.pnl_card_metrics_for_state(&state)?;

        let theme = self.theme();
        let pnl_color = self.direction_color(&theme, metrics.upnl);
        render_pnl_card_image(
            &state,
            metrics,
            self.display_denomination_context(),
            pnl_color,
            &theme,
        )
    }
}

fn pnl_card_account_matches(current_address: Option<&str>, state: &PnlCardWindowState) -> bool {
    current_address
        .and_then(TradingTerminal::normalize_wallet_address)
        .as_deref()
        .is_some_and(|address| address == state.account_address)
}

// ---------------------------------------------------------------------------
// PnL Card Views
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_pnl_card_window(&self, window_id: window::Id) -> Element<'_, Message> {
        let theme = self.theme();
        let Some(state) = self.pnl_card_windows.get(&window_id) else {
            return missing_pnl_card_view(&theme, "PnL card not found");
        };

        let content = self
            .pnl_card_metrics_for_state(state)
            .map(|metrics| self.view_pnl_card_content(window_id, state, metrics, &theme))
            .unwrap_or_else(|message| missing_pnl_card_view(&theme, message));

        container(scrollable(content).width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.palette().background.into()),
                text_color: Some(theme.palette().text),
                ..Default::default()
            })
            .into()
    }

    fn view_pnl_card_content<'a>(
        &'a self,
        window_id: window::Id,
        state: &'a PnlCardWindowState,
        metrics: PnlCardMetrics,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let card = self.view_pnl_card_preview(state, metrics, theme);
        let editor = view_pnl_card_editor(window_id, state, theme);

        column![card, editor]
            .spacing(14)
            .padding(18)
            .width(Fill)
            .height(Length::Shrink)
            .into()
    }

    fn view_pnl_card_preview<'a>(
        &'a self,
        state: &'a PnlCardWindowState,
        metrics: PnlCardMetrics,
        theme: &Theme,
    ) -> Element<'a, Message> {
        let pnl_color = self.direction_color(theme, metrics.upnl);
        let card_palette = pnl_card_palette(theme, pnl_color);
        let text_color = card_palette.text;
        let weak_text = card_palette.weak_text;
        let denomination = self.display_denomination_context();
        let render_text = pnl_card_render_text(state, &metrics, &denomination);
        let ticker = render_text.ticker;
        let leverage_display = render_text.leverage_display;

        let mut value_stack = Column::new()
            .spacing(4)
            .push(
                text(render_text.primary_value)
                    .size(38)
                    .font(crate::app_fonts::monospace_font())
                    .color(text_color),
            )
            .push(
                text(render_text.percent_mode_label)
                    .size(11)
                    .font(crate::app_fonts::monospace_font())
                    .color(weak_text),
            );
        if let Some(secondary) = render_text.secondary_value {
            value_stack = value_stack.push(
                text(secondary)
                    .size(18)
                    .font(crate::app_fonts::monospace_font())
                    .color(text_color),
            );
        }

        let details = container(
            column![
                row![
                    card_metric("Lev", leverage_display, weak_text, text_color),
                    card_metric("Entry", render_text.entry_display, weak_text, text_color),
                    card_metric("Exit", render_text.exit_display, weak_text, text_color),
                ]
                .spacing(10),
                text(render_text.context)
                    .size(11)
                    .font(crate::app_fonts::monospace_font())
                    .color(weak_text),
            ]
            .spacing(8),
        )
        .width(Fill)
        .padding([8, 10])
        .style(move |theme: &Theme| pnl_card_detail_band_style(theme, pnl_color));

        let card_content = column![
            row![
                text("kerosene")
                    .size(18)
                    .font(crate::app_fonts::monospace_font())
                    .color(text_color),
                Space::new().width(Fill),
                text(ticker)
                    .size(24)
                    .font(crate::app_fonts::monospace_font())
                    .color(text_color),
            ]
            .align_y(Alignment::Center),
            Space::new().height(Fill),
            value_stack,
            Space::new().height(Fill),
            details,
        ]
        .spacing(10)
        .width(Fill)
        .height(Fill);

        let inner = container(card_content)
            .width(Fill)
            .height(Fill)
            .padding(18)
            .style(move |theme: &Theme| pnl_card_inner_style(theme, pnl_color));

        container(inner)
            .width(Fill)
            .height(300.0)
            .padding(4)
            .style(move |theme: &Theme| pnl_card_border_style(theme, pnl_color))
            .into()
    }
}

fn view_pnl_card_editor<'a>(
    window_id: window::Id,
    state: &'a PnlCardWindowState,
    theme: &Theme,
) -> Element<'a, Message> {
    let display_modes =
        PnlCardDisplayMode::ALL
            .into_iter()
            .fold(Column::new().spacing(5), |col, mode| {
                col.push(radio(
                    mode.to_string(),
                    mode,
                    Some(state.display_mode),
                    move |selected| Message::SetPnlCardDisplayMode(window_id, selected),
                ))
            });

    let percent_modes =
        PnlCardPercentMode::ALL
            .into_iter()
            .fold(Column::new().spacing(5), |col, mode| {
                col.push(radio(
                    mode.to_string(),
                    mode,
                    Some(state.percent_mode),
                    move |selected| Message::SetPnlCardPercentMode(window_id, selected),
                ))
            });

    let controls = column![
        text("Card display").size(13).color(theme.palette().text),
        rule::horizontal(1),
        row![
            pnl_card_action_button("Copy Image", Message::CopyPnlCard(window_id)),
            pnl_card_action_button("Save PNG", Message::SavePnlCard(window_id)),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        settings_group("PnL", display_modes.into()),
        settings_group("Percent", percent_modes.into()),
        settings_group(
            "Privacy",
            column![
                checkbox(state.obscure_prices)
                    .label("Obscure entry and exit digits")
                    .on_toggle(move |checked| Message::TogglePnlCardPricePrivacy(
                        window_id, checked
                    ))
                    .size(12)
                    .spacing(6)
                    .text_size(12)
                    .font(crate::app_fonts::monospace_font())
                    .width(Fill),
                checkbox(state.show_position_size)
                    .label("Show position size")
                    .on_toggle(move |checked| Message::TogglePnlCardPositionSize(
                        window_id, checked
                    ))
                    .size(12)
                    .spacing(6)
                    .text_size(12)
                    .font(crate::app_fonts::monospace_font())
                    .width(Fill),
            ]
            .spacing(6)
            .into(),
        ),
    ]
    .spacing(10)
    .width(Fill);

    container(controls)
        .width(Fill)
        .padding(12)
        .style(|theme: &Theme| container_style::Style {
            background: Some(theme.extended_palette().background.weak.color.into()),
            border: iced::Border {
                radius: 6.0.into(),
                width: 1.0,
                color: theme.extended_palette().background.strong.color,
            },
            ..Default::default()
        })
        .into()
}

fn pnl_card_action_button(label: &'static str, msg: Message) -> Element<'static, Message> {
    button(text(label).size(12).center())
        .on_press(msg)
        .padding([6, 12])
        .style(|theme: &Theme, status| {
            let bg = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => theme.extended_palette().background.weak.color,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: theme.palette().text,
                border: iced::Border {
                    radius: 4.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

pub(crate) fn pnl_card_icon_button(
    message: Option<Message>,
    tooltip_label: &'static str,
) -> Element<'static, Message> {
    let button = button(text("\u{25F0}").size(10).center())
        .on_press_maybe(message)
        .padding([1, 4])
        .style(|theme: &Theme, status| {
            let background = match status {
                button::Status::Hovered => theme.extended_palette().background.strong.color,
                _ => Color::TRANSPARENT,
            };
            button::Style {
                background: Some(background.into()),
                text_color: theme.palette().primary,
                border: iced::Border {
                    radius: 3.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    tooltip(
        button,
        text(tooltip_label).size(10).font(crate::app_fonts::monospace_font()),
        tooltip::Position::Top,
    )
    .into()
}

fn settings_group<'a>(label: &'static str, content: Element<'a, Message>) -> Element<'a, Message> {
    column![
        text(label)
            .size(10)
            .font(crate::app_fonts::monospace_font())
            .color(Color::from_rgb8(0x88, 0x88, 0x88)),
        content,
    ]
    .spacing(5)
    .width(Fill)
    .into()
}

fn card_metric<'a>(
    label: &'static str,
    value: String,
    label_color: Color,
    value_color: Color,
) -> Element<'a, Message> {
    container(
        column![
            text(label)
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(label_color),
            text(value)
                .size(14)
                .font(crate::app_fonts::monospace_font())
                .color(value_color),
        ]
        .spacing(3),
    )
    .width(Fill)
    .into()
}

fn missing_pnl_card_view<'a>(theme: &Theme, message: impl Into<String>) -> Element<'a, Message> {
    container(
        column![
            text("kerosene").size(18).font(crate::app_fonts::monospace_font()),
            text(message.into())
                .size(12)
                .color(theme.extended_palette().background.weak.text),
        ]
        .spacing(10)
        .padding(18),
    )
    .width(Fill)
    .height(Fill)
    .into()
}

// ---------------------------------------------------------------------------
// PnL Card Metrics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct PnlCardMetrics {
    ticker: String,
    leverage_display: String,
    entry_display: String,
    exit_display: String,
    context: String,
    private_context: Option<String>,
    upnl: f64,
    asset_move_pct: Option<f64>,
    leveraged_pct: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PnlCardRenderText {
    ticker: String,
    leverage_display: String,
    primary_value: String,
    percent_mode_label: &'static str,
    secondary_value: Option<String>,
    entry_display: String,
    exit_display: String,
    context: String,
}

impl TradingTerminal {
    fn position_pnl_card_metrics(&self, coin: &str) -> Option<PnlCardMetrics> {
        let data = self.account_data.as_ref()?;
        let ap = position_for_coin(data, coin)?;
        let pos = &ap.position;
        let numbers = PositionCardNumbers::from_position(self, pos)?;
        let side = if numbers.szi >= 0.0 { "Long" } else { "Short" };
        let leverage = pos.leverage.value.max(1);

        Some(PnlCardMetrics {
            ticker: pos.coin.clone(),
            leverage_display: format!("{leverage}x"),
            entry_display: self.format_display_price(numbers.entry_px),
            exit_display: self.format_display_price(numbers.mark_px),
            context: format!(
                "{side} {}",
                self.display_size_for_symbol(&pos.coin, numbers.szi.abs())
            ),
            private_context: Some(format!("{side} position")),
            upnl: numbers.upnl,
            asset_move_pct: position_asset_move_pct(numbers.szi, numbers.entry_px, numbers.mark_px),
            leveraged_pct: position_asset_move_pct(numbers.szi, numbers.entry_px, numbers.mark_px)
                .map(|pct| pct * f64::from(leverage)),
        })
    }

    fn summary_pnl_card_metrics(&self) -> Option<PnlCardMetrics> {
        let mut count = 0usize;
        let mut upnl = 0.0;
        let mut entry_notional = 0.0;
        let mut margin_basis = 0.0;
        let mut weighted_leverage = 0.0;

        for ap in self.visible_pnl_card_positions() {
            let pos = &ap.position;
            let Some(numbers) = PositionCardNumbers::from_position(self, pos) else {
                continue;
            };
            let leverage = f64::from(pos.leverage.value.max(1));
            let position_entry_notional = numbers.szi.abs() * numbers.entry_px.abs();
            let margin = if numbers.margin_used > 0.0 {
                numbers.margin_used
            } else {
                position_entry_notional / leverage
            };

            count += 1;
            upnl += numbers.upnl;
            entry_notional += position_entry_notional;
            margin_basis += margin;
            weighted_leverage += leverage * position_entry_notional;
        }

        if count == 0 {
            return None;
        }

        let avg_leverage = if entry_notional > f64::EPSILON {
            weighted_leverage / entry_notional
        } else {
            0.0
        };

        Some(PnlCardMetrics {
            ticker: "PORTFOLIO".to_string(),
            leverage_display: if avg_leverage > 0.0 {
                format!("{avg_leverage:.1}x avg")
            } else {
                "Mixed".to_string()
            },
            entry_display: "Mixed".to_string(),
            exit_display: "Live marks".to_string(),
            context: format!("{count} open position{}", if count == 1 { "" } else { "s" }),
            private_context: None,
            upnl,
            asset_move_pct: pct_from_basis(upnl, entry_notional),
            leveraged_pct: pct_from_basis(upnl, margin_basis),
        })
    }

    fn visible_pnl_card_positions(&self) -> impl Iterator<Item = &account::AssetPosition> {
        self.account_data
            .as_ref()
            .into_iter()
            .flat_map(|data| data.clearinghouse.asset_positions.iter())
            .filter(|ap| {
                !self.symbol_key_is_hidden(&ap.position.coin)
                    && (self.show_hidden_positions || !self.position_is_hidden(&ap.position.coin))
            })
    }
}

struct PositionCardNumbers {
    szi: f64,
    entry_px: f64,
    mark_px: f64,
    upnl: f64,
    margin_used: f64,
}

impl PositionCardNumbers {
    fn from_position(terminal: &TradingTerminal, pos: &account::Position) -> Option<Self> {
        let szi = parse_pnl_card_number(&pos.szi)?;
        let entry_px = parse_pnl_card_number(&pos.entry_px)?;
        let wire_upnl = parse_pnl_card_number(&pos.unrealized_pnl);
        let mark_px = terminal
            .resolve_mid_for_symbol(&pos.coin)
            .or_else(|| mark_from_wire_upnl(szi, entry_px, wire_upnl))?;
        let upnl = szi * (mark_px - entry_px);
        let margin_used = parse_pnl_card_number(&pos.margin_used).unwrap_or_default();

        Some(Self {
            szi,
            entry_px,
            mark_px,
            upnl,
            margin_used,
        })
    }
}

// ---------------------------------------------------------------------------
// Image Export
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct PnlCardImage {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    png: Vec<u8>,
    default_filename: String,
}

fn render_pnl_card_image(
    state: &PnlCardWindowState,
    metrics: PnlCardMetrics,
    denomination: DisplayDenominationContext,
    pnl_color: Color,
    theme: &Theme,
) -> Result<PnlCardImage, String> {
    const WIDTH: u32 = 1200;
    const HEIGHT: u32 = 675;

    let mut rgba = vec![0; WIDTH as usize * HEIGHT as usize * 4];
    draw_pnl_card_gradient(&mut rgba, WIDTH, HEIGHT, pnl_color, theme);

    let card_palette = pnl_card_palette(theme, pnl_color);
    let text_rgba = color_to_rgba(card_palette.text, 255);
    let weak_rgba = color_to_rgba(card_palette.weak_text, 232);
    let render_text = pnl_card_render_text(state, &metrics, &denomination);
    let primary_value = export_text(&render_text.primary_value);
    let secondary_value = render_text
        .secondary_value
        .as_ref()
        .map(|value| export_text(value));
    let entry_display = export_text(&render_text.entry_display);
    let exit_display = export_text(&render_text.exit_display);
    let ticker = export_text(&render_text.ticker);
    let context = export_text(&render_text.context);
    let leverage_display = export_text(&render_text.leverage_display);

    draw_pnl_card_export_border(&mut rgba, WIDTH, HEIGHT, pnl_color, theme);

    draw_bitmap_text(
        &mut rgba,
        WIDTH,
        HEIGHT,
        PixelPoint { x: 60, y: 54 },
        5,
        "KEROSENE",
        text_rgba,
    );

    let ticker_scale = best_text_scale(&ticker, 430, 8, 3);
    let ticker_width = bitmap_text_width(&ticker, ticker_scale);
    draw_bitmap_text(
        &mut rgba,
        WIDTH,
        HEIGHT,
        PixelPoint {
            x: WIDTH.saturating_sub(60 + ticker_width),
            y: 48,
        },
        ticker_scale,
        &ticker,
        text_rgba,
    );

    let primary_scale = best_text_scale(&primary_value, WIDTH - 120, 15, 5);
    draw_bitmap_text(
        &mut rgba,
        WIDTH,
        HEIGHT,
        PixelPoint { x: 60, y: 226 },
        primary_scale,
        &primary_value,
        text_rgba,
    );

    let percent_mode = export_text(state.percent_mode.label());
    draw_bitmap_text(
        &mut rgba,
        WIDTH,
        HEIGHT,
        PixelPoint { x: 64, y: 356 },
        4,
        &percent_mode,
        weak_rgba,
    );

    if let Some(secondary_value) = secondary_value {
        draw_bitmap_text(
            &mut rgba,
            WIDTH,
            HEIGHT,
            PixelPoint { x: 60, y: 398 },
            best_text_scale(&secondary_value, WIDTH - 120, 7, 3),
            &secondary_value,
            text_rgba,
        );
    }

    let metric_style = ExportMetricStyle {
        width: WIDTH,
        height: HEIGHT,
        label_color: weak_rgba,
        value_color: text_rgba,
    };
    draw_export_metric(
        &mut rgba,
        metric_style,
        PixelPoint { x: 60, y: 506 },
        "LEV",
        &leverage_display,
    );
    draw_export_metric(
        &mut rgba,
        metric_style,
        PixelPoint { x: 420, y: 506 },
        "ENTRY",
        &entry_display,
    );
    draw_export_metric(
        &mut rgba,
        metric_style,
        PixelPoint { x: 780, y: 506 },
        "EXIT",
        &exit_display,
    );

    let context_scale = best_text_scale(&context, WIDTH - 120, 3, 2);
    draw_bitmap_text(
        &mut rgba,
        WIDTH,
        HEIGHT,
        PixelPoint {
            x: 60,
            y: if context_scale <= 2 { 590 } else { 586 },
        },
        context_scale,
        &context,
        weak_rgba,
    );

    let png = encode_png_rgba(WIDTH, HEIGHT, &rgba)?;
    let default_filename = pnl_card_filename(&metrics.ticker);

    Ok(PnlCardImage {
        width: WIDTH,
        height: HEIGHT,
        rgba,
        png,
        default_filename,
    })
}

#[derive(Debug, Clone, Copy)]
struct ExportMetricStyle {
    width: u32,
    height: u32,
    label_color: [u8; 4],
    value_color: [u8; 4],
}

fn draw_export_metric(
    rgba: &mut [u8],
    style: ExportMetricStyle,
    origin: PixelPoint,
    label: &'static str,
    value: &str,
) {
    draw_bitmap_text(
        rgba,
        style.width,
        style.height,
        origin,
        3,
        label,
        style.label_color,
    );
    draw_bitmap_text(
        rgba,
        style.width,
        style.height,
        PixelPoint {
            x: origin.x,
            y: origin.y + 34,
        },
        best_text_scale(value, 320, 5, 2),
        value,
        style.value_color,
    );
}

fn draw_pnl_card_gradient(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    pnl_color: Color,
    theme: &Theme,
) {
    let card_palette = pnl_card_palette(theme, pnl_color);
    let shadow = mix_color(card_palette.end, Color::BLACK, 0.20);

    for y in 0..height {
        for x in 0..width {
            let t =
                (x as f32 * 0.72 + y as f32 * 0.28) / (width as f32 * 0.72 + height as f32 * 0.28);
            let color = if t < 0.58 {
                mix_color(card_palette.start, card_palette.mid, t / 0.58)
            } else {
                mix_color(card_palette.mid, shadow, (t - 0.58) / 0.42)
            };
            let idx = (y as usize * width as usize + x as usize) * 4;
            rgba[idx] = color_to_byte(color.r);
            rgba[idx + 1] = color_to_byte(color.g);
            rgba[idx + 2] = color_to_byte(color.b);
            rgba[idx + 3] = 255;
        }
    }

    fill_rect(
        rgba,
        width,
        height,
        Rect {
            x: 0,
            y: height.saturating_sub(184),
            width,
            height: 184,
        },
        detail_band_rgba(pnl_card_palette(theme, pnl_color).text, 44),
    );
}

fn draw_pnl_card_export_border(
    rgba: &mut [u8],
    width: u32,
    height: u32,
    pnl_color: Color,
    theme: &Theme,
) {
    let palette = pnl_card_palette(theme, pnl_color);
    let border_width = 22;
    for y in 24..height.saturating_sub(24) {
        for x in 24..width.saturating_sub(24) {
            let in_left = x < 24 + border_width;
            let in_right = x >= width.saturating_sub(24 + border_width);
            let in_top = y < 24 + border_width;
            let in_bottom = y >= height.saturating_sub(24 + border_width);
            if !(in_left || in_right || in_top || in_bottom) {
                continue;
            }

            let t = (x as f32 + y as f32) / (width as f32 + height as f32);
            let color = if t < 0.5 {
                mix_color(palette.border_start, palette.border_mid, t * 2.0)
            } else {
                mix_color(palette.border_mid, palette.border_end, (t - 0.5) * 2.0)
            };
            set_pixel(rgba, width, x, y, color);
        }
    }
}

fn set_pixel(rgba: &mut [u8], width: u32, x: u32, y: u32, color: Color) {
    let idx = (y as usize * width as usize + x as usize) * 4;
    if idx + 3 >= rgba.len() {
        return;
    }

    rgba[idx] = color_to_byte(color.r);
    rgba[idx + 1] = color_to_byte(color.g);
    rgba[idx + 2] = color_to_byte(color.b);
    rgba[idx + 3] = 255;
}

fn mix_color(left: Color, right: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: left.r + (right.r - left.r) * t,
        g: left.g + (right.g - left.g) * t,
        b: left.b + (right.b - left.b) * t,
        a: left.a + (right.a - left.a) * t,
    }
}

fn color_to_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn best_text_scale(text: &str, max_width: u32, preferred: u32, minimum: u32) -> u32 {
    (minimum..=preferred)
        .rev()
        .find(|scale| bitmap_text_width(text, *scale) <= max_width)
        .unwrap_or(minimum)
}

fn export_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            let upper = ch.to_ascii_uppercase();
            if matches!(
                upper,
                'A'..='Z'
                    | '0'..='9'
                    | '/'
                    | ':'
                    | '-'
                    | '_'
                    | '.'
                    | ','
                    | '+'
                    | '$'
                    | '%'
                    | '*'
                    | ' '
            ) {
                upper
            } else if ch.is_whitespace() {
                ' '
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn copy_pnl_card_to_clipboard(image: PnlCardImage) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|err| err.to_string())?;
    clipboard
        .set_image(ImageData {
            width: image.width as usize,
            height: image.height as usize,
            bytes: Cow::Owned(image.rgba),
        })
        .map_err(|err| err.to_string())
}

async fn save_pnl_card_png(image: PnlCardImage) -> Result<Option<PathBuf>, String> {
    let path = rfd::AsyncFileDialog::new()
        .add_filter("PNG image", &["png"])
        .set_file_name(image.default_filename)
        .save_file()
        .await;

    let Some(path) = path else {
        return Ok(None);
    };

    std::fs::write(path.path(), &image.png).map_err(|err| err.to_string())?;
    Ok(Some(path.path().to_path_buf()))
}

fn pnl_card_filename(ticker: &str) -> String {
    let safe_ticker = ticker
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let safe_ticker = if safe_ticker.is_empty() {
        "pnl-card".to_string()
    } else {
        safe_ticker
    };
    format!(
        "kerosene-{safe_ticker}-pnl-card-{}.png",
        Local::now().format("%Y%m%d-%H%M%S")
    )
}

fn position_for_coin<'a>(
    data: &'a account::AccountData,
    coin: &str,
) -> Option<&'a account::AssetPosition> {
    data.clearinghouse
        .asset_positions
        .iter()
        .find(|ap| ap.position.coin == coin)
}

fn parse_pnl_card_number(raw: &str) -> Option<f64> {
    raw.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn mark_from_wire_upnl(szi: f64, entry_px: f64, wire_upnl: Option<f64>) -> Option<f64> {
    if szi.abs() <= f64::EPSILON {
        return None;
    }
    wire_upnl.map(|upnl| entry_px + upnl / szi)
}

fn position_asset_move_pct(szi: f64, entry_px: f64, mark_px: f64) -> Option<f64> {
    if entry_px.abs() <= f64::EPSILON {
        return None;
    }

    let side = if szi >= 0.0 { 1.0 } else { -1.0 };
    Some((mark_px - entry_px) / entry_px * 100.0 * side)
}

fn pct_from_basis(value: f64, basis: f64) -> Option<f64> {
    (basis.abs() > f64::EPSILON).then_some(value / basis * 100.0)
}

impl PnlCardPercentMode {
    fn select(self, asset_move_pct: Option<f64>, leveraged_pct: Option<f64>) -> Option<f64> {
        match self {
            Self::AssetMove => asset_move_pct,
            Self::Leveraged => leveraged_pct,
        }
    }
}

fn pnl_card_render_text(
    state: &PnlCardWindowState,
    metrics: &PnlCardMetrics,
    denomination: &DisplayDenominationContext,
) -> PnlCardRenderText {
    let percent = state
        .percent_mode
        .select(metrics.asset_move_pct, metrics.leveraged_pct);

    PnlCardRenderText {
        ticker: metrics.ticker.clone(),
        leverage_display: metrics.leverage_display.clone(),
        primary_value: pnl_card_primary_value(
            state.display_mode,
            percent,
            metrics.upnl,
            denomination,
        ),
        percent_mode_label: state.percent_mode.label(),
        secondary_value: pnl_card_secondary_value(state.display_mode, metrics.upnl, denomination),
        entry_display: privacy_price_display(&metrics.entry_display, state.obscure_prices),
        exit_display: privacy_price_display(&metrics.exit_display, state.obscure_prices),
        context: pnl_card_context_display(state, metrics),
    }
}

fn pnl_card_context_display(state: &PnlCardWindowState, metrics: &PnlCardMetrics) -> String {
    if state.show_position_size {
        metrics.context.clone()
    } else {
        metrics
            .private_context
            .clone()
            .unwrap_or_else(|| metrics.context.clone())
    }
}

// ---------------------------------------------------------------------------
// Formatting & Styles
// ---------------------------------------------------------------------------

fn pnl_card_primary_value(
    display_mode: PnlCardDisplayMode,
    percent: Option<f64>,
    upnl: f64,
    denomination: &DisplayDenominationContext,
) -> String {
    match display_mode {
        PnlCardDisplayMode::PercentOnly | PnlCardDisplayMode::Both => percent
            .map(format_signed_percent)
            .unwrap_or_else(|| "--%".to_string()),
        PnlCardDisplayMode::UsdOnly => format_signed_usd(upnl, denomination),
    }
}

fn pnl_card_secondary_value(
    display_mode: PnlCardDisplayMode,
    upnl: f64,
    denomination: &DisplayDenominationContext,
) -> Option<String> {
    match display_mode {
        PnlCardDisplayMode::Both => Some(format_signed_usd(upnl, denomination)),
        PnlCardDisplayMode::PercentOnly | PnlCardDisplayMode::UsdOnly => None,
    }
}

fn format_signed_usd(value: f64, denomination: &DisplayDenominationContext) -> String {
    let display_value = if value.abs() < 0.005 { 0.0 } else { value };
    denomination.format_signed_value(display_value, 2)
}

fn format_signed_percent(value: f64) -> String {
    let display_value = if value.abs() < 0.005 { 0.0 } else { value };
    if display_value > 0.0 {
        format!("+{display_value:.2}%")
    } else {
        format!("{display_value:.2}%")
    }
}

fn privacy_price_display(value: &str, obscure: bool) -> String {
    if obscure {
        obscure_price_digits(value)
    } else {
        value.to_string()
    }
}

fn obscure_price_digits(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return value.to_string();
    }

    let (sign, unsigned) = trimmed
        .strip_prefix('-')
        .map(|value| ("-", value))
        .or_else(|| trimmed.strip_prefix('+').map(|value| ("+", value)))
        .unwrap_or(("", trimmed));
    let (whole, fraction) = unsigned
        .rsplit_once('.')
        .map_or((unsigned, None), |(whole, fraction)| {
            (whole, Some(fraction))
        });
    let whole_digits = whole.chars().filter(|ch| ch.is_ascii_digit()).count();
    let whole_is_zero = whole
        .chars()
        .filter(|ch| ch.is_ascii_digit())
        .all(|ch| ch == '0');

    if whole_digits >= 4 {
        return format!("{sign}{}", obscure_last_digits(whole, 2, 'x'));
    }
    if whole_digits >= 2 {
        return format!("{sign}{}", obscure_last_digits(whole, 1, 'x'));
    }
    if whole_digits == 1 && !whole_is_zero {
        return match fraction {
            Some(fraction) if !fraction.is_empty() => {
                format!("{sign}{whole}.{}", "x".repeat(fraction.len().max(2)))
            }
            _ => format!("{sign}x"),
        };
    }

    match fraction {
        Some(fraction) if !fraction.is_empty() => {
            format!("{sign}{whole}.{}", obscure_small_fraction(fraction))
        }
        _ => format!("{sign}x"),
    }
}

fn obscure_small_fraction(fraction: &str) -> String {
    let digit_count = fraction.chars().filter(|ch| ch.is_ascii_digit()).count();
    if digit_count == 0 {
        return fraction.to_string();
    }

    let first_significant_digit = fraction
        .chars()
        .position(|ch| ch.is_ascii_digit() && ch != '0');
    let visible_digits = first_significant_digit
        .map(|idx| idx + 1)
        .unwrap_or_else(|| digit_count.saturating_sub(2))
        .min(digit_count.saturating_sub(2));
    obscure_fraction_after_visible_digits(fraction, visible_digits, 'x')
}

fn obscure_fraction_after_visible_digits(
    value: &str,
    visible_digits: usize,
    mask_char: char,
) -> String {
    let mut seen_digits = 0usize;
    value
        .chars()
        .map(|ch| {
            if !ch.is_ascii_digit() {
                ch
            } else {
                seen_digits += 1;
                if seen_digits > visible_digits {
                    mask_char
                } else {
                    ch
                }
            }
        })
        .collect()
}

fn obscure_last_digits(value: &str, max_digits: usize, mask_char: char) -> String {
    let total_digits = value.chars().filter(|ch| ch.is_ascii_digit()).count();
    let mask_from = total_digits.saturating_sub(max_digits);
    let mut seen_digits = 0usize;

    value
        .chars()
        .map(|ch| {
            if !ch.is_ascii_digit() {
                ch
            } else {
                seen_digits += 1;
                if seen_digits > mask_from {
                    mask_char
                } else {
                    ch
                }
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct PnlCardPalette {
    start: Color,
    mid: Color,
    end: Color,
    border_start: Color,
    border_mid: Color,
    border_end: Color,
    text: Color,
    weak_text: Color,
}

fn pnl_card_palette(theme: &Theme, pnl_color: Color) -> PnlCardPalette {
    let palette = theme.palette();
    let extended = theme.extended_palette();
    let raw_start = mix_color(pnl_color, palette.primary, 0.26);
    let raw_mid = mix_color(
        extended.background.base.color,
        mix_color(pnl_color, palette.primary, 0.34),
        0.42,
    );
    let raw_end = mix_color(extended.background.weak.color, palette.background, 0.52);
    let ([start, mid, end], text) = readable_card_surfaces([raw_start, raw_mid, raw_end]);
    let border_start = mix_color(palette.primary, Color::WHITE, 0.08);
    let border_mid = mix_color(pnl_color, palette.primary, 0.20);
    let border_end = mix_color(extended.background.strong.color, pnl_color, 0.24);
    let weak_text = Color { a: 0.84, ..text };

    PnlCardPalette {
        start,
        mid,
        end,
        border_start,
        border_mid,
        border_end,
        text,
        weak_text,
    }
}

fn readable_card_surfaces(surfaces: [Color; 3]) -> ([Color; 3], Color) {
    let light = Color::WHITE;
    let dark = Color::from_rgb(0.04, 0.04, 0.04);
    let light_surfaces = surfaces.map(|surface| surface_with_min_contrast(surface, light));
    let dark_surfaces = surfaces.map(|surface| surface_with_min_contrast(surface, dark));
    let light_adjustment = contrast_adjustment_score(&surfaces, &light_surfaces);
    let dark_adjustment = contrast_adjustment_score(&surfaces, &dark_surfaces);

    if dark_adjustment < light_adjustment {
        (dark_surfaces, dark)
    } else {
        (light_surfaces, light)
    }
}

fn surface_with_min_contrast(surface: Color, text: Color) -> Color {
    if contrast_ratio(text, surface) >= PNL_CARD_MIN_TEXT_CONTRAST {
        return surface;
    }

    let target = if relative_luminance(text) > 0.5 {
        Color::BLACK
    } else {
        Color::WHITE
    };

    for step in 1..=64 {
        let candidate = mix_color(surface, target, step as f32 / 64.0);
        if contrast_ratio(text, candidate) >= PNL_CARD_MIN_TEXT_CONTRAST {
            return candidate;
        }
    }

    target
}

fn contrast_adjustment_score(original: &[Color; 3], adjusted: &[Color; 3]) -> f32 {
    original
        .iter()
        .zip(adjusted.iter())
        .map(|(left, right)| {
            (left.r - right.r).abs() + (left.g - right.g).abs() + (left.b - right.b).abs()
        })
        .sum()
}

fn pnl_card_detail_band_style(theme: &Theme, pnl_color: Color) -> container_style::Style {
    let palette = pnl_card_palette(theme, pnl_color);
    container_style::Style {
        background: Some(detail_band_color(palette.text, 0.16).into()),
        border: iced::Border {
            radius: 5.0.into(),
            width: 1.0,
            color: Color {
                a: 0.18,
                ..palette.text
            },
        },
        ..Default::default()
    }
}

fn detail_band_color(text_color: Color, alpha: f32) -> Color {
    if relative_luminance(text_color) > 0.5 {
        Color {
            a: alpha,
            ..Color::BLACK
        }
    } else {
        Color {
            a: alpha,
            ..Color::WHITE
        }
    }
}

fn detail_band_rgba(text_color: Color, alpha: u8) -> [u8; 4] {
    if relative_luminance(text_color) > 0.5 {
        [0, 0, 0, alpha]
    } else {
        [255, 255, 255, alpha]
    }
}

fn pnl_card_border_style(theme: &Theme, pnl_color: Color) -> container_style::Style {
    let palette = pnl_card_palette(theme, pnl_color);

    container_style::Style {
        background: Some(
            gradient::Linear::new(Degrees(135.0))
                .add_stop(0.0, palette.border_start)
                .add_stop(0.45, palette.border_mid)
                .add_stop(1.0, palette.border_end)
                .into(),
        ),
        border: iced::Border {
            radius: 10.0.into(),
            width: 1.0,
            color: Color {
                a: 0.42,
                ..palette.border_mid
            },
        },
        ..Default::default()
    }
}

fn pnl_card_inner_style(theme: &Theme, pnl_color: Color) -> container_style::Style {
    let palette = pnl_card_palette(theme, pnl_color);

    container_style::Style {
        background: Some(
            gradient::Linear::new(Degrees(135.0))
                .add_stop(0.0, palette.start)
                .add_stop(0.56, palette.mid)
                .add_stop(1.0, palette.end)
                .into(),
        ),
        border: iced::Border {
            radius: 7.0.into(),
            width: 1.0,
            color: Color {
                a: 0.20,
                ..palette.text
            },
        },
        ..Default::default()
    }
}

#[cfg(test)]
fn minimum_contrast_ratio(text: Color, surfaces: &[Color]) -> f32 {
    surfaces
        .iter()
        .map(|surface| contrast_ratio(text, *surface))
        .fold(f32::INFINITY, f32::min)
}

fn contrast_ratio(left: Color, right: Color) -> f32 {
    let left = relative_luminance(left);
    let right = relative_luminance(right);
    let bright = left.max(right);
    let dark = left.min(right);
    (bright + 0.05) / (dark + 0.05)
}

fn relative_luminance(color: Color) -> f32 {
    fn channel(value: f32) -> f32 {
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_metrics() -> PnlCardMetrics {
        PnlCardMetrics {
            ticker: "BTC".to_string(),
            leverage_display: "20x".to_string(),
            entry_display: "82,543.2".to_string(),
            exit_display: "84,612.8".to_string(),
            context: "Short 0.52 BTC".to_string(),
            private_context: Some("Short position".to_string()),
            upnl: 1076.19,
            asset_move_pct: Some(2.51),
            leveraged_pct: Some(50.14),
        }
    }

    fn test_account() -> String {
        "0x1111111111111111111111111111111111111111".to_string()
    }

    fn other_account() -> String {
        "0x2222222222222222222222222222222222222222".to_string()
    }

    #[test]
    fn pnl_card_window_defaults_are_privacy_first() {
        let state =
            PnlCardWindowState::new(PnlCardTarget::Position("BTC".to_string()), test_account());

        assert_eq!(state.target, PnlCardTarget::Position("BTC".to_string()));
        assert_eq!(state.account_address, test_account());
        assert_eq!(state.display_mode, PnlCardDisplayMode::Both);
        assert_eq!(state.percent_mode, PnlCardPercentMode::Leveraged);
        assert!(state.obscure_prices);
        assert!(!state.show_position_size);
    }

    #[test]
    fn pnl_card_account_binding_rejects_current_account_switch() {
        let state =
            PnlCardWindowState::new(PnlCardTarget::Position("BTC".to_string()), test_account());

        assert!(pnl_card_account_matches(
            Some(&test_account().to_uppercase()),
            &state
        ));
        assert!(!pnl_card_account_matches(Some(&other_account()), &state));
        assert!(!pnl_card_account_matches(None, &state));
    }

    #[test]
    fn render_text_default_card_uses_leveraged_percent_usd_and_private_context() {
        let state =
            PnlCardWindowState::new(PnlCardTarget::Position("BTC".to_string()), test_account());
        let denomination = DisplayDenominationContext::default();
        let render_text = pnl_card_render_text(&state, &sample_metrics(), &denomination);

        assert_eq!(render_text.ticker, "BTC");
        assert_eq!(render_text.leverage_display, "20x");
        assert_eq!(render_text.primary_value, "+50.14%");
        assert_eq!(render_text.percent_mode_label, "By leverage");
        assert_eq!(render_text.secondary_value, Some("+$1,076.19".to_string()));
        assert_eq!(render_text.entry_display, "82,5xx");
        assert_eq!(render_text.exit_display, "84,6xx");
        assert_eq!(render_text.context, "Short position");
    }

    #[test]
    fn render_text_can_show_asset_move_only_with_exact_prices_and_position_size() {
        let mut state =
            PnlCardWindowState::new(PnlCardTarget::Position("BTC".to_string()), test_account());
        state.display_mode = PnlCardDisplayMode::PercentOnly;
        state.percent_mode = PnlCardPercentMode::AssetMove;
        state.obscure_prices = false;
        state.show_position_size = true;

        let denomination = DisplayDenominationContext::default();
        let render_text = pnl_card_render_text(&state, &sample_metrics(), &denomination);

        assert_eq!(render_text.primary_value, "+2.51%");
        assert_eq!(render_text.percent_mode_label, "Asset move");
        assert_eq!(render_text.secondary_value, None);
        assert_eq!(render_text.entry_display, "82,543.2");
        assert_eq!(render_text.exit_display, "84,612.8");
        assert_eq!(render_text.context, "Short 0.52 BTC");
    }

    #[test]
    fn render_text_can_show_usd_only_without_secondary_value() {
        let mut state =
            PnlCardWindowState::new(PnlCardTarget::Position("ETH".to_string()), test_account());
        let mut metrics = sample_metrics();
        state.display_mode = PnlCardDisplayMode::UsdOnly;
        metrics.upnl = -42.5;

        let denomination = DisplayDenominationContext::default();
        let render_text = pnl_card_render_text(&state, &metrics, &denomination);

        assert_eq!(render_text.primary_value, "-$42.50");
        assert_eq!(render_text.secondary_value, None);
    }

    #[test]
    fn render_text_preserves_usd_when_percent_basis_is_missing() {
        let state = PnlCardWindowState::new(PnlCardTarget::Summary, test_account());
        let mut metrics = sample_metrics();
        metrics.asset_move_pct = None;
        metrics.leveraged_pct = None;

        let denomination = DisplayDenominationContext::default();
        let render_text = pnl_card_render_text(&state, &metrics, &denomination);

        assert_eq!(render_text.primary_value, "--%");
        assert_eq!(render_text.secondary_value, Some("+$1,076.19".to_string()));
    }

    #[test]
    fn position_asset_move_is_side_adjusted() {
        assert_eq!(position_asset_move_pct(2.0, 100.0, 110.0), Some(10.0));
        assert_eq!(position_asset_move_pct(-2.0, 100.0, 90.0), Some(10.0));
        assert_eq!(position_asset_move_pct(-2.0, 100.0, 110.0), Some(-10.0));
    }

    #[test]
    fn mark_can_be_reconstructed_from_wire_upnl() {
        assert_eq!(mark_from_wire_upnl(2.0, 100.0, Some(20.0)), Some(110.0));
        assert_eq!(mark_from_wire_upnl(-2.0, 100.0, Some(20.0)), Some(90.0));
        assert_eq!(mark_from_wire_upnl(0.0, 100.0, Some(20.0)), None);
    }

    #[test]
    fn pct_from_basis_rejects_zero_basis() {
        assert_eq!(pct_from_basis(50.0, 1_000.0), Some(5.0));
        assert_eq!(pct_from_basis(50.0, 0.0), None);
    }

    #[test]
    fn privacy_price_display_can_be_disabled() {
        assert_eq!(privacy_price_display("82,543.2", true), "82,5xx");
        assert_eq!(privacy_price_display("82,543.2", false), "82,543.2");
    }

    #[test]
    fn price_privacy_obscures_large_prices_to_hundreds() {
        assert_eq!(obscure_price_digits("82,543.2"), "82,5xx");
        assert_eq!(obscure_price_digits("12,345.7"), "12,3xx");
        assert_eq!(obscure_price_digits("-12,345.7"), "-12,3xx");
        assert_eq!(obscure_price_digits("1,234.5"), "1,2xx");
    }

    #[test]
    fn price_privacy_scales_across_mid_price_denominations() {
        assert_eq!(obscure_price_digits("825.42"), "82x");
        assert_eq!(obscure_price_digits("82.54"), "8x");
        assert_eq!(obscure_price_digits("8.254"), "8.xxx");
        assert_eq!(obscure_price_digits("8"), "x");
    }

    #[test]
    fn price_privacy_keeps_only_early_significant_sub_dollar_digits() {
        assert_eq!(obscure_price_digits("0.123456"), "0.1xxxxx");
        assert_eq!(obscure_price_digits("0.012345"), "0.01xxxx");
        assert_eq!(obscure_price_digits("0.00001234"), "0.00001xxx");
        assert_eq!(obscure_price_digits("0.0000"), "0.00xx");
    }

    #[test]
    fn pnl_card_context_hides_position_size_by_default() {
        let mut state =
            PnlCardWindowState::new(PnlCardTarget::Position("BTC".to_string()), test_account());
        let metrics = sample_metrics();

        assert_eq!(pnl_card_context_display(&state, &metrics), "Short position");

        state.show_position_size = true;

        assert_eq!(pnl_card_context_display(&state, &metrics), "Short 0.52 BTC");
    }

    #[test]
    fn summary_context_is_not_replaced_by_position_privacy_text() {
        let state = PnlCardWindowState::new(PnlCardTarget::Summary, test_account());
        let mut metrics = sample_metrics();
        metrics.context = "3 open positions".to_string();
        metrics.private_context = None;

        assert_eq!(
            pnl_card_context_display(&state, &metrics),
            "3 open positions"
        );
    }

    #[test]
    fn export_text_keeps_card_glyphs_and_sanitizes_unsupported_characters() {
        assert_eq!(
            export_text("BTC +50.14% / $1,076.19"),
            "BTC +50.14% / $1,076.19"
        );
        assert_eq!(export_text("xyz:BTC→USD"), "XYZ:BTC-USD");
    }

    #[test]
    fn filename_sanitizes_asset_ticker() {
        let filename = pnl_card_filename("xyz:BTC/USD");

        assert!(filename.starts_with("kerosene-xyz-btc-usd-pnl-card-"));
        assert!(filename.ends_with(".png"));
    }

    #[test]
    fn render_pnl_card_image_produces_expected_png_payload() {
        let state =
            PnlCardWindowState::new(PnlCardTarget::Position("BTC".to_string()), test_account());
        let image = render_pnl_card_image(
            &state,
            sample_metrics(),
            DisplayDenominationContext::default(),
            Color::from_rgb8(0x50, 0xfa, 0x7b),
            &Theme::Dark,
        )
        .expect("pnl card image renders");

        assert_eq!(image.width, 1200);
        assert_eq!(image.height, 675);
        assert_eq!(image.rgba.len(), 1200 * 675 * 4);
        assert!(image.png.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(image.default_filename.starts_with("kerosene-btc-pnl-card-"));
        assert!(image.default_filename.ends_with(".png"));
    }

    #[test]
    fn positive_and_negative_exports_use_distinct_gradients() {
        let state =
            PnlCardWindowState::new(PnlCardTarget::Position("BTC".to_string()), test_account());
        let positive = render_pnl_card_image(
            &state,
            sample_metrics(),
            DisplayDenominationContext::default(),
            Color::from_rgb8(0x50, 0xfa, 0x7b),
            &Theme::Dark,
        )
        .expect("positive pnl card renders");
        let negative = render_pnl_card_image(
            &state,
            sample_metrics(),
            DisplayDenominationContext::default(),
            Color::from_rgb8(0xff, 0x55, 0x55),
            &Theme::Dark,
        )
        .expect("negative pnl card renders");

        assert_ne!(&positive.rgba[0..64], &negative.rgba[0..64]);
    }

    #[test]
    fn card_palette_keeps_text_readable_across_builtin_themes() {
        let pnl_colors = [
            Color::from_rgb8(0x50, 0xfa, 0x7b),
            Color::from_rgb8(0xff, 0x55, 0x55),
        ];

        for theme in Theme::ALL {
            for pnl_color in pnl_colors {
                let palette = pnl_card_palette(theme, pnl_color);
                let min_contrast = minimum_contrast_ratio(
                    palette.text,
                    &[palette.start, palette.mid, palette.end],
                );

                assert!(
                    min_contrast >= 4.5,
                    "theme {theme:?} contrast {min_contrast:.2} is too low"
                );
            }
        }
    }
}
