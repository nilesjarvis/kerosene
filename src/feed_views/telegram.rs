use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::symbol_mentions::SymbolAliasSource;
use crate::telegram_feed::{
    TELEGRAM_COUNTRY_CODES, TelegramChannelProfile, TelegramFastAuthStage, TelegramFeedPost,
    TelegramFeedScreen, TelegramPostMedia, TelegramPrivateChannelCandidate, masked_telegram_phone,
    telegram_age_countdown_label, telegram_arrival_latency_label, telegram_new_message_heat,
    telegram_price_impact_pct,
};
use iced::widget::container as container_style;
use iced::widget::image::Handle as ImageHandle;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Space, button, canvas, column, container, image, pick_list, responsive, row, rule, scrollable,
    stack, text, text_input, tooltip,
};
use iced::{
    Alignment, Background, Border, Color, ContentFit, Element, Fill, Length, Point, Rectangle,
    Renderer, Size, Theme,
};

// ---- Layout constants ----
const TELEGRAM_COMPACT_CONTROLS_WIDTH: f32 = 360.0;
// Past this many subscribed channels the chip list collapses by default so a
// long list does not dominate the pane.
const TELEGRAM_CHANNEL_COLLAPSE_THRESHOLD: usize = 4;
const TELEGRAM_MEDIA_MAX_HEIGHT: f32 = 240.0;
const TELEGRAM_MEDIA_PLACEHOLDER_HEIGHT: f32 = 56.0;
const TELEGRAM_PRIVATE_CANDIDATE_LIST_HEIGHT: f32 = 150.0;
const TELEGRAM_CODE_CELL_HEIGHT: f32 = 52.0;
const TELEGRAM_AVATAR_SIZE: f32 = 32.0;
const TELEGRAM_SPARKLINE_WIDTH: f32 = 34.0;
const TELEGRAM_SPARKLINE_HEIGHT: f32 = 13.0;
const TELEGRAM_RESEND_COOLDOWN_MS: u64 = 60_000;
const TELEGRAM_NEW_BADGE_HEAT: f32 = 0.55;

// ----------------------------------------------------------------------------
// Palette
// ----------------------------------------------------------------------------

/// The Telegram-feed colour set, derived once per render from the active theme so
/// leaf builders stay theme-aware without re-deriving the palette each call.
#[derive(Debug, Clone, Copy)]
struct TelegramColors {
    primary: Color,
    orange_soft: Color,
    text: Color,
    text_bright: Color,
    muted: Color,
    dim: Color,
    up: Color,
    down: Color,
    warn: Color,
    border: Color,
    border_orange: Color,
    sunken: Color,
    panel: Color,
}

impl TelegramColors {
    fn from_theme(theme: &Theme) -> Self {
        let palette = theme.palette();
        let ext = theme.extended_palette();
        let primary = palette.primary;
        let text = palette.text;
        let muted = ext.background.weak.text;
        Self {
            primary,
            orange_soft: blend_color(primary, Color::WHITE, 0.5),
            text,
            text_bright: blend_color(text, Color::WHITE, 0.4),
            muted,
            dim: Color { a: 0.7, ..muted },
            up: palette.success,
            down: palette.danger,
            warn: Color::from_rgb8(0xff, 0xb6, 0x48),
            border: Color { a: 0.1, ..text },
            border_orange: Color { a: 0.34, ..primary },
            sunken: ext.background.weak.color,
            panel: ext.background.base.color,
        }
    }
}

// ----------------------------------------------------------------------------
// Impact-chip data
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct TelegramTickerImpactCard {
    symbol: String,
    ticker: String,
    matched_text: String,
    source: SymbolAliasSource,
    confidence: u8,
    impact_pct: Option<f64>,
    is_outcome: bool,
    sparkline: Vec<f32>,
}

