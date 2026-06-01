use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::telegram_feed::{
    TelegramChannelProfile, TelegramFastAuthStage, TelegramFeedPost, telegram_age_countdown_label,
    telegram_arrival_latency_label, telegram_new_message_heat, telegram_price_impact_pct,
};
use iced::ContentFit;
use iced::widget::container as container_style;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Space, button, column, container, image, responsive, row, rule, scrollable, text, text_input,
    tooltip,
};
use iced::{Alignment, Color, Element, Fill, Theme};

const TELEGRAM_COMPACT_CONTROLS_WIDTH: f32 = 360.0;
const TELEGRAM_CHANNEL_COLLAPSE_THRESHOLD: usize = 4;
const TELEGRAM_FAST_STATUS_BUTTON_SIZE: f32 = 24.0;
const TELEGRAM_FAST_STATUS_ICON_SIZE: f32 = 14.0;
const TELEGRAM_FAST_STATUS_DOT_SIZE: f32 = 7.0;

#[derive(Debug, Clone)]
struct TelegramTickerImpactCard {
    symbol: String,
    ticker: String,
    impact_pct: Option<f64>,
}

impl TradingTerminal {
    pub(crate) fn view_telegram_feed(&self) -> Element<'_, Message> {
        let now_ms = Self::now_ms();

