use crate::app_state::TradingTerminal;
use crate::helpers::{format_relative_time, format_seen_latency_label, format_timestamp_exact};
use crate::message::Message;
use crate::x_feed::{XFeedId, XFeedInstance, XFeedSourceOption};
use iced::widget::{
    Space, button, column, container, pick_list, row, rule, scrollable, text, text_input,
};
use iced::{Alignment, Element, Fill, Length, Theme};

impl TradingTerminal {
    pub(crate) fn view_x_feed(&self, id: XFeedId) -> Element<'_, Message> {
        let Some(instance) = self.x_feed.instances.get(&id) else {
            return container(text("X Feed instance is missing").size(12))
                .width(Fill)
                .height(Fill)
                .center_x(Fill)
                .center_y(Fill)
                .into();
        };

        let content = column![
            self.view_x_feed_controls(id, instance),
            self.view_x_feed_status(instance),
            rule::horizontal(1),
            self.view_x_feed_posts(instance),
        ]
        .spacing(8)
        .width(Fill)
        .height(Fill);

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(10)
            .style(|theme: &Theme| iced::widget::container::Style {
                background: Some(theme.extended_palette().background.base.color.into()),
                ..Default::default()
            })
            .into()
    }

    pub(crate) fn view_x_feed_status_chip(&self, id: XFeedId) -> Element<'_, Message> {
        let theme = self.theme();
        let palette = theme.palette();
        let (label, color) = if self.x_feed.auth_user.is_some() {
            ("X · AUTH", palette.success)
        } else if self.x_feed.has_access_token() {
            ("X · TOKEN", palette.primary)
        } else {
            ("X · BYOK", theme.extended_palette().background.weak.text)
        };

        button(text(label).size(10).color(color))
            .on_press(Message::RefreshXFeed(id))
            .padding([2, 6])
            .style(move |_theme: &Theme, _status| iced::widget::button::Style {
                text_color: color,
                background: Some(iced::Background::Color(iced::Color { a: 0.08, ..color })),
                border: iced::Border {
                    color: iced::Color { a: 0.3, ..color },
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn view_x_feed_controls(&self, id: XFeedId, instance: &XFeedInstance) -> Element<'_, Message> {
        let theme = self.theme();
        let source_options = self.x_feed.source_options();
        let selected_source = Some(XFeedSourceOption::new(instance.source.clone()));

        let source_picker = pick_list(source_options, selected_source, move |option| {
            Message::XFeedSourceSelected(id, option)
        })
        .padding([4, 8])
        .text_size(11)
        .width(Length::Fixed(190.0));

        let refresh = button(text("Refresh").size(11))
            .on_press(Message::RefreshXFeed(id))
            .padding([5, 10]);

        let lists = button(text("Lists").size(11))
            .on_press(Message::XFeedListsRefresh)
            .padding([5, 10]);

        let mut top = row![
            text(instance.source.label())
                .size(13)
                .color(theme.palette().text),
            source_picker,
            refresh,
            lists,
            Space::new().width(Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        if let Some(user) = &self.x_feed.auth_user {
            top = top.push(
                text(format!("@{}", user.username))
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
            );
        }

        let token_input = text_input(
            "X OAuth 2.0 user access token",
            &self.x_feed.access_token_input,
        )
        .on_input(|value| Message::XFeedAccessTokenChanged(value.into()))
        .on_submit(Message::XFeedConnect)
        .secure(true)
        .padding([6, 8])
        .size(11)
        .width(Fill);
        let connect = button(
            text(if self.x_feed.connecting {
                "Connecting"
            } else {
                "Connect"
            })
            .size(11),
        )
        .on_press(Message::XFeedConnect)
        .padding([6, 10]);
        let clear = button(text("Clear").size(11))
            .on_press(Message::XFeedClearAccessToken)
            .padding([6, 10]);

        column![
            top,
            row![token_input, connect, clear]
                .spacing(8)
                .align_y(Alignment::Center),
        ]
        .spacing(8)
        .into()
    }

    fn view_x_feed_status<'a>(&'a self, instance: &'a XFeedInstance) -> Element<'a, Message> {
        let theme = self.theme();
        let status = instance
            .last_error
            .as_ref()
            .map(|message| (message.as_str(), true))
            .or_else(|| {
                self.x_feed
                    .status
                    .as_ref()
                    .map(|(message, is_error)| (message.as_str(), *is_error))
            });

        let Some((message, is_error)) = status else {
            let label = instance
                .last_refresh_ms
                .map(format_timestamp_exact)
                .map(|time| format!("Last refresh {time}"))
                .unwrap_or_else(|| "Connect X to start polling Following and Lists".to_string());
            return text(label)
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .into();
        };

        text(message)
            .size(11)
            .color(if is_error {
                theme.palette().danger
            } else {
                theme.extended_palette().background.weak.text
            })
            .into()
    }

    fn view_x_feed_posts<'a>(&'a self, instance: &'a XFeedInstance) -> Element<'a, Message> {
        let theme = self.theme();
        if instance.posts.is_empty() {
            return container(text("No posts loaded").size(12).color(theme.palette().text))
                .width(Fill)
                .height(Fill)
                .center_x(Fill)
                .center_y(Fill)
                .into();
        }

        let now_ms = self.status_bar_now_ms;
        let mut posts = column![].spacing(8).width(Fill);
        for post in &instance.posts {
            posts = posts.push(x_post_card(post, now_ms, &theme));
        }

        scrollable(posts).height(Fill).into()
    }
}

fn x_post_card<'a>(
    post: &'a crate::x_feed::XFeedPost,
    now_ms: u64,
    theme: &Theme,
) -> Element<'a, Message> {
    let muted = theme.extended_palette().background.weak.text;
    let latency =
        format_seen_latency_label(post.created_at_ms, post.received_at_ms, post.received_at_ms)
            .unwrap_or_else(|| format_relative_time(post.created_at_ms, now_ms));
    let header = row![
        text(format!("@{}", post.author_username))
            .size(12)
            .color(theme.palette().text),
        text(post.author_name.as_str()).size(11).color(muted),
        Space::new().width(Fill),
        text(latency).size(10).color(muted),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let copy = button(text("Copy").size(10))
        .on_press(Message::CopyToClipboard(post.url.clone().into()))
        .padding([3, 8]);

    container(
        column![
            header,
            text(post.text.as_str()).size(12).width(Fill),
            row![
                text(format_timestamp_exact(post.created_at_ms))
                    .size(10)
                    .color(muted),
                Space::new().width(Fill),
                copy,
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(6)
        .width(Fill),
    )
    .width(Fill)
    .padding(10)
    .style(|theme: &Theme| iced::widget::container::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: iced::Border {
            color: iced::Color {
                a: 0.12,
                ..theme.palette().text
            },
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    })
    .into()
}