// ----------------------------------------------------------------------------
// Entry + screen routing
// ----------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_telegram_feed(&self) -> Element<'_, Message> {
        let now_ms = self.status_bar_now_ms;
        container(responsive(move |size| {
            self.view_telegram_feed_sized(now_ms, size.width)
        }))
        .width(Fill)
        .height(Fill)
        .into()
    }

    fn view_telegram_feed_sized(&self, now_ms: u64, available_width: f32) -> Element<'_, Message> {
        let colors = TelegramColors::from_theme(&self.theme());
        match self.telegram_feed.current_screen() {
            TelegramFeedScreen::Connect => self.view_telegram_connect(colors),
            TelegramFeedScreen::SignInPhone => {
                self.view_telegram_sign_in_phone(colors, available_width)
            }
            TelegramFeedScreen::SignInCode => self.view_telegram_sign_in_code(colors, now_ms),
            TelegramFeedScreen::LiveFeed => {
                self.view_telegram_live_feed(colors, now_ms, available_width)
            }
        }
    }

    /// Status chip rendered into the pane title-bar controls (see `main_view/grid`).
    pub(crate) fn view_telegram_status_chip(&self) -> Element<'_, Message> {
        let colors = TelegramColors::from_theme(&self.theme());
        match self.telegram_feed.current_screen() {
            TelegramFeedScreen::Connect => {
                telegram_status_chip("PUBLIC MODE", colors.dim, colors.muted, colors.border, None)
            }
            TelegramFeedScreen::SignInPhone | TelegramFeedScreen::SignInCode => {
                telegram_status_chip(
                    "CONNECTING",
                    colors.warn,
                    colors.warn,
                    Color {
                        a: 0.5,
                        ..colors.warn
                    },
                    None,
                )
            }
            TelegramFeedScreen::LiveFeed if self.telegram_feed.signed_in() => telegram_status_chip(
                "FAST · LIVE",
                colors.up,
                colors.up,
                Color {
                    a: 0.4,
                    ..colors.up
                },
                Some((Message::TelegramFastSignOut, "Sign out of Fast Mode")),
            ),
            TelegramFeedScreen::LiveFeed => telegram_status_chip(
                "PUBLIC",
                colors.dim,
                colors.muted,
                colors.border,
                Some((
                    Message::TelegramFeedShowOnboarding,
                    "Connect Telegram for live updates",
                )),
            ),
        }
    }

    // ------------------------------------------------------------------------
    // Screen 1 — Connect (onboarding)
    // ------------------------------------------------------------------------

    fn view_telegram_connect(&self, colors: TelegramColors) -> Element<'_, Message> {
        let icon_tile = container(telegram_zap_icon(26.0, colors.primary, false))
            .center(54.0)
            .style(move |_t: &Theme| telegram_icon_tile_style(colors));

        let connect_button = button(
            row![
                telegram_zap_icon(15.0, theme_on_orange(colors), true),
                text("Connect Telegram").size(13),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .on_press(Message::ToggleTelegramFastFeed)
        .padding([10, 14])
        .width(Fill)
        .style(move |_t: &Theme, status| telegram_primary_button(colors, status));

        let public_button = button(text("Continue on public mode").size(13).center())
            .on_press(Message::TelegramFeedDismissOnboarding)
            .padding([9, 14])
            .width(Fill)
            .style(move |_t: &Theme, status| telegram_quiet_button(colors, status));

        let hero = column![
            icon_tile,
            text("Go real-time")
                .size(24)
                .color(colors.text_bright)
                .center(),
            text(
                "Telegram Feed works on public channels with no login, polling \
                 every 15 seconds. Turn on Fast Mode to stream new posts the \
                 instant they're sent."
            )
            .size(13)
            .color(colors.muted)
            .center()
            .width(Length::Fixed(320.0)),
            column![connect_button, public_button]
                .spacing(9)
                .width(Length::Fixed(320.0)),
            text(
                "Sign-in uses Telegram's API. Your session is stored locally; \
                 the API hash is never written to config."
            )
            .size(10)
            .font(crate::app_fonts::monospace_font())
            .color(colors.dim)
            .center()
            .width(Length::Fixed(320.0)),
        ]
        .spacing(15)
        .align_x(Alignment::Center);

        let cards = row![
            telegram_info_card("PUBLIC", "No login · polls t.me every ~15s", false, colors,),
            telegram_info_card(
                "FAST MODE",
                "Live updates · phone + code sign-in",
                true,
                colors,
            ),
        ]
        .spacing(10)
        .width(Fill);

        let body = column![
            container(hero)
                .width(Fill)
                .height(Fill)
                .center_x(Fill)
                .center_y(Fill),
            cards,
        ]
        .spacing(18)
        .width(Fill)
        .height(Fill);

        container(body)
            .width(Fill)
            .height(Fill)
            .padding([26, 24])
            .into()
    }

    // ------------------------------------------------------------------------
    // Screen 2 — Sign in (phone)
    // ------------------------------------------------------------------------

    fn view_telegram_sign_in_phone(
        &self,
        colors: TelegramColors,
        available_width: f32,
    ) -> Element<'_, Message> {
        let header = telegram_sign_in_header(colors, 1, 2);

        let heading = column![
            text("Sign in to Telegram")
                .size(23)
                .color(colors.text_bright),
            text(
                "Enter the phone number for your Telegram account. A login code \
                 will be sent to your Telegram app."
            )
            .size(13)
            .color(colors.muted),
        ]
        .spacing(7)
        .width(Fill);

        let country_options: Vec<String> = TELEGRAM_COUNTRY_CODES
            .iter()
            .map(|c| c.to_string())
            .collect();
        let country = pick_list(
            country_options,
            Some(self.telegram_feed.fast_country_code.clone()),
            Message::TelegramFastCountryCodeChanged,
        )
        .text_size(13)
        .padding([10, 12])
        .width(Length::Fixed(86.0));

        let phone = text_input("415 813 2207", &self.telegram_feed.fast_phone_input)
            .style(telegram_focus_input_style)
            .on_input(|value| Message::TelegramFastPhoneChanged(value.into()))
            .on_submit(Message::TelegramFastRequestCode)
            .font(crate::app_fonts::monospace_font())
            .size(14)
            .padding([10, 13])
            .width(Fill);

        let phone_field = column![
            text("PHONE NUMBER")
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(colors.dim),
            row![country, phone].spacing(8).align_y(Alignment::Center),
        ]
        .spacing(6)
        .width(Fill);

        let mut send_button = button(text("Send login code").size(13).center())
            .padding([10, 14])
            .width(Fill)
            .style(move |_t: &Theme, status| telegram_primary_button(colors, status));
        if !self.telegram_feed.fast_auth_in_flight {
            send_button = send_button.on_press(Message::TelegramFastRequestCode);
        }

        let advanced = self.view_telegram_advanced_credentials(colors, available_width);

        let footer = telegram_note_card(
            "Prefer no login? Public mode keeps working — Fast Mode only adds \
             real-time delivery on top of it.",
            colors,
        );

        let mut content = column![header, heading, phone_field, send_button, advanced]
            .spacing(18)
            .width(Fill);
        if let Some(status) = self.telegram_fast_error_text(colors) {
            content = content.push(status);
        }
        content = content.push(Space::new().height(Fill)).push(footer);

        container(content)
            .width(Fill)
            .height(Fill)
            .padding([20, 24])
            .into()
    }

    fn view_telegram_advanced_credentials(
        &self,
        colors: TelegramColors,
        available_width: f32,
    ) -> Element<'_, Message> {
        let expanded = self.telegram_feed.fast_advanced_expanded;
        let chevron = if expanded { "⌄" } else { "›" };
        let mut header_row = row![
            text(chevron)
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(colors.dim),
            text("Advanced — use my own API credentials")
                .size(12)
                .color(colors.text),
            Space::new().width(Fill),
        ]
        .spacing(9)
        .align_y(Alignment::Center)
        .width(Fill);
        if crate::telegram_fast_feed::bundled_telegram_api_id().is_some() {
            header_row = header_row.push(telegram_bundled_badge(colors));
        }

        let header = button(header_row)
            .on_press(Message::ToggleTelegramFastAdvanced)
            .padding([10, 12])
            .width(Fill)
            .style(move |_t: &Theme, status| telegram_sunken_button(colors, status));

        if !expanded {
            return container(header)
                .style(move |_t: &Theme| telegram_sunken_outline_style(colors))
                .width(Fill)
                .into();
        }

        let api_id = text_input("API ID", &self.telegram_feed.fast_api_id_input)
            .style(telegram_focus_input_style)
            .on_input(|value| Message::TelegramFastApiIdChanged(value.into()))
            .size(12)
            .padding([8, 10])
            .width(Fill);
        let api_hash = text_input("API hash", &self.telegram_feed.fast_api_hash_input)
            .style(telegram_focus_input_style)
            .on_input(|value| Message::TelegramFastApiHashChanged(value.into()))
            .secure(true)
            .size(12)
            .padding([8, 10])
            .width(Fill);
        let inputs: Element<'_, Message> = if available_width < TELEGRAM_COMPACT_CONTROLS_WIDTH {
            column![api_id, api_hash].spacing(8).width(Fill).into()
        } else {
            row![api_id, api_hash].spacing(8).width(Fill).into()
        };

        container(
            column![
                header,
                container(inputs).padding([0, 12]),
                Space::new().height(2),
            ]
            .spacing(8)
            .width(Fill),
        )
        .style(move |_t: &Theme| telegram_sunken_outline_style(colors))
        .padding(iced::Padding {
            top: 0.0,
            right: 0.0,
            bottom: 8.0,
            left: 0.0,
        })
        .width(Fill)
        .into()
    }

    // ------------------------------------------------------------------------
    // Screen 3 — Sign in (code)
    // ------------------------------------------------------------------------

    fn view_telegram_sign_in_code(
        &self,
        colors: TelegramColors,
        now_ms: u64,
    ) -> Element<'_, Message> {
        let header = telegram_sign_in_header(colors, 2, 2);

        let masked = masked_telegram_phone(&self.telegram_feed.fast_phone_input);
        let heading = column![
            text("Enter your code").size(23).color(colors.text_bright),
            row![
                text("Telegram sent a 5-digit code to ")
                    .size(13)
                    .color(colors.muted),
                text(masked)
                    .size(13)
                    .font(crate::app_fonts::monospace_font())
                    .color(colors.text),
            ]
            .wrap(),
        ]
        .spacing(7)
        .width(Fill);

        let cells = self.view_telegram_code_cells(colors);

        let resend = self.view_telegram_resend_row(colors, now_ms);

        let mut content = column![header, heading, cells, resend]
            .spacing(18)
            .width(Fill);

        if matches!(
            self.telegram_feed.fast_auth_stage,
            TelegramFastAuthStage::PasswordRequired
        ) {
            content = content.push(rule::horizontal(1));
            content = content.push(self.view_telegram_2fa_field(colors));
        }

        let verify_message = if matches!(
            self.telegram_feed.fast_auth_stage,
            TelegramFastAuthStage::PasswordRequired
        ) {
            Message::TelegramFastSubmitPassword
        } else {
            Message::TelegramFastSubmitCode
        };
        let mut verify = button(text("Verify & connect").size(13).center())
            .padding([10, 14])
            .width(Fill)
            .style(move |_t: &Theme, status| telegram_primary_button(colors, status));
        if !self.telegram_feed.fast_auth_in_flight {
            verify = verify.on_press(verify_message);
        }
        content = content.push(verify);

        if let Some(status) = self.telegram_fast_error_text(colors) {
            content = content.push(status);
        }

        content = content.push(Space::new().height(Fill)).push(
            text("Session saved locally to telegram_fast.session, permission-tightened.")
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(colors.dim)
                .center()
                .width(Fill),
        );

        container(content)
            .width(Fill)
            .height(Fill)
            .padding([20, 24])
            .into()
    }

    /// Five digit cells driven by a single transparent text-input overlay: the
    /// cells render the typed digits (and the active-cell caret) while the
    /// invisible input on the bottom layer captures focus and keystrokes.
    fn view_telegram_code_cells(&self, colors: TelegramColors) -> Element<'_, Message> {
        let code = self.telegram_feed.fast_code_input.as_str();
        let digits: Vec<char> = code.chars().collect();

        let mut cells = row![].spacing(9).width(Fill);
        for index in 0..5usize {
            let entered = digits.get(index).copied();
            let is_active = digits.len() < 5 && index == digits.len();
            let inner: Element<'_, Message> = match entered {
                Some(ch) => text(ch.to_string())
                    .size(22)
                    .font(crate::app_fonts::monospace_font())
                    .color(colors.text)
                    .into(),
                None if is_active => container(Space::new().width(2.0).height(24.0))
                    .style(move |_t: &Theme| container_style::Style {
                        background: Some(colors.primary.into()),
                        ..Default::default()
                    })
                    .into(),
                None => Space::new().into(),
            };
            cells = cells.push(
                container(inner)
                    .center_x(Fill)
                    .center_y(Fill)
                    .width(Fill)
                    .height(TELEGRAM_CODE_CELL_HEIGHT)
                    .style(move |_t: &Theme| telegram_code_cell_style(colors, is_active)),
            );
        }

        let capture = text_input("", code)
            .style(telegram_transparent_input_style)
            .on_input(|value| Message::TelegramFastCodeChanged(value.into()))
            .on_submit(Message::TelegramFastSubmitCode)
            .size(20)
            .padding([15, 0])
            .width(Fill);

        stack![capture, cells].width(Fill).into()
    }

    fn view_telegram_resend_row(
        &self,
        colors: TelegramColors,
        now_ms: u64,
    ) -> Element<'_, Message> {
        let remaining = self
            .telegram_feed
            .fast_code_sent_at_ms
            .map(|sent| TELEGRAM_RESEND_COOLDOWN_MS.saturating_sub(now_ms.saturating_sub(sent)))
            .unwrap_or(0);

        let resend: Element<'_, Message> =
            if remaining == 0 && !self.telegram_feed.fast_auth_in_flight {
                button(
                    text("Resend code")
                        .size(11)
                        .font(crate::app_fonts::monospace_font())
                        .color(colors.orange_soft),
                )
                .on_press(Message::TelegramFastRequestCode)
                .padding(0)
                .style(telegram_link_button)
                .into()
            } else {
                let secs = remaining.div_ceil(1_000);
                text(format!("Resend code in 0:{secs:02}"))
                    .size(11)
                    .font(crate::app_fonts::monospace_font())
                    .color(colors.dim)
                    .into()
            };

        let edit = button(
            text("Edit number")
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(colors.orange_soft),
        )
        .on_press(Message::TelegramFastEditNumber)
        .padding(0)
        .style(telegram_link_button);

        row![resend, Space::new().width(Fill), edit]
            .align_y(Alignment::Center)
            .width(Fill)
            .into()
    }

    fn view_telegram_2fa_field(&self, colors: TelegramColors) -> Element<'_, Message> {
        let placeholder = self
            .telegram_feed
            .fast_password_hint
            .as_ref()
            .map(|hint| format!("Two-step password ({hint})"))
            .unwrap_or_else(|| "Two-step password".to_string());
        let password = text_input(
            placeholder.as_str(),
            &self.telegram_feed.fast_password_input,
        )
        .style(telegram_focus_input_style)
        .on_input(|value| Message::TelegramFastPasswordChanged(value.into()))
        .on_submit(Message::TelegramFastSubmitPassword)
        .secure(true)
        .size(13)
        .padding([10, 13])
        .width(Fill);

        column![
            row![
                text("TWO-STEP PASSWORD")
                    .size(10)
                    .font(crate::app_fonts::monospace_font())
                    .color(colors.dim),
                text(" · if enabled").size(10).color(colors.dim),
            ]
            .wrap(),
            password,
        ]
        .spacing(6)
        .width(Fill)
        .into()
    }

    fn telegram_fast_error_text(&self, colors: TelegramColors) -> Option<Element<'_, Message>> {
        let (message, is_error) = self.telegram_feed.fast_status.as_ref()?;
        if !is_error {
            return None;
        }
        Some(
            text(message.clone())
                .size(11)
                .color(colors.down)
                .width(Fill)
                .into(),
        )
    }

    // ------------------------------------------------------------------------
    // Screen 4 — Live feed
    // ------------------------------------------------------------------------

    fn view_telegram_live_feed(
        &self,
        colors: TelegramColors,
        now_ms: u64,
        available_width: f32,
    ) -> Element<'_, Message> {
        let mut header = column![
            self.view_telegram_add_bar(colors, available_width),
            self.view_telegram_channel_chips(colors),
        ]
        .spacing(9)
        .width(Fill);
        if let Some(error) = &self.telegram_feed.last_error {
            header = header.push(text(error.clone()).size(11).color(colors.down).width(Fill));
        }
        let header = container(header)
            .width(Fill)
            .padding([11, 12])
            .style(move |_t: &Theme| telegram_section_divider_style(colors));

        let mut content = column![header, self.view_telegram_meta_strip(colors)].width(Fill);

        // Private-channel scanning stays available in Fast Mode without crowding
        // the redesigned bar.
        if let Some(private) = self.view_telegram_private_section(colors) {
            content = content.push(private);
        }

        content = content.push(self.view_telegram_feed_body(colors, now_ms));

        container(content).width(Fill).height(Fill).into()
    }

    fn view_telegram_add_bar(
        &self,
        colors: TelegramColors,
        available_width: f32,
    ) -> Element<'_, Message> {
        let input = row![
            text("@")
                .size(13)
                .font(crate::app_fonts::monospace_font())
                .color(colors.dim),
            text_input("add a public channel…", &self.telegram_feed.channel_input)
                .style(telegram_transparent_field_style)
                .on_input(Message::TelegramFeedChannelInputChanged)
                .on_submit(Message::TelegramFeedAddChannel)
                .font(crate::app_fonts::monospace_font())
                .size(12)
                .width(Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        let input = container(input)
            .width(Fill)
            .padding([8, 11])
            .style(move |_t: &Theme| telegram_well_style(colors));

        let add = button(text("Add").size(12).center())
            .on_press(Message::TelegramFeedAddChannel)
            .padding([8, 15])
            .style(move |_t: &Theme, status| telegram_accent_button(colors, status));

        let alerts = self.view_telegram_alerts_toggle(colors);
        let outcomes = self.view_telegram_outcomes_toggle(colors);
        let refresh = self.view_telegram_refresh_button(colors);

        if available_width < TELEGRAM_COMPACT_CONTROLS_WIDTH {
            column![
                input,
                row![add, alerts, outcomes, Space::new().width(Fill), refresh]
                    .spacing(7)
                    .align_y(Alignment::Center),
            ]
            .spacing(7)
            .width(Fill)
            .into()
        } else {
            row![input, add, alerts, outcomes, refresh]
                .spacing(7)
                .align_y(Alignment::Center)
                .width(Fill)
                .into()
        }
    }

    fn view_telegram_alerts_toggle(&self, colors: TelegramColors) -> Element<'_, Message> {
        let on = self.telegram_feed.notifications_enabled;
        tooltip(
            button(text("Alerts").size(11).center())
                .on_press(Message::ToggleTelegramFeedNotifications)
                .padding([8, 11])
                .style(move |_t: &Theme, status| telegram_chip_toggle_button(colors, status, on)),
            text(if on { "Alerts on" } else { "Alerts off" }).size(10),
            tooltip::Position::Top,
        )
        .into()
    }

    fn view_telegram_outcomes_toggle(&self, colors: TelegramColors) -> Element<'_, Message> {
        let on = self.telegram_feed.include_outcome_markets;
        tooltip(
            button(text("Outcomes").size(11).center())
                .on_press(Message::ToggleTelegramFeedOutcomeMarkets)
                .padding([8, 11])
                .style(move |_t: &Theme, status| telegram_chip_toggle_button(colors, status, on)),
            text("Show prediction-market chips").size(10),
            tooltip::Position::Top,
        )
        .into()
    }

    fn view_telegram_refresh_button(&self, colors: TelegramColors) -> Element<'_, Message> {
        let content: Element<'_, Message> = if self.telegram_feed.loading() {
            self.view_spinner(13)
        } else {
            text("\u{21bb}")
                .size(14)
                .center()
                .font(crate::app_fonts::monospace_font())
                .into()
        };
        let mut refresh = button(content)
            .padding([7, 10])
            .style(move |_t: &Theme, status| telegram_icon_button(colors, status));
        if !self.telegram_feed.channel_refresh_in_flight() {
            refresh = refresh.on_press(Message::RefreshTelegramFeed);
        }
        tooltip(refresh, text("Refresh").size(10), tooltip::Position::Top).into()
    }

    fn view_telegram_channel_chips(&self, colors: TelegramColors) -> Element<'_, Message> {
        let count = self.telegram_feed.selected_channel_count();
        let collapsible = count > TELEGRAM_CHANNEL_COLLAPSE_THRESHOLD;
        let expanded = self.telegram_feed.channels_expanded;

        // Collapsed: show only a compact toggle so a long channel list does not
        // take over the pane. The add bar above still adds, the meta strip below
        // still shows the count.
        if collapsible && !expanded {
            return telegram_channels_collapse_header(count, false, colors);
        }

        let mut chips = row![].spacing(7).align_y(Alignment::Center);
        for channel in &self.telegram_feed.channels {
            let active = self
                .telegram_feed
                .loading_channels
                .iter()
                .any(|loading| loading == channel);
            chips = chips.push(telegram_channel_chip(
                channel.clone(),
                format!("@{channel}"),
                active,
                self.telegram_feed.channel_profiles.get(channel).cloned(),
                colors,
            ));
        }
        for channel in &self.telegram_feed.private_channels {
            let key = channel.key();
            let profile = self.telegram_feed.channel_profiles.get(&key).cloned();
            let label = profile
                .as_ref()
                .map(|profile| profile.title.clone())
                .unwrap_or_else(|| channel.title.clone());
            chips = chips.push(telegram_channel_chip(key, label, false, profile, colors));
        }
        // Trailing dashed "+ add" affordance (the live input above performs the
        // actual add; this is a visual cue that the row is editable).
        chips = chips.push(
            container(
                text("+ add")
                    .size(11)
                    .font(crate::app_fonts::monospace_font())
                    .color(colors.dim),
            )
            .padding([5, 10])
            .style(move |_t: &Theme| telegram_add_pill_style(colors)),
        );
        let chips = chips.wrap().vertical_spacing(7);

        if collapsible {
            column![
                telegram_channels_collapse_header(count, true, colors),
                chips
            ]
            .spacing(7)
            .width(Fill)
            .into()
        } else {
            chips.into()
        }
    }

    fn view_telegram_meta_strip(&self, colors: TelegramColors) -> Element<'_, Message> {
        let count = self.telegram_feed.selected_channel_count();
        let alerts = if self.telegram_feed.notifications_enabled {
            "ALERTS ON"
        } else {
            "ALERTS OFF"
        };
        let mode = if self.telegram_feed.signed_in() {
            "FAST MODE"
        } else {
            "PUBLIC"
        };
        let left = text(format!("{count} CHANNELS · {alerts} · {mode}"))
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(colors.dim);

        let (status_label, status_color) = if self.telegram_feed.signed_in() {
            ("streaming", colors.up)
        } else {
            ("polling", colors.muted)
        };
        let right = row![
            container(Space::new().width(5.0).height(5.0))
                .style(move |_t: &Theme| telegram_dot_style(status_color)),
            text(status_label)
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(status_color),
        ]
        .spacing(5)
        .align_y(Alignment::Center);

        container(
            row![left, Space::new().width(Fill), right]
                .align_y(Alignment::Center)
                .width(Fill),
        )
        .width(Fill)
        .padding([7, 12])
        .style(move |_t: &Theme| telegram_section_divider_style(colors))
        .into()
    }

    fn view_telegram_private_section(
        &self,
        colors: TelegramColors,
    ) -> Option<Element<'_, Message>> {
        if !self.telegram_feed.fast_mode_enabled {
            return None;
        }
        let candidates = self.telegram_feed.available_private_channel_candidates();
        let scan_status = self.telegram_private_scan_status(colors);
        if candidates.is_empty() && scan_status.is_none() {
            // Still expose a scan trigger so private channels remain reachable.
            let scan = self.view_telegram_private_scan_button(colors)?;
            return Some(
                container(scan)
                    .width(Fill)
                    .padding([7, 12])
                    .style(move |_t: &Theme| telegram_section_divider_style(colors))
                    .into(),
            );
        }

        let mut content = column![].spacing(7).width(Fill);
        if let Some(scan) = self.view_telegram_private_scan_button(colors) {
            content = content.push(scan);
        }
        if let Some(status) = scan_status {
            content = content.push(status);
        }
        if !candidates.is_empty() {
            content = content.push(telegram_private_candidate_selector(
                candidates,
                self.telegram_feed.private_channel_candidates_expanded,
                colors,
            ));
        }
        Some(
            container(content)
                .width(Fill)
                .padding([8, 12])
                .style(move |_t: &Theme| telegram_section_divider_style(colors))
                .into(),
        )
    }

    fn view_telegram_private_scan_button(
        &self,
        colors: TelegramColors,
    ) -> Option<Element<'_, Message>> {
        let signed_in = self.telegram_feed.signed_in();
        let content: Element<'_, Message> = if self.telegram_feed.private_channel_candidates_loading
        {
            row![
                self.view_spinner(12),
                text("Scanning private channels…")
                    .size(11)
                    .color(colors.muted),
            ]
            .spacing(7)
            .align_y(Alignment::Center)
            .into()
        } else {
            text(if signed_in {
                "Scan private channels"
            } else {
                "Sign in to scan private channels"
            })
            .size(11)
            .color(colors.muted)
            .into()
        };
        let mut scan = button(content)
            .padding([6, 10])
            .style(move |_t: &Theme, status| telegram_icon_button(colors, status));
        if signed_in && !self.telegram_feed.private_channel_candidates_loading {
            scan = scan.on_press(Message::TelegramPrivateChannelsRefresh);
        }
        Some(scan.into())
    }

    fn telegram_private_scan_status(
        &self,
        colors: TelegramColors,
    ) -> Option<Element<'static, Message>> {
        let (message, is_error) = self.telegram_feed.fast_status.as_ref()?;
        let scan_related = self.telegram_feed.private_channel_candidates_loading
            || message == "Scanning Telegram channels"
            || (message.starts_with("Found ") && message.contains("private Telegram channels"))
            || message.contains("private channels")
            || message.contains("Telegram channel list failed");
        if !scan_related {
            return None;
        }
        let color = if *is_error { colors.down } else { colors.muted };
        Some(
            text(message.clone())
                .size(11)
                .color(color)
                .width(Fill)
                .into(),
        )
    }

    fn view_telegram_feed_body(&self, colors: TelegramColors, now_ms: u64) -> Element<'_, Message> {
        let posts = self.telegram_feed.visible_posts();
        if posts.is_empty() {
            let label = if self.telegram_feed.loading() {
                "Loading posts…"
            } else if self.telegram_feed.selected_channel_count() == 0 {
                "Add a public channel to start the feed"
            } else {
                "No posts found"
            };
            return container(text(label).size(12).color(colors.muted))
                .width(Fill)
                .height(Fill)
                .center_x(Fill)
                .center_y(Fill)
                .into();
        }

        let rows = posts
            .iter()
            .enumerate()
            .fold(column![].width(Fill), |rows, (index, post)| {
                let profile = self
                    .telegram_feed
                    .channel_profiles
                    .get(&post.channel)
                    .cloned();
                let impacts = self.telegram_ticker_impact_cards(post);
                let mut rows = rows;
                if index > 0 {
                    rows = rows.push(
                        rule::horizontal(1).style(move |_t: &Theme| telegram_rule_style(colors)),
                    );
                }
                rows.push(telegram_post_card(
                    post.clone(),
                    profile,
                    impacts,
                    now_ms,
                    colors,
                ))
            });

        scrollable(rows)
            .direction(Direction::Vertical(
                Scrollbar::new().width(4).margin(0).scroller_width(4),
            ))
            .height(Fill)
            .into()
    }

    fn telegram_ticker_impact_cards(
        &self,
        post: &TelegramFeedPost,
    ) -> Vec<TelegramTickerImpactCard> {
        let include_outcomes = self.telegram_feed.include_outcome_markets;
        post.ticker_mentions
            .iter()
            .filter_map(|mention| {
                let symbol = self
                    .resolve_exchange_symbol_by_key_or_ticker(&mention.symbol)
                    .filter(|symbol| {
                        symbol.market_type != MarketType::Spot
                            && (include_outcomes || symbol.market_type != MarketType::Outcome)
                            && self.exchange_symbol_is_orderable(symbol)
                    })?;
                let is_outcome = symbol.outcome.is_some();
                let ticker = if is_outcome {
                    Self::exchange_symbol_display_name(symbol)
                } else {
                    mention.ticker.clone()
                };
                // Mid freshness must be judged against the real wall clock,
                // because mids are stamped with it on arrival.
                let current = self.resolve_mid_for_symbol(&mention.symbol);
                let impact_pct = telegram_price_impact_pct(mention.reference_price, current);
                let sparkline = self.telegram_impact_sparkline(
                    &mention.symbol,
                    mention.reference_price,
                    mention.reference_seen_ms,
                    post.first_seen_ms,
                    current,
                );
                Some(TelegramTickerImpactCard {
                    symbol: mention.symbol.clone(),
                    ticker,
                    matched_text: mention.matched_text.clone(),
                    source: mention.source,
                    confidence: mention.confidence,
                    impact_pct,
                    is_outcome,
                    sparkline,
                })
            })
            .collect()
    }

    /// Build the impact-chip sparkline: the first-seen anchor price, the recorded
    /// mids since first-seen (1/min from the screener history), and the current
    /// mid. Returns fewer than two points only when there is nothing to plot.
    fn telegram_impact_sparkline(
        &self,
        symbol: &str,
        reference_price: Option<f64>,
        reference_seen_ms: u64,
        first_seen_ms: u64,
        current: Option<f64>,
    ) -> Vec<f32> {
        let from_ms = if reference_seen_ms > 0 {
            reference_seen_ms
        } else {
            first_seen_ms
        };
        let candidates = self.mid_candidates_for_symbol(symbol);
        let mut values = Vec::new();
        if let Some(reference) = reference_price {
            values.push(reference as f32);
        }
        values.extend(self.screener.mid_samples_since(&candidates, from_ms));
        if let Some(current) = current {
            values.push(current as f32);
        }
        values
    }
}