        container(responsive(move |size| {
            self.view_telegram_feed_sized(now_ms, size.width)
        }))
        .width(Fill)
        .height(Fill)
        .padding(10)
        .into()
    }

    fn view_telegram_feed_sized(&self, now_ms: u64, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
        let mut content = column![self.view_telegram_feed_controls(available_width)]
            .spacing(8)
            .width(Fill);
        if self.telegram_fast_panel_visible() {
            content = content.push(self.view_telegram_fast_panel(available_width));
        }
        content = content.push(self.view_telegram_feed_channels());

        if let Some(error) = &self.telegram_feed.last_error {
            content = content.push(
                text(error.clone())
                    .size(11)
                    .color(theme.palette().danger)
                    .width(Fill),
            );
        }

        content = content
            .push(rule::horizontal(1))
            .push(self.view_telegram_feed_body(now_ms));

        container(content).width(Fill).height(Fill).into()
    }

    fn view_telegram_feed_controls(&self, available_width: f32) -> Element<'_, Message> {
        let input = text_input("@public_channel", &self.telegram_feed.channel_input)
            .style(helpers::text_input_style)
            .on_input(Message::TelegramFeedChannelInputChanged)
            .on_submit(Message::TelegramFeedAddChannel)
            .size(12)
            .padding([5, 8])
            .width(Fill);

        let add_button = button(text("Add").size(11).center())
            .on_press(Message::TelegramFeedAddChannel)
            .padding([5, 10])
            .style(telegram_action_button);

        let notification_button = self.view_telegram_notification_button();
        let fast_button = self.view_telegram_fast_button();
        let refresh_button = self.view_telegram_refresh_button();

        if available_width < TELEGRAM_COMPACT_CONTROLS_WIDTH {
            column![
                input,
                row![
                    add_button,
                    notification_button,
                    fast_button,
                    Space::new().width(Fill),
                    refresh_button
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(6)
            .width(Fill)
            .into()
        } else {
            row![
                input,
                add_button,
                notification_button,
                fast_button,
                refresh_button
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .width(Fill)
            .into()
        }
    }

    fn view_telegram_notification_button(&self) -> Element<'static, Message> {
        let enabled = self.telegram_feed.notifications_enabled;
        let label = if enabled { "Alerts: ON" } else { "Alerts: OFF" };

        button(text(label).size(11).center())
            .on_press(Message::ToggleTelegramFeedNotifications)
            .padding([5, 10])
            .style(move |theme: &Theme, status| telegram_toggle_button(theme, status, enabled))
            .into()
    }

    fn view_telegram_fast_button(&self) -> Element<'_, Message> {
        let enabled = self.telegram_feed.fast_mode_enabled;
        if self.telegram_fast_signed_in() {
            let connected = self.telegram_feed.fast_connected;
            let status = self
                .telegram_feed
                .fast_status
                .as_ref()
                .map(|(message, _)| message.clone())
                .unwrap_or_else(|| "Fast mode signed in".to_string());
            let icon = container(
                Space::new()
                    .width(TELEGRAM_FAST_STATUS_DOT_SIZE)
                    .height(TELEGRAM_FAST_STATUS_DOT_SIZE),
            )
            .center(TELEGRAM_FAST_STATUS_ICON_SIZE)
            .style(move |theme: &Theme| telegram_fast_status_icon(theme, connected));
            let mut fast_button = button(icon)
                .width(TELEGRAM_FAST_STATUS_BUTTON_SIZE)
                .height(TELEGRAM_FAST_STATUS_BUTTON_SIZE)
                .padding(0)
                .style(move |theme: &Theme, status| telegram_toggle_button(theme, status, true));
            if !self.telegram_feed.fast_auth_in_flight {
                fast_button = fast_button.on_press(Message::TelegramFastSignOut);
            }

            return tooltip(fast_button, text(status).size(10), tooltip::Position::Top).into();
        }

        let label = if enabled { "Fast: ON" } else { "Fast: OFF" };

        button(text(label).size(11).center())
            .on_press(Message::ToggleTelegramFastFeed)
            .padding([5, 10])
            .style(move |theme: &Theme, status| telegram_toggle_button(theme, status, enabled))
            .into()
    }

    fn view_telegram_refresh_button(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = if self.telegram_feed.loading() {
            self.view_spinner(13)
        } else {
            text("\u{21bb}")
                .size(13)
                .center()
                .font(crate::app_fonts::monospace_font())
                .into()
        };

        let mut refresh_button = button(content)
            .padding([4, 8])
            .style(subtle_telegram_icon_button);

        if !self.telegram_feed.refreshing() {
            refresh_button = refresh_button.on_press(Message::RefreshTelegramFeed);
        }

        tooltip(
            refresh_button,
            text("Refresh").size(10),
            tooltip::Position::Top,
        )
        .into()
    }

    fn view_telegram_fast_panel(&self, available_width: f32) -> Element<'_, Message> {
        if !self.telegram_fast_panel_visible() {
            return Space::new().height(0).into();
        }

        let theme = self.theme();
        let status = self
            .telegram_feed
            .fast_status
            .as_ref()
            .map(|(message, is_error)| {
                text(message.clone()).size(10).color(if *is_error {
                    theme.palette().danger
                } else {
                    theme.extended_palette().background.weak.text
                })
            })
            .unwrap_or_else(|| {
                text("Fast mode waiting for Telegram session")
                    .size(10)
                    .color(theme.extended_palette().background.weak.text)
            });

        let api_id = text_input("API ID", &self.telegram_feed.fast_api_id_input)
            .style(helpers::text_input_style)
            .on_input(Message::TelegramFastApiIdChanged)
            .size(11)
            .padding([4, 7])
            .width(Fill);
        let api_hash = text_input("API hash", &self.telegram_feed.fast_api_hash_input)
            .style(helpers::text_input_style)
            .on_input(Message::TelegramFastApiHashChanged)
            .secure(true)
            .size(11)
            .padding([4, 7])
            .width(Fill);
        let phone = text_input("+ phone", &self.telegram_feed.fast_phone_input)
            .style(helpers::text_input_style)
            .on_input(Message::TelegramFastPhoneChanged)
            .on_submit(Message::TelegramFastRequestCode)
            .size(11)
            .padding([4, 7])
            .width(Fill);

        let mut request_button = button(text("Send code").size(10).center())
            .padding([4, 8])
            .style(telegram_action_button);
        if !self.telegram_feed.fast_auth_in_flight {
            request_button = request_button.on_press(Message::TelegramFastRequestCode);
        }

        let credentials_row: Element<'_, Message> =
            if available_width < TELEGRAM_COMPACT_CONTROLS_WIDTH {
                column![
                    row![api_id, api_hash].spacing(6).width(Fill),
                    row![phone, request_button]
                        .spacing(6)
                        .align_y(Alignment::Center),
                ]
                .spacing(6)
                .width(Fill)
                .into()
            } else {
                row![api_id, api_hash, phone, request_button]
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .width(Fill)
                    .into()
            };

        let mut panel = column![status, credentials_row].spacing(6).width(Fill);

        if matches!(
            self.telegram_feed.fast_auth_stage,
            TelegramFastAuthStage::CodeRequested
        ) {
            let code = text_input("Code", &self.telegram_feed.fast_code_input)
                .style(helpers::text_input_style)
                .on_input(Message::TelegramFastCodeChanged)
                .on_submit(Message::TelegramFastSubmitCode)
                .secure(true)
                .size(11)
                .padding([4, 7])
                .width(Fill);
            let mut submit = button(text("Sign in").size(10).center())
                .padding([4, 8])
                .style(telegram_action_button);
            if !self.telegram_feed.fast_auth_in_flight {
                submit = submit.on_press(Message::TelegramFastSubmitCode);
            }
            panel = panel.push(row![code, submit].spacing(6).align_y(Alignment::Center));
        }

        if matches!(
            self.telegram_feed.fast_auth_stage,
            TelegramFastAuthStage::PasswordRequired
        ) {
            let placeholder = self
                .telegram_feed
                .fast_password_hint
                .as_ref()
                .map(|hint| format!("2FA password ({hint})"))
                .unwrap_or_else(|| "2FA password".to_string());
            let password = text_input(
                placeholder.as_str(),
                &self.telegram_feed.fast_password_input,
            )
            .style(helpers::text_input_style)
            .on_input(Message::TelegramFastPasswordChanged)
            .on_submit(Message::TelegramFastSubmitPassword)
            .secure(true)
            .size(11)
            .padding([4, 7])
            .width(Fill);
            let mut submit = button(text("Unlock").size(10).center())
                .padding([4, 8])
                .style(telegram_action_button);
            if !self.telegram_feed.fast_auth_in_flight {
                submit = submit.on_press(Message::TelegramFastSubmitPassword);
            }
            panel = panel.push(row![password, submit].spacing(6).align_y(Alignment::Center));
        }

        container(panel)
            .width(Fill)
            .padding([7, 8])
            .style(telegram_fast_panel_container)
            .into()
    }

    fn telegram_fast_signed_in(&self) -> bool {
        self.telegram_feed.fast_connected
            || matches!(
                self.telegram_feed.fast_auth_stage,
                TelegramFastAuthStage::SignedIn
            )
    }

    fn telegram_fast_panel_visible(&self) -> bool {
        self.telegram_feed.fast_mode_enabled && !self.telegram_fast_signed_in()
    }

    fn view_telegram_feed_channels(&self) -> Element<'_, Message> {
        let theme = self.theme();
        if self.telegram_feed.channels.is_empty() {
            return text("No channels")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .into();
        }

        let collapsible = self.telegram_feed.channels.len() > TELEGRAM_CHANNEL_COLLAPSE_THRESHOLD;
        if collapsible && !self.telegram_feed.channels_expanded {
            return telegram_channel_collapse_summary(
                self.telegram_feed.channels.len(),
                self.telegram_feed.refreshing(),
                false,
                theme.palette().primary,
                theme.extended_palette().background.weak.text,
            );
        }

        let chips = self
            .telegram_feed
            .channels
            .iter()
            .fold(
                row![].spacing(6).align_y(Alignment::Center),
                |channels, channel| {
                    let label_color = if self
                        .telegram_feed
                        .loading_channels
                        .iter()
                        .any(|loading| loading == channel)
                    {
                        theme.palette().primary
                    } else {
                        theme.extended_palette().background.weak.text
                    };

                    channels.push(telegram_channel_chip(
                        channel.clone(),
                        label_color,
                        self.telegram_feed.channel_profiles.get(channel).cloned(),
                    ))
                },
            )
            .wrap()
            .vertical_spacing(6);

        if collapsible {
            column![
                telegram_channel_collapse_summary(
                    self.telegram_feed.channels.len(),
                    self.telegram_feed.refreshing(),
                    true,
                    theme.palette().primary,
                    theme.extended_palette().background.weak.text,
                ),
                chips,
            ]
            .spacing(6)
            .width(Fill)
            .into()
        } else {
            chips.into()
        }
    }

    fn view_telegram_feed_body(&self, now_ms: u64) -> Element<'_, Message> {
        let theme = self.theme();
        let posts = self.telegram_feed.visible_posts();

        if posts.is_empty() {
            let label = if self.telegram_feed.loading() {
                "Loading posts..."
            } else if self.telegram_feed.channels.is_empty() {
                "Add a public Telegram channel"
            } else {
                "No public posts found"
            };
            return container(text(label).size(12).color(theme.palette().text))
                .width(Fill)
                .height(Fill)
                .center_x(Fill)
                .center_y(Fill)
                .into();
        }

        let rows = posts
            .into_iter()
            .fold(column![].spacing(8).width(Fill), |rows, post| {
                let profile = self
                    .telegram_feed
                    .channel_profiles
                    .get(&post.channel)
                    .cloned();
                let ticker_impacts = self.telegram_ticker_impact_cards(&post);
                rows.push(telegram_post_card(
                    post,
                    profile,
                    ticker_impacts,
                    now_ms,
                    theme.palette().primary,
                    theme.palette().text,
                    theme.extended_palette().background.weak.text,
                    theme.palette().success,
                    theme.palette().danger,
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
        post.ticker_mentions
            .iter()
            .filter(|mention| {
                self.resolve_exchange_symbol_by_key_or_ticker(&mention.symbol)
                    .is_some_and(|symbol| {
                        symbol.market_type != MarketType::Spot
                            && self.exchange_symbol_is_orderable(symbol)
                    })
            })
            .map(|mention| TelegramTickerImpactCard {
                symbol: mention.symbol.clone(),
                ticker: mention.ticker.clone(),
                impact_pct: telegram_price_impact_pct(
                    mention.reference_price,
                    self.resolve_mid_for_symbol(&mention.symbol),
                ),
            })
            .collect()
    }
}

fn telegram_channel_chip(
    channel: String,
    label_color: Color,
    profile: Option<TelegramChannelProfile>,
) -> Element<'static, Message> {
    let label = format!("@{channel}");
    let remove_channel = channel.clone();
    let avatar = telegram_channel_avatar(profile.as_ref(), &channel, 18.0, label_color);

    container(
        row![
            avatar,
            text(label).size(11).color(label_color),
            button(text("x").size(10).center())
                .on_press(Message::TelegramFeedRemoveChannel(remove_channel))
                .padding([0, 4])
                .style(subtle_telegram_icon_button),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding([3, 6])
    .style(telegram_chip_container)
    .into()
}

fn telegram_channel_collapse_summary(
    channel_count: usize,
    refreshing: bool,
    expanded: bool,
    active_text: Color,
    muted_text: Color,
) -> Element<'static, Message> {
    let label = if refreshing {
        format!("{channel_count} channels - refreshing")
    } else {
        format!("{channel_count} channels")
    };
    let toggle_label = if expanded { "Hide" } else { "Show" };

    container(
        row![
            text(label)
                .size(11)
                .color(if refreshing { active_text } else { muted_text }),
            Space::new().width(Fill),
            button(text(toggle_label).size(10).center())
                .on_press(Message::ToggleTelegramFeedChannelsExpanded)
                .padding([1, 7])
                .style(subtle_telegram_icon_button),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([3, 6])
    .style(telegram_chip_container)
    .into()
}

fn telegram_post_card(
    post: TelegramFeedPost,
    profile: Option<TelegramChannelProfile>,
    ticker_impacts: Vec<TelegramTickerImpactCard>,
    now_ms: u64,
    primary_text: Color,
    body_text: Color,
    muted_text: Color,
    success_text: Color,
    danger_text: Color,
) -> Element<'static, Message> {
    let channel = format!("@{}", post.channel);
    let title = profile
        .as_ref()
        .map(|profile| profile.title.clone())
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| channel.clone());
    let age = telegram_age_countdown_label(post.timestamp_ms, now_ms);
    let latency = telegram_arrival_latency_label(&post);
    let url = post.url.clone();
    let heat = telegram_new_message_heat(post.first_seen_ms, now_ms);
    let identity = telegram_channel_identity(
        &post.channel,
        &channel,
        &title,
        profile.as_ref(),
        primary_text,
        muted_text,
    );

    let top_line = row![
        identity,
        Space::new().width(Fill),
        tooltip(
            button(text("Link").size(10).center())
                .on_press(Message::CopyToClipboard(url))
                .padding([1, 6])
                .style(subtle_telegram_icon_button),
            text("Copy link").size(10),
            tooltip::Position::Top,
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Fill);

    let mut time_line = row![text(age).size(11).color(muted_text)]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Fill);
    if let Some(latency) = latency {
        time_line = time_line.push(text(latency).size(10).color(muted_text));
    }
    let time_line = time_line.wrap().vertical_spacing(4);

    let mut content = column![
        top_line,
        time_line,
        text(post.text).size(12).color(body_text).width(Fill),
    ]
    .spacing(6)
    .width(Fill);
    if !ticker_impacts.is_empty() {
        content = content.push(telegram_ticker_impact_cards(
            ticker_impacts,
            primary_text,
            muted_text,
            success_text,
            danger_text,
        ));
    }

    container(content)
        .width(Fill)
        .padding([8, 10])
        .style(move |theme: &Theme| telegram_post_container(theme, heat))
        .into()
}

fn telegram_ticker_impact_cards(
    impacts: Vec<TelegramTickerImpactCard>,
    primary_text: Color,
    muted_text: Color,
    success_text: Color,
    danger_text: Color,
) -> Element<'static, Message> {
    impacts
        .into_iter()
        .fold(row![].spacing(6).width(Fill), |row, impact| {
            row.push(telegram_ticker_impact_card(
                impact,
                primary_text,
                muted_text,
                success_text,
                danger_text,
            ))
        })
        .wrap()
        .vertical_spacing(6)
        .into()
}

fn telegram_ticker_impact_card(
    impact: TelegramTickerImpactCard,
    primary_text: Color,
    muted_text: Color,
    success_text: Color,
    danger_text: Color,
) -> Element<'static, Message> {
    let symbol = impact.symbol.clone();
    let impact_label = telegram_impact_label(impact.impact_pct);
    let impact_color =
        telegram_impact_color(impact.impact_pct, muted_text, success_text, danger_text);
    let icon: Element<'static, Message> = helpers::symbol_icon(&impact.symbol, 14, primary_text)
        .map(Element::from)
        .unwrap_or_else(|| telegram_ticker_fallback_icon(&impact.ticker, primary_text));
    let pct = impact.impact_pct;

    button(
        row![
            icon,
            text(impact.ticker)
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(primary_text),
            text(impact_label)
                .size(11)
                .font(crate::app_fonts::monospace_font())
                .color(impact_color),
        ]
        .spacing(5)
        .align_y(Alignment::Center),
    )
    .on_press(Message::SymbolSelected(symbol))
    .padding([3, 7])
    .style(move |theme: &Theme, status| telegram_ticker_impact_button(theme, status, pct))
    .into()
}

