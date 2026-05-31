use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::telegram_feed::{
    TelegramChannelProfile, TelegramFeedPost, telegram_age_countdown_label,
    telegram_arrival_latency_label, telegram_exact_time_label, telegram_new_message_heat,
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
        let mut content = column![
            self.view_telegram_feed_controls(available_width),
            self.view_telegram_feed_channels(),
        ]
        .spacing(8)
        .width(Fill);

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
        let refresh_button = self.view_telegram_refresh_button();

        if available_width < TELEGRAM_COMPACT_CONTROLS_WIDTH {
            column![
                input,
                row![
                    add_button,
                    notification_button,
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
            row![input, add_button, notification_button, refresh_button]
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

    fn view_telegram_feed_channels(&self) -> Element<'_, Message> {
        let theme = self.theme();
        if self.telegram_feed.channels.is_empty() {
            return text("No channels")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .into();
        }

        self.telegram_feed
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
            .vertical_spacing(6)
            .into()
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
                rows.push(telegram_post_card(
                    post,
                    profile,
                    now_ms,
                    theme.palette().primary,
                    theme.palette().text,
                    theme.extended_palette().background.weak.text,
                ))
            });

        scrollable(rows)
            .direction(Direction::Vertical(
                Scrollbar::new().width(4).margin(0).scroller_width(4),
            ))
            .height(Fill)
            .into()
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

fn telegram_post_card(
    post: TelegramFeedPost,
    profile: Option<TelegramChannelProfile>,
    now_ms: u64,
    primary_text: Color,
    body_text: Color,
    muted_text: Color,
) -> Element<'static, Message> {
    let channel = format!("@{}", post.channel);
    let title = profile
        .as_ref()
        .map(|profile| profile.title.clone())
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| channel.clone());
    let age = telegram_age_countdown_label(post.timestamp_ms, now_ms);
    let exact_time = telegram_exact_time_label(post.timestamp_ms);
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

    let mut time_line = row![text(exact_time).size(10).color(muted_text)]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Fill);
    if let Some(latency) = latency {
        time_line = time_line.push(text(latency).size(10).color(muted_text));
    }
    time_line = time_line.push(text(age).size(11).color(muted_text));
    let time_line = time_line.wrap().vertical_spacing(4);

    container(
        column![
            top_line,
            time_line,
            text(post.text).size(12).color(body_text).width(Fill),
        ]
        .spacing(6)
        .width(Fill),
    )
    .width(Fill)
    .padding([8, 10])
    .style(move |theme: &Theme| telegram_post_container(theme, heat))
    .into()
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
        return image(handle.clone())
            .width(size)
            .height(size)
            .content_fit(ContentFit::Cover)
            .border_radius(size / 2.0)
            .into();
    }

    let initials = profile
        .map(|profile| profile.initials.clone())
        .filter(|initials| !initials.trim().is_empty())
        .unwrap_or_else(|| channel.chars().take(2).collect::<String>().to_uppercase());
    container(text(initials).size(size * 0.42).color(text_color).center())
        .width(size)
        .height(size)
        .center(Fill)
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