// ----------------------------------------------------------------------------
// Sign-in shared pieces
// ----------------------------------------------------------------------------

fn telegram_sign_in_header(
    colors: TelegramColors,
    step: u16,
    total: u16,
) -> Element<'static, Message> {
    let back = button(
        text("‹ Back")
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(colors.dim),
    )
    .on_press(Message::TelegramFeedShowOnboarding)
    .padding([2, 4])
    .style(telegram_link_button);

    let step_row = row![
        text(format!("STEP {step} / {total}"))
            .size(10)
            .font(crate::app_fonts::monospace_font())
            .color(colors.orange_soft),
        telegram_progress_bar(step, total, colors),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Fill);

    column![row![back, Space::new().width(Fill)].width(Fill), step_row,]
        .spacing(12)
        .width(Fill)
        .into()
}

fn telegram_progress_bar(
    step: u16,
    total: u16,
    colors: TelegramColors,
) -> Element<'static, Message> {
    let fill =
        container(Space::new().height(3.0)).style(move |_t: &Theme| container_style::Style {
            background: Some(colors.primary.into()),
            border: Border {
                radius: 2.0.into(),
                ..Default::default()
            },
            ..Default::default()
        });
    let bar: Element<'static, Message> = if step >= total {
        fill.width(Fill).into()
    } else {
        let remaining = total.saturating_sub(step).max(1);
        row![
            fill.width(Length::FillPortion(step.max(1))),
            Space::new().width(Length::FillPortion(remaining)),
        ]
        .width(Fill)
        .into()
    };

    container(bar)
        .width(Fill)
        .height(3.0)
        .style(move |_t: &Theme| container_style::Style {
            background: Some(colors.sunken.into()),
            border: Border {
                radius: 2.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn telegram_bundled_badge(colors: TelegramColors) -> Element<'static, Message> {
    container(
        text("BUNDLED")
            .size(9)
            .font(crate::app_fonts::monospace_font())
            .color(colors.up),
    )
    .padding([1, 5])
    .style(move |_t: &Theme| container_style::Style {
        background: Some(
            Color {
                a: 0.14,
                ..colors.up
            }
            .into(),
        ),
        border: Border {
            radius: 3.0.into(),
            width: 1.0,
            color: Color {
                a: 0.4,
                ..colors.up
            },
        },
        ..Default::default()
    })
    .into()
}

fn telegram_info_card(
    eyebrow: &str,
    body: &str,
    accent: bool,
    colors: TelegramColors,
) -> Element<'static, Message> {
    let eyebrow_color = if accent {
        colors.orange_soft
    } else {
        colors.muted
    };
    container(
        column![
            text(eyebrow.to_string())
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(eyebrow_color),
            text(body.to_string())
                .size(11)
                .color(if accent { colors.muted } else { colors.dim }),
        ]
        .spacing(5)
        .width(Fill),
    )
    .width(Fill)
    .padding([11, 12])
    .style(move |_t: &Theme| telegram_info_card_style(colors, accent))
    .into()
}

fn telegram_note_card(body: &str, colors: TelegramColors) -> Element<'static, Message> {
    container(
        text(body.to_string())
            .size(11)
            .color(colors.dim)
            .width(Fill),
    )
    .width(Fill)
    .padding([11, 12])
    .style(move |_t: &Theme| container_style::Style {
        background: Some(
            Color {
                a: 0.012,
                ..colors.text
            }
            .into(),
        ),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: colors.border,
        },
        ..Default::default()
    })
    .into()
}