fn telegram_ticker_fallback_icon(ticker: &str, color: Color) -> Element<'static, Message> {
    let label = ticker
        .chars()
        .find(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string());

    container(
        text(label)
            .size(8)
            .font(crate::app_fonts::monospace_font())
            .color(color)
            .center(),
    )
    .center(14.0)
    .style(move |_theme: &Theme| telegram_ticker_icon_container(color))
    .into()
}

fn telegram_impact_label(impact_pct: Option<f64>) -> String {
    impact_pct
        .map(|impact| format!("{impact:+.2}%"))
        .unwrap_or_else(|| "-".to_string())
}

fn telegram_impact_color(
    impact_pct: Option<f64>,
    fallback: Color,
    success_text: Color,
    danger_text: Color,
) -> Color {
    match impact_pct {
        Some(impact) if impact >= 0.0 => success_text,
        Some(_) => danger_text,
        None => fallback,
    }
}

fn telegram_channel_identity(
    channel: &str,
    username: &str,
    title: &str,
    profile: Option<&TelegramChannelProfile>,
    primary_text: Color,
    muted_text: Color,
) -> Element<'static, Message> {
    let avatar = telegram_channel_avatar(profile, channel, 22.0, primary_text);
    let mut labels = column![text(title.to_string()).size(11).color(primary_text)].spacing(1);
    if title != username {
        labels = labels.push(text(username.to_string()).size(10).color(muted_text));
    }

    row![avatar, labels]
        .spacing(7)
        .align_y(Alignment::Center)
        .into()
}

fn telegram_channel_avatar(
    profile: Option<&TelegramChannelProfile>,
    channel: &str,
    size: f32,
    text_color: Color,
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
    container(text(initials).size(size * 0.42).color(text_color).center())
        .center(size)
        .style(move |theme: &Theme| telegram_avatar_placeholder_style(theme, text_color))
        .into()
}

fn telegram_action_button(theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => Color {
            a: 0.14,
            ..theme.palette().primary
        },
        _ => Color {
            a: 0.08,
            ..theme.palette().primary
        },
    };

    button::Style {
        background: Some(background.into()),
        text_color: theme.palette().primary,
        border: iced::Border {
            radius: 3.0.into(),
            width: 1.0,
            color: Color {
                a: 0.32,
                ..theme.palette().primary
            },
        },
        ..Default::default()
    }
}

fn telegram_toggle_button(theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let background = match (active, status) {
        (true, button::Status::Hovered | button::Status::Pressed) => Color {
            a: 0.18,
            ..theme.palette().primary
        },
        (true, _) => Color {
            a: 0.10,
            ..theme.palette().primary
        },
        (false, button::Status::Hovered | button::Status::Pressed) => Color {
            a: 0.08,
            ..theme.palette().text
        },
        (false, _) => Color::TRANSPARENT,
    };

    button::Style {
        background: Some(background.into()),
        text_color: if active {
            theme.palette().primary
        } else {
            theme.extended_palette().background.weak.text
        },
        border: iced::Border {
            radius: 3.0.into(),
            width: if active { 1.0 } else { 0.0 },
            color: if active {
                Color {
                    a: 0.32,
                    ..theme.palette().primary
                }
            } else {
                Color::TRANSPARENT
            },
        },
        ..Default::default()
    }
}