// ----------------------------------------------------------------------------
// Status chip + channel chips
// ----------------------------------------------------------------------------

fn telegram_status_chip(
    label: &str,
    dot_color: Color,
    text_color: Color,
    border_color: Color,
    action: Option<(Message, &'static str)>,
) -> Element<'static, Message> {
    let content = row![
        container(Space::new().width(6.0).height(6.0))
            .style(move |_t: &Theme| telegram_dot_style(dot_color)),
        text(label.to_string())
            .size(10)
            .font(crate::app_fonts::monospace_font())
            .color(text_color),
    ]
    .spacing(5)
    .align_y(Alignment::Center);

    match action {
        Some((message, tip)) => tooltip(
            button(content)
                .on_press(message)
                .padding([2, 7])
                .style(move |_t: &Theme, status| telegram_status_chip_button(border_color, status)),
            text(tip).size(10),
            tooltip::Position::Bottom,
        )
        .into(),
        None => container(content)
            .padding([2, 7])
            .style(move |_t: &Theme| container_style::Style {
                background: Some(
                    Color {
                        a: 0.03,
                        ..Color::WHITE
                    }
                    .into(),
                ),
                border: Border {
                    radius: 3.0.into(),
                    width: 1.0,
                    color: border_color,
                },
                ..Default::default()
            })
            .into(),
    }
}