fn subtle_telegram_icon_button(theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => Color {
            a: 0.08,
            ..theme.palette().text
        },
        _ => Color::TRANSPARENT,
    };

    button::Style {
        background: Some(background.into()),
        text_color: theme.extended_palette().background.weak.text,
        border: iced::Border {
            radius: 3.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn telegram_ticker_impact_button(
    theme: &Theme,
    status: button::Status,
    impact_pct: Option<f64>,
) -> button::Style {
    let accent = match impact_pct {
        Some(impact) if impact >= 0.0 => theme.palette().success,
        Some(_) => theme.palette().danger,
        None => theme.palette().primary,
    };
    let alpha = match status {
        button::Status::Hovered | button::Status::Pressed => 0.16,
        _ => 0.08,
    };

    button::Style {
        background: Some(Color { a: alpha, ..accent }.into()),
        text_color: theme.palette().text,
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: Color { a: 0.28, ..accent },
        },
        ..Default::default()
    }
}

fn telegram_chip_container(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: iced::Border {
            radius: 4.0.into(),
            width: 1.0,
            color: Color {
                a: 0.25,
                ..theme.extended_palette().background.strong.color
            },
        },
        ..Default::default()
    }
}

fn telegram_fast_panel_container(theme: &Theme) -> container_style::Style {
    container_style::Style {
        background: Some(
            Color {
                a: 0.08,
                ..theme.palette().primary
            }
            .into(),
        ),
        border: iced::Border {
            radius: 5.0.into(),
            width: 1.0,
            color: Color {
                a: 0.20,
                ..theme.palette().primary
            },
        },
        ..Default::default()
    }
}

fn telegram_fast_status_icon(theme: &Theme, connected: bool) -> container_style::Style {
    let color = if connected {
        theme.palette().success
    } else {
        theme.palette().danger
    };

    container_style::Style {
        background: Some(color.into()),
        border: iced::Border {
            radius: 7.0.into(),
            width: 1.0,
            color: Color { a: 0.45, ..color },
        },
        ..Default::default()
    }
}

fn telegram_ticker_icon_container(color: Color) -> container_style::Style {
    container_style::Style {
        background: Some(Color { a: 0.10, ..color }.into()),
        border: iced::Border {
            radius: 7.0.into(),
            width: 1.0,
            color: Color { a: 0.24, ..color },
        },
        ..Default::default()
    }
}

fn telegram_avatar_placeholder_style(theme: &Theme, text_color: Color) -> container_style::Style {
    container_style::Style {
        background: Some(
            Color {
                a: 0.10,
                ..text_color
            }
            .into(),
        ),
        border: iced::Border {
            radius: 10.0.into(),
            width: 1.0,
            color: Color {
                a: 0.22,
                ..theme.extended_palette().background.strong.color
            },
        },
        ..Default::default()
    }
}

fn telegram_post_container(theme: &Theme, heat: f32) -> container_style::Style {
    let base = theme.extended_palette().background.weak.color;
    let accent = theme.palette().primary;
    let clamped_heat = heat.clamp(0.0, 1.0);
    let background = blend_color(base, accent, 0.20 * clamped_heat);

    container_style::Style {
        background: Some(background.into()),
        border: iced::Border {
            radius: 5.0.into(),
            width: if clamped_heat > 0.0 { 1.0 } else { 0.0 },
            color: if clamped_heat > 0.0 {
                Color {
                    a: 0.40 * clamped_heat,
                    ..accent
                }
            } else {
                Color::TRANSPARENT
            },
        },
        ..Default::default()
    }
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