fn telegram_channel_chip(
    channel: String,
    label: String,
    active: bool,
    profile: Option<TelegramChannelProfile>,
    colors: TelegramColors,
) -> Element<'static, Message> {
    let label_color = if active { colors.primary } else { colors.text };
    let avatar = telegram_channel_avatar(profile.as_ref(), &channel, 17.0, colors);
    let remove = button(
        text("✕")
            .size(9)
            .font(crate::app_fonts::monospace_font())
            .color(colors.dim),
    )
    .on_press(Message::TelegramFeedRemoveChannel(channel.into()))
    .padding([1, 3])
    .style(telegram_link_button);

    container(
        row![avatar, text(label).size(11).color(label_color), remove]
            .spacing(6)
            .align_y(Alignment::Center),
    )
    .padding([3, 8])
    .style(move |_t: &Theme| telegram_pill_style(colors))
    .into()
}

/// Compact toggle for the channel chip list. Collapsed it stands alone; expanded
/// it sits above the chips. Clicking either form flips `channels_expanded`.
fn telegram_channels_collapse_header(
    count: usize,
    expanded: bool,
    colors: TelegramColors,
) -> Element<'static, Message> {
    let chevron = if expanded { "⌄" } else { "›" };
    let toggle = if expanded { "Hide" } else { "Show" };
    button(
        row![
            text(chevron)
                .size(12)
                .font(crate::app_fonts::monospace_font())
                .color(colors.dim),
            text(format!("{count} channels"))
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(colors.muted),
            Space::new().width(Fill),
            text(toggle)
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(colors.orange_soft),
        ]
        .spacing(7)
        .align_y(Alignment::Center)
        .width(Fill),
    )
    .on_press(Message::ToggleTelegramFeedChannelsExpanded)
    .padding([5, 9])
    .width(Fill)
    .style(move |_t: &Theme, status| telegram_icon_button(colors, status))
    .into()
}

fn telegram_private_candidate_selector(
    candidates: Vec<TelegramPrivateChannelCandidate>,
    expanded: bool,
    colors: TelegramColors,
) -> Element<'static, Message> {
    let count = candidates.len();
    let toggle_label = if expanded { "Hide" } else { "Show" };
    let header = container(
        row![
            text(format!("{count} private channels"))
                .size(11)
                .color(colors.muted),
            Space::new().width(Fill),
            button(text(toggle_label).size(10).center())
                .on_press(Message::ToggleTelegramPrivateChannelCandidatesExpanded)
                .padding([1, 7])
                .style(move |_t: &Theme, status| telegram_icon_button(colors, status)),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([3, 6])
    .style(move |_t: &Theme| telegram_pill_style(colors));

    if !expanded {
        return header.into();
    }

    let rows = candidates
        .into_iter()
        .fold(column![].spacing(4).width(Fill), |rows, candidate| {
            rows.push(telegram_private_candidate_chip(candidate, colors))
        });
    column![
        header,
        scrollable(rows)
            .direction(Direction::Vertical(
                Scrollbar::new().width(4).margin(0).scroller_width(4),
            ))
            .height(TELEGRAM_PRIVATE_CANDIDATE_LIST_HEIGHT)
            .width(Fill),
    ]
    .spacing(6)
    .width(Fill)
    .into()
}

fn telegram_private_candidate_chip(
    candidate: TelegramPrivateChannelCandidate,
    colors: TelegramColors,
) -> Element<'static, Message> {
    let peer_id = candidate.peer_id;
    let title = candidate.title;
    let avatar = telegram_private_candidate_avatar(candidate.avatar_handle, &title, 18.0, colors);
    container(
        row![
            avatar,
            text(title).size(11).color(colors.text),
            Space::new().width(Fill),
            button(text("+").size(11).center().color(colors.orange_soft))
                .on_press(Message::TelegramFeedAddPrivateChannel(peer_id))
                .padding([0, 5])
                .style(telegram_link_button),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([3, 6])
    .style(move |_t: &Theme| telegram_pill_style(colors))
    .into()
}

fn telegram_private_candidate_avatar(
    avatar_handle: Option<ImageHandle>,
    title: &str,
    size: f32,
    colors: TelegramColors,
) -> Element<'static, Message> {
    if let Some(handle) = avatar_handle {
        return container(
            image(handle)
                .width(size)
                .height(size)
                .content_fit(ContentFit::Cover)
                .border_radius(size / 2.0),
        )
        .width(size)
        .height(size)
        .clip(true)
        .into();
    }
    telegram_channel_avatar(None, title, size, colors)
}

fn telegram_channel_avatar(
    profile: Option<&TelegramChannelProfile>,
    channel: &str,
    size: f32,
    colors: TelegramColors,
) -> Element<'static, Message> {
    if let Some(handle) = profile.and_then(|profile| profile.avatar_handle.as_ref()) {
        return container(
            image(handle.clone())
                .width(size)
                .height(size)
                .content_fit(ContentFit::Cover)
                .border_radius(size / 2.0),
        )
        .width(size)
        .height(size)
        .clip(true)
        .into();
    }
    let initials = profile
        .map(|profile| profile.initials.clone())
        .filter(|initials| !initials.trim().is_empty())
        .unwrap_or_else(|| channel.chars().take(2).collect::<String>().to_uppercase());
    container(
        text(initials)
            .size(size * 0.42)
            .font(crate::app_fonts::monospace_font())
            .color(colors.text)
            .center(),
    )
    .center(size)
    .style(move |_t: &Theme| telegram_avatar_placeholder_style(colors))
    .into()
}

// ----------------------------------------------------------------------------
// Message card
// ----------------------------------------------------------------------------

fn telegram_post_card(
    post: TelegramFeedPost,
    profile: Option<TelegramChannelProfile>,
    impacts: Vec<TelegramTickerImpactCard>,
    now_ms: u64,
    colors: TelegramColors,
) -> Element<'static, Message> {
    let channel = format!("@{}", post.channel);
    let title = profile
        .as_ref()
        .map(|profile| profile.title.clone())
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| channel.clone());
    let age = telegram_age_countdown_label(post.timestamp_ms, now_ms);
    let latency = telegram_arrival_latency_label(&post);
    let heat = telegram_new_message_heat(post.first_seen_ms, now_ms);
    let url = post.url.clone();

    let avatar = telegram_channel_avatar(
        profile.as_ref(),
        &post.channel,
        TELEGRAM_AVATAR_SIZE,
        colors,
    );

    let mut name_row = row![text(title).size(13).color(colors.orange_soft)]
        .spacing(7)
        .align_y(Alignment::Center);
    if heat >= TELEGRAM_NEW_BADGE_HEAT {
        name_row = name_row.push(telegram_new_badge(colors));
    }

    let mut meta_row = row![
        text(format!("{channel} · {age} ago"))
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(colors.dim),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    if let Some(latency) = latency {
        meta_row = meta_row.push(
            text(format!("seen {latency}"))
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(colors.up),
        );
    }

    let identity = column![name_row, meta_row.wrap()].spacing(1).width(Fill);

    let link = tooltip(
        button(
            text("Link")
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(colors.dim),
        )
        .on_press(Message::CopyToClipboard(url.into()))
        .padding([1, 4])
        .style(telegram_link_button),
        text("Copy link").size(10),
        tooltip::Position::Top,
    );

    let header = row![avatar, identity, link]
        .spacing(10)
        .align_y(Alignment::Start)
        .width(Fill);

    let mut content = column![header].spacing(8).width(Fill);
    if !post.text.trim().is_empty() {
        content = content.push(
            container(text(post.text).size(13).color(colors.text).width(Fill))
                .padding(left_pad(42.0)),
        );
    }
    if let Some(media) = post.media {
        content = content.push(
            container(telegram_post_media_view(media, post.url.clone(), colors))
                .padding(left_pad(42.0)),
        );
    }
    if !impacts.is_empty() {
        content =
            content.push(container(telegram_impact_chips(impacts, colors)).padding(left_pad(42.0)));
    }

    container(content)
        .width(Fill)
        .padding([13, 14])
        .style(move |_t: &Theme| telegram_post_row_style(colors, heat))
        .into()
}

fn telegram_new_badge(colors: TelegramColors) -> Element<'static, Message> {
    container(
        text("NEW")
            .size(8)
            .font(crate::app_fonts::monospace_font())
            .color(theme_on_orange(colors)),
    )
    .padding([1, 4])
    .style(move |_t: &Theme| container_style::Style {
        background: Some(colors.primary.into()),
        border: Border {
            radius: 3.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn telegram_post_media_view(
    media: TelegramPostMedia,
    post_url: String,
    colors: TelegramColors,
) -> Element<'static, Message> {
    if let Some(handle) = media.handle {
        return button(
            container(
                image(handle)
                    .width(Fill)
                    .height(TELEGRAM_MEDIA_MAX_HEIGHT)
                    .content_fit(ContentFit::Contain)
                    .border_radius(6.0),
            )
            .width(Fill)
            .clip(true),
        )
        .on_press(Message::CopyToClipboard(post_url.into()))
        .padding(0)
        .width(Fill)
        .style(telegram_media_button)
        .into();
    }

    let label = if media.failed_at_ms.is_some() {
        "[media unavailable]"
    } else {
        media.kind.placeholder_label()
    };
    container(text(label).size(11).color(colors.muted))
        .width(Fill)
        .height(TELEGRAM_MEDIA_PLACEHOLDER_HEIGHT)
        .center_x(Fill)
        .center_y(Fill)
        .style(move |_t: &Theme| telegram_media_placeholder_style(colors))
        .into()
}

fn telegram_impact_chips(
    impacts: Vec<TelegramTickerImpactCard>,
    colors: TelegramColors,
) -> Element<'static, Message> {
    // Keep perp and outcome (prediction) markets on their own rows.
    let (outcome, normal): (Vec<_>, Vec<_>) =
        impacts.into_iter().partition(|impact| impact.is_outcome);
    let mut groups = column![].spacing(7).width(Fill);
    for group in [normal, outcome] {
        if group.is_empty() {
            continue;
        }
        let chips = group
            .into_iter()
            .fold(row![].spacing(7).width(Fill), |chips, impact| {
                chips.push(telegram_impact_chip(impact, colors))
            })
            .wrap()
            .vertical_spacing(7);
        groups = groups.push(chips);
    }
    groups.into()
}

fn telegram_impact_chip(
    impact: TelegramTickerImpactCard,
    colors: TelegramColors,
) -> Element<'static, Message> {
    let symbol = impact.symbol.clone();
    let pct = impact.impact_pct;
    let sign_color = match pct {
        Some(value) if value >= 0.0 => colors.up,
        Some(_) => colors.down,
        None => colors.muted,
    };
    let arrow = match pct {
        Some(value) if value >= 0.0 => "\u{25B2}",
        Some(_) => "\u{25BC}",
        None => "",
    };
    let pct_label = match pct {
        Some(value) => format!("{arrow} {value:+.2}%"),
        None => "—".to_string(),
    };
    let ticker_color = if telegram_source_is_fuzzy(impact.source) {
        colors.muted
    } else {
        colors.text
    };

    let mut chip_row = row![
        container(Space::new().width(7.0).height(7.0))
            .style(move |_t: &Theme| telegram_dot_style(sign_color)),
        text(impact.ticker.clone())
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(ticker_color),
        text(pct_label)
            .size(11)
            .font(crate::app_fonts::monospace_font())
            .color(sign_color),
    ]
    .spacing(7)
    .align_y(Alignment::Center);

    if pct.is_some() && impact.sparkline.len() >= 2 {
        chip_row = chip_row.push(
            canvas(TelegramImpactSparkline {
                values: impact.sparkline.clone(),
                color: sign_color,
            })
            .width(Length::Fixed(TELEGRAM_SPARKLINE_WIDTH))
            .height(Length::Fixed(TELEGRAM_SPARKLINE_HEIGHT)),
        );
    }

    let chip = button(chip_row)
        .on_press(Message::SymbolSelected(symbol))
        .padding([3, 9])
        .style(move |_t: &Theme, status| telegram_impact_chip_button(colors, status));

    if let Some(label) = telegram_ticker_match_tooltip(&impact) {
        tooltip(chip, text(label).size(10), tooltip::Position::Top).into()
    } else {
        chip.into()
    }
}

fn telegram_ticker_match_tooltip(impact: &TelegramTickerImpactCard) -> Option<String> {
    let matched = impact.matched_text.trim();
    if matched.is_empty()
        || (impact.source == SymbolAliasSource::Ticker
            && matched.eq_ignore_ascii_case(&impact.ticker))
    {
        return None;
    }
    Some(format!(
        "Matched \"{}\" as {} · confidence {}%",
        matched,
        telegram_symbol_alias_source_label(impact.source),
        impact.confidence
    ))
}

fn telegram_source_is_fuzzy(source: SymbolAliasSource) -> bool {
    matches!(
        source,
        SymbolAliasSource::DisplayName
            | SymbolAliasSource::Keyword
            | SymbolAliasSource::CuratedKeyword
    )
}

fn telegram_symbol_alias_source_label(source: SymbolAliasSource) -> &'static str {
    match source {
        SymbolAliasSource::Ticker => "ticker",
        SymbolAliasSource::Key => "symbol key",
        SymbolAliasSource::KeySuffix => "symbol key",
        SymbolAliasSource::DisplayName => "display name",
        SymbolAliasSource::Keyword => "keyword",
        SymbolAliasSource::CuratedKeyword => "curated keyword",
    }
}

// ----------------------------------------------------------------------------
// Sparkline + zap canvas
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct TelegramImpactSparkline {
    values: Vec<f32>,
    color: Color,
}

impl canvas::Program<Message> for TelegramImpactSparkline {
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
        draw_telegram_sparkline(&mut frame, bounds.size(), &self.values, self.color);
        vec![frame.into_geometry()]
    }
}

fn draw_telegram_sparkline(frame: &mut canvas::Frame, size: Size, values: &[f32], color: Color) {
    if size.width <= 1.0 || size.height <= 1.0 || values.len() < 2 {
        return;
    }
    let pad = 1.5_f32;
    let plot_w = (size.width - pad * 2.0).max(1.0);
    let plot_h = (size.height - pad * 2.0).max(1.0);
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

/// Lucide "zap" bolt, drawn from the design's exact polygon. `filled` paints the
/// glyph (for the orange button); otherwise it is stroked (for the icon tile).
fn telegram_zap_icon(size: f32, color: Color, filled: bool) -> Element<'static, Message> {
    canvas(TelegramZapIcon { color, filled })
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .into()
}

#[derive(Debug, Clone)]
struct TelegramZapIcon {
    color: Color,
    filled: bool,
}

impl canvas::Program<Message> for TelegramZapIcon {
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
        let scale = bounds.width.min(bounds.height) / 24.0;
        // Lucide "zap": 13 2 · 3 14 · 12 14 · 11 22 · 21 10 · 12 10 · 13 2
        let pts = [
            (13.0, 2.0),
            (3.0, 14.0),
            (12.0, 14.0),
            (11.0, 22.0),
            (21.0, 10.0),
            (12.0, 10.0),
            (13.0, 2.0),
        ];
        let path = canvas::Path::new(|p| {
            p.move_to(Point::new(pts[0].0 * scale, pts[0].1 * scale));
            for (x, y) in &pts[1..] {
                p.line_to(Point::new(x * scale, y * scale));
            }
            p.close();
        });
        if self.filled {
            frame.fill(&path, self.color);
        } else {
            frame.stroke(
                &path,
                canvas::Stroke::default()
                    .with_color(self.color)
                    .with_width(2.0 * scale),
            );
        }
        vec![frame.into_geometry()]
    }
}

// ----------------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------------

/// Left-only padding to indent card body content under the 32px avatar gutter
/// (iced `Padding` has no `[_; 4]` array conversion).
fn left_pad(left: f32) -> iced::Padding {
    iced::Padding {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left,
    }
}

fn theme_on_orange(colors: TelegramColors) -> Color {
    // Dark ink that reads on the flame-orange fill.
    blend_color(colors.primary, Color::BLACK, 0.82)
}

fn blend_color(base: Color, accent: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color {
        r: base.r + (accent.r - base.r) * amount,
        g: base.g + (accent.g - base.g) * amount,
        b: base.b + (accent.b - base.b) * amount,
        a: base.a + (accent.a - base.a) * amount,
    }
}

// ----------------------------------------------------------------------------
// Styles
// ----------------------------------------------------------------------------

fn telegram_primary_button(colors: TelegramColors, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let background = if hovered {
        blend_color(colors.primary, Color::WHITE, 0.08)
    } else {
        colors.primary
    };
    button::Style {
        background: Some(background.into()),
        text_color: theme_on_orange(colors),
        border: Border {
            radius: 5.0.into(),
            width: 1.0,
            color: colors.border_orange,
        },
        ..Default::default()
    }
}

fn telegram_quiet_button(colors: TelegramColors, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(
            if hovered {
                Color {
                    a: 0.06,
                    ..colors.text
                }
            } else {
                Color::TRANSPARENT
            }
            .into(),
        ),
        text_color: colors.muted,
        border: Border {
            radius: 5.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn telegram_accent_button(colors: TelegramColors, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(
            Color {
                a: if hovered { 0.16 } else { 0.09 },
                ..colors.primary
            }
            .into(),
        ),
        text_color: colors.orange_soft,
        border: Border {
            radius: 5.0.into(),
            width: 1.0,
            color: colors.border_orange,
        },
        ..Default::default()
    }
}

fn telegram_icon_button(colors: TelegramColors, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(
            Color {
                a: if hovered { 0.06 } else { 0.035 },
                ..colors.text
            }
            .into(),
        ),
        text_color: colors.muted,
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: colors.border,
        },
        ..Default::default()
    }
}

fn telegram_chip_toggle_button(
    colors: TelegramColors,
    status: button::Status,
    active: bool,
) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let background = match (active, hovered) {
        (true, true) => Color {
            a: 0.18,
            ..colors.primary
        },
        (true, false) => Color {
            a: 0.1,
            ..colors.primary
        },
        (false, true) => Color {
            a: 0.06,
            ..colors.text
        },
        (false, false) => Color {
            a: 0.035,
            ..colors.text
        },
    };
    button::Style {
        background: Some(background.into()),
        text_color: if active {
            colors.orange_soft
        } else {
            colors.muted
        },
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: if active {
                colors.border_orange
            } else {
                colors.border
            },
        },
        ..Default::default()
    }
}

fn telegram_link_button(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Color::TRANSPARENT.into()),
        border: Border {
            radius: 3.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn telegram_status_chip_button(border_color: Color, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(
            Color {
                a: if hovered { 0.08 } else { 0.03 },
                ..Color::WHITE
            }
            .into(),
        ),
        border: Border {
            radius: 3.0.into(),
            width: 1.0,
            color: border_color,
        },
        ..Default::default()
    }
}

fn telegram_sunken_button(colors: TelegramColors, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(
            if hovered {
                Color {
                    a: 0.04,
                    ..colors.text
                }
            } else {
                Color::TRANSPARENT
            }
            .into(),
        ),
        text_color: colors.text,
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn telegram_impact_chip_button(colors: TelegramColors, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(colors.sunken.into()),
        text_color: colors.text,
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: if hovered {
                colors.border_orange
            } else {
                colors.border
            },
        },
        ..Default::default()
    }
}

fn telegram_media_button(_theme: &Theme, status: button::Status) -> button::Style {
    let overlay = match status {
        button::Status::Hovered | button::Status::Pressed => Color {
            a: 0.06,
            ..Color::BLACK
        },
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(overlay.into()),
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn telegram_focus_input_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let base = helpers::text_input_style(theme, status);
    match status {
        text_input::Status::Focused { .. } => text_input::Style {
            border: Border {
                color: theme.palette().primary,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..base
        },
        _ => base,
    }
}

fn telegram_transparent_field_style(
    theme: &Theme,
    _status: text_input::Status,
) -> text_input::Style {
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        icon: theme.extended_palette().background.weak.text,
        placeholder: theme.extended_palette().background.weak.text,
        value: theme.palette().text,
        selection: theme.extended_palette().primary.weak.color,
    }
}

fn telegram_transparent_input_style(
    _theme: &Theme,
    _status: text_input::Status,
) -> text_input::Style {
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        icon: Color::TRANSPARENT,
        placeholder: Color::TRANSPARENT,
        value: Color::TRANSPARENT,
        selection: Color::TRANSPARENT,
    }
}

fn telegram_dot_style(color: Color) -> container_style::Style {
    container_style::Style {
        background: Some(color.into()),
        border: Border {
            radius: 999.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn telegram_pill_style(colors: TelegramColors) -> container_style::Style {
    container_style::Style {
        background: Some(
            Color {
                a: 0.025,
                ..colors.text
            }
            .into(),
        ),
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: colors.border,
        },
        ..Default::default()
    }
}

fn telegram_add_pill_style(colors: TelegramColors) -> container_style::Style {
    container_style::Style {
        background: Some(Color::TRANSPARENT.into()),
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: Color {
                a: 0.2,
                ..colors.muted
            },
        },
        ..Default::default()
    }
}

fn telegram_well_style(colors: TelegramColors) -> container_style::Style {
    container_style::Style {
        background: Some(colors.sunken.into()),
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: colors.border,
        },
        ..Default::default()
    }
}

fn telegram_code_cell_style(colors: TelegramColors, active: bool) -> container_style::Style {
    container_style::Style {
        background: Some(colors.sunken.into()),
        border: Border {
            radius: 5.0.into(),
            width: 1.0,
            color: if active {
                colors.border_orange
            } else {
                colors.border
            },
        },
        ..Default::default()
    }
}

fn telegram_icon_tile_style(colors: TelegramColors) -> container_style::Style {
    container_style::Style {
        background: Some(
            Color {
                a: 0.1,
                ..colors.primary
            }
            .into(),
        ),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: colors.border_orange,
        },
        ..Default::default()
    }
}

fn telegram_info_card_style(colors: TelegramColors, accent: bool) -> container_style::Style {
    container_style::Style {
        background: Some(
            if accent {
                Color {
                    a: 0.05,
                    ..colors.primary
                }
            } else {
                colors.panel
            }
            .into(),
        ),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: if accent {
                colors.border_orange
            } else {
                colors.border
            },
        },
        ..Default::default()
    }
}

fn telegram_sunken_outline_style(colors: TelegramColors) -> container_style::Style {
    container_style::Style {
        background: Some(colors.sunken.into()),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: colors.border,
        },
        ..Default::default()
    }
}

fn telegram_rule_style(colors: TelegramColors) -> rule::Style {
    rule::Style {
        color: colors.border,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

fn telegram_section_divider_style(colors: TelegramColors) -> container_style::Style {
    container_style::Style {
        background: Some(
            Color {
                a: 0.008,
                ..colors.text
            }
            .into(),
        ),
        border: Border {
            color: colors.border,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn telegram_media_placeholder_style(colors: TelegramColors) -> container_style::Style {
    container_style::Style {
        background: Some(colors.sunken.into()),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: Color {
                a: 0.18,
                ..colors.muted
            },
        },
        ..Default::default()
    }
}

fn telegram_avatar_placeholder_style(colors: TelegramColors) -> container_style::Style {
    container_style::Style {
        background: Some(
            Color {
                a: 0.1,
                ..colors.muted
            }
            .into(),
        ),
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: Color {
                a: 0.22,
                ..colors.muted
            },
        },
        ..Default::default()
    }
}

fn telegram_post_row_style(colors: TelegramColors, heat: f32) -> container_style::Style {
    let clamped = heat.clamp(0.0, 1.0);
    let background = blend_color(Color::TRANSPARENT, colors.primary, 0.06 * clamped);
    container_style::Style {
        background: Some(background.into()),
        border: Border {
            // Hairline divider between cards (bottom edge only is not expressible,
            // so a faint full border reads as a separator on the flat surface).
            color: colors.border,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests;
