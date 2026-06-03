use crate::api::MarketType;
use crate::app_state::TradingTerminal;
use crate::helpers;
use crate::message::Message;
use crate::x_feed::{
    XFeedAuthorProfile, XFeedPost, x_age_countdown_label, x_arrival_latency_label, x_new_post_heat,
    x_price_impact_pct,
};
use iced::widget::container as container_style;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Space, button, column, container, responsive, row, rule, scrollable, text, text_input, tooltip,
};
use iced::{Alignment, Color, Element, Fill, Theme};

const X_COMPACT_CONTROLS_WIDTH: f32 = 420.0;
const X_SOURCE_COLLAPSE_THRESHOLD: usize = 5;

#[derive(Debug, Clone)]
struct XTickerImpactCard {
    ticker: String,
    impact_pct: Option<f64>,
}

impl TradingTerminal {
    pub(crate) fn view_x_feed(&self) -> Element<'_, Message> {
        let now_ms = Self::now_ms();

        container(responsive(move |size| {
            self.view_x_feed_sized(now_ms, size.width)
        }))
        .width(Fill)
        .height(Fill)
        .padding(10)
        .into()
    }

    fn view_x_feed_sized(&self, now_ms: u64, available_width: f32) -> Element<'_, Message> {
        let theme = self.theme();
        let mut content = column![
            self.view_x_feed_token_controls(available_width),
            self.view_x_feed_source_controls(available_width),
            self.view_x_feed_sources(),
        ]
        .spacing(8)
        .width(Fill);

        if let Some((status, is_error)) = &self.x_feed.stream_status {
            content = content.push(text(status.clone()).size(10).color(if *is_error {
                theme.palette().danger
            } else {
                theme.extended_palette().background.weak.text
            }));
        }

        if let Some(error) = &self.x_feed.last_error {
            content = content.push(
                text(error.clone())
                    .size(11)
                    .color(theme.palette().danger)
                    .width(Fill),
            );
        }

        content = content
            .push(rule::horizontal(1))
            .push(self.view_x_feed_body(now_ms));

        container(content).width(Fill).height(Fill).into()
    }

    fn view_x_feed_token_controls(&self, available_width: f32) -> Element<'_, Message> {
        let token = text_input("X bearer token", &self.x_feed.bearer_token_input)
            .style(helpers::text_input_style)
            .on_input(Message::XFeedBearerTokenChanged)
            .on_submit(Message::SaveXFeedBearerToken)
            .secure(true)
            .size(12)
            .padding([5, 8])
            .width(Fill);

        let save = button(text("Save token").size(11).center())
            .on_press(Message::SaveXFeedBearerToken)
            .padding([5, 10])
            .style(x_action_button);

        let status = if self.x_feed.bearer_token.trim().is_empty() {
            "No token"
        } else {
            "Token saved"
        };

        if available_width < X_COMPACT_CONTROLS_WIDTH {
            column![
                token,
                row![text(status).size(10), Space::new().width(Fill), save]
                    .align_y(Alignment::Center)
            ]
            .spacing(6)
            .width(Fill)
            .into()
        } else {
            row![token, text(status).size(10), save]
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Fill)
                .into()
        }
    }

    fn view_x_feed_source_controls(&self, available_width: f32) -> Element<'_, Message> {
        let input = text_input("@handle", &self.x_feed.source_input)
            .style(helpers::text_input_style)
            .on_input(Message::XFeedSourceInputChanged)
            .on_submit(Message::XFeedAddSource)
            .size(12)
            .padding([5, 8])
            .width(Fill);

        let add = button(text("Add").size(11).center())
            .on_press(Message::XFeedAddSource)
            .padding([5, 10])
            .style(x_action_button);
        let alerts = self.view_x_feed_alert_button();
        let stream = self.view_x_feed_stream_button();
        let refresh = self.view_x_feed_refresh_button();

        if available_width < X_COMPACT_CONTROLS_WIDTH {
            column![
                input,
                row![add, alerts, stream, Space::new().width(Fill), refresh]
                    .spacing(8)
                    .align_y(Alignment::Center),
            ]
            .spacing(6)
            .width(Fill)
            .into()
        } else {
            row![input, add, alerts, stream, refresh]
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Fill)
                .into()
        }
    }

    fn view_x_feed_alert_button(&self) -> Element<'static, Message> {
        let enabled = self.x_feed.notifications_enabled;
        let label = if enabled { "Alerts: ON" } else { "Alerts: OFF" };

        button(text(label).size(11).center())
            .on_press(Message::ToggleXFeedNotifications)
            .padding([5, 10])
            .style(move |theme: &Theme, status| x_toggle_button(theme, status, enabled))
            .into()
    }

    fn view_x_feed_stream_button(&self) -> Element<'static, Message> {
        let enabled = self.x_feed.streaming_enabled;
        let connected = self.x_feed.stream_connected;
        let label = if enabled && connected {
            "Live: ON"
        } else if enabled {
            "Live: ..."
        } else {
            "Live: OFF"
        };

        button(text(label).size(11).center())
            .on_press(Message::ToggleXFeedStreaming)
            .padding([5, 10])
            .style(move |theme: &Theme, status| x_toggle_button(theme, status, enabled))
            .into()
    }

    fn view_x_feed_refresh_button(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = if self.x_feed.loading {
            self.view_spinner(13)
        } else {
            text("\u{21bb}")
                .size(13)
                .center()
                .font(crate::app_fonts::monospace_font())
                .into()
        };

        let mut refresh = button(content).padding([4, 8]).style(subtle_x_icon_button);
        if !self.x_feed.refreshing() {
            refresh = refresh.on_press(Message::RefreshXFeed);
        }

        tooltip(refresh, text("Refresh").size(10), tooltip::Position::Top).into()
    }

    fn view_x_feed_sources(&self) -> Element<'_, Message> {
        let theme = self.theme();
        if self.x_feed.handles.is_empty() {
            return text("No sources")
                .size(11)
                .color(theme.extended_palette().background.weak.text)
                .into();
        }

        let collapsible = self.x_feed.handles.len() > X_SOURCE_COLLAPSE_THRESHOLD;
        if collapsible && !self.x_feed.sources_expanded {
            return x_source_collapse_summary(
                self.x_feed.handles.len(),
                self.x_feed.refreshing(),
                false,
                theme.palette().primary,
                theme.extended_palette().background.weak.text,
            );
        }

        let chips = self
            .x_feed
            .handles
            .iter()
            .fold(
                row![].spacing(6).align_y(Alignment::Center),
                |sources, handle| {
                    sources.push(x_source_chip(
                        handle.clone(),
                        theme.extended_palette().background.weak.text,
                    ))
                },
            )
            .wrap()
            .vertical_spacing(6);

        if collapsible {
            column![
                x_source_collapse_summary(
                    self.x_feed.handles.len(),
                    self.x_feed.refreshing(),
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

    fn view_x_feed_body(&self, now_ms: u64) -> Element<'_, Message> {
        let theme = self.theme();
        let posts = self.x_feed.visible_posts();

        if posts.is_empty() {
            let label = if self.x_feed.loading {
                "Loading posts..."
            } else if self.x_feed.bearer_token.trim().is_empty() {
                "Enter an X API bearer token"
            } else if self.x_feed.handles.is_empty() {
                "Add a public X handle"
            } else {
                "No posts found"
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
                let profile = self.x_feed.profiles.get(&post.author_id).cloned();
                let impacts = self.x_ticker_impact_cards(&post);
                rows.push(x_post_card(
                    post,
                    profile,
                    impacts,
                    now_ms,
                    XPostCardPalette {
                        primary_text: theme.palette().primary,
                        text_color: theme.palette().text,
                        muted_text: theme.extended_palette().background.weak.text,
                        success_text: theme.palette().success,
                        danger_text: theme.palette().danger,
                    },
                ))
            });

        scrollable(rows)
            .direction(Direction::Vertical(
                Scrollbar::new().width(4).margin(0).scroller_width(4),
            ))
            .height(Fill)
            .into()
    }

    fn x_ticker_impact_cards(&self, post: &XFeedPost) -> Vec<XTickerImpactCard> {
        post.ticker_mentions
            .iter()
            .filter(|mention| {
                self.resolve_exchange_symbol_by_key_or_ticker(&mention.symbol)
                    .is_some_and(|symbol| {
                        symbol.market_type != MarketType::Spot
                            && self.exchange_symbol_is_orderable(symbol)
                    })
            })
            .map(|mention| XTickerImpactCard {
                ticker: mention.ticker.clone(),
                impact_pct: x_price_impact_pct(
                    mention.reference_price,
                    self.resolve_mid_for_symbol(&mention.symbol),
                ),
            })
            .collect()
    }
}

fn x_source_chip(handle: String, label_color: Color) -> Element<'static, Message> {
    let remove_handle = handle.clone();
    container(
        row![
            text(format!("@{handle}")).size(11).color(label_color),
            button(text("x").size(10).center())
                .on_press(Message::XFeedRemoveSource(remove_handle))
                .padding([0, 4])
                .style(subtle_x_icon_button),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding([3, 6])
    .style(x_chip_container)
    .into()
}

fn x_source_collapse_summary(
    source_count: usize,
    refreshing: bool,
    expanded: bool,
    active_text: Color,
    muted_text: Color,
) -> Element<'static, Message> {
    let label = if refreshing {
        format!("{source_count} sources - refreshing")
    } else {
        format!("{source_count} sources")
    };
    let toggle_label = if expanded { "Hide" } else { "Show" };

    container(
        row![
            text(label).size(11).color(muted_text),
            button(text(toggle_label).size(10).color(active_text))
                .on_press(Message::ToggleXFeedSourcesExpanded)
                .padding([0, 4])
                .style(subtle_x_icon_button),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([3, 6])
    .style(x_chip_container)
    .into()
}

#[derive(Debug, Clone, Copy)]
struct XPostCardPalette {
    primary_text: Color,
    text_color: Color,
    muted_text: Color,
    success_text: Color,
    danger_text: Color,
}

fn x_post_card(
    post: XFeedPost,
    profile: Option<XFeedAuthorProfile>,
    impacts: Vec<XTickerImpactCard>,
    now_ms: u64,
    palette: XPostCardPalette,
) -> Element<'static, Message> {
    let XPostCardPalette {
        primary_text,
        text_color,
        muted_text,
        success_text,
        danger_text,
    } = palette;
    let age = x_age_countdown_label(post.timestamp_ms, now_ms);
    let latency = x_arrival_latency_label(&post);
    let heat = x_new_post_heat(post.first_seen_ms, now_ms);
    let identity = x_author_identity(profile.as_ref(), &post.username, primary_text, muted_text);
    let post_url = post.url.clone();

    let mut metadata = row![text(age).size(10).color(muted_text)]
        .spacing(6)
        .align_y(Alignment::Center);
    if let Some(latency) = latency {
        metadata = metadata.push(text(latency).size(10).color(muted_text));
    }

    let mut content = column![
        row![
            identity,
            Space::new().width(Fill),
            tooltip(
                button(text("link").size(10).center())
                    .on_press(Message::CopyToClipboard(post_url))
                    .padding([2, 6])
                    .style(subtle_x_icon_button),
                text("Copy link").size(10),
                tooltip::Position::Top,
            )
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        metadata,
        text(post.text).size(12).color(text_color).width(Fill),
    ]
    .spacing(6)
    .width(Fill);

    if !impacts.is_empty() {
        content = content.push(x_ticker_impact_cards(
            impacts,
            muted_text,
            success_text,
            danger_text,
        ));
    }

    container(content)
        .width(Fill)
        .padding(9)
        .style(move |theme: &Theme| x_post_container(theme, heat))
        .into()
}

fn x_author_identity(
    profile: Option<&XFeedAuthorProfile>,
    fallback_username: &str,
    primary_text: Color,
    muted_text: Color,
) -> Element<'static, Message> {
    let initials = profile
        .map(|profile| profile.initials.clone())
        .unwrap_or_else(|| {
            fallback_username
                .chars()
                .take(2)
                .collect::<String>()
                .to_uppercase()
        });
    let name = profile
        .map(|profile| profile.name.clone())
        .unwrap_or_else(|| format!("@{fallback_username}"));
    let username = profile
        .map(|profile| profile.username.clone())
        .unwrap_or_else(|| fallback_username.to_string());

    row![
        container(text(initials).size(10).color(primary_text).center())
            .width(24)
            .height(24)
            .center_x(24)
            .center_y(24)
            .style(move |theme: &Theme| x_avatar_placeholder_style(theme, primary_text)),
        column![
            text(name).size(12).color(primary_text),
            text(format!("@{username}")).size(10).color(muted_text),
        ]
        .spacing(1)
    ]
    .spacing(7)
    .align_y(Alignment::Center)
    .into()
}

fn x_ticker_impact_cards(
    impacts: Vec<XTickerImpactCard>,
    muted_text: Color,
    success_text: Color,
    danger_text: Color,
) -> Element<'static, Message> {
    impacts
        .into_iter()
        .fold(
            row![].spacing(6).align_y(Alignment::Center),
            |row, impact| {
                row.push(x_ticker_impact_card(
                    impact,
                    muted_text,
                    success_text,
                    danger_text,
                ))
            },
        )
        .wrap()
        .vertical_spacing(6)
        .into()
}

fn x_ticker_impact_card(
    impact: XTickerImpactCard,
    muted_text: Color,
    success_text: Color,
    danger_text: Color,
) -> Element<'static, Message> {
    let impact_label = x_impact_label(impact.impact_pct);
    let impact_color = x_impact_color(impact.impact_pct, muted_text, success_text, danger_text);
    container(
        row![
            text(impact.ticker).size(11).color(impact_color),
            text(impact_label).size(10).color(impact_color),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding([3, 6])
    .style(move |theme: &Theme| x_ticker_impact_container(theme, impact_color))
    .into()
}

fn x_impact_label(impact_pct: Option<f64>) -> String {
    impact_pct
        .map(|pct| format!("{pct:+.2}%"))
        .unwrap_or_else(|| "--".to_string())
}

fn x_impact_color(
    impact_pct: Option<f64>,
    muted_text: Color,
    success_text: Color,
    danger_text: Color,
) -> Color {
    match impact_pct {
        Some(pct) if pct > 0.0 => success_text,
        Some(pct) if pct < 0.0 => danger_text,
        Some(_) => muted_text,
        None => muted_text,
    }
}

fn x_action_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => palette.primary.strong.color,
        _ => palette.primary.base.color,
    };
    button::Style {
        background: Some(background.into()),
        text_color: palette.primary.base.text,
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn x_toggle_button(theme: &Theme, status: button::Status, active: bool) -> button::Style {
    let palette = theme.extended_palette();
    let base = if active {
        palette.primary.base
    } else {
        palette.background.weak
    };
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => {
            if active {
                palette.primary.strong.color
            } else {
                palette.background.strong.color
            }
        }
        _ => base.color,
    };
    button::Style {
        background: Some(background.into()),
        text_color: base.text,
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn subtle_x_icon_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => {
            Some(palette.background.strong.color.into())
        }
        _ => None,
    };
    button::Style {
        background,
        text_color: palette.background.weak.text,
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn x_chip_container(theme: &Theme) -> container_style::Style {
    let palette = theme.extended_palette();
    container_style::Style {
        background: Some(palette.background.weak.color.into()),
        border: iced::Border {
            radius: 4.0.into(),
            color: palette.background.strong.color,
            width: 1.0,
        },
        ..Default::default()
    }
}

fn x_avatar_placeholder_style(theme: &Theme, text_color: Color) -> container_style::Style {
    container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        text_color: Some(text_color),
        border: iced::Border {
            radius: 12.0.into(),
            color: theme.extended_palette().background.strong.color,
            width: 1.0,
        },
        ..Default::default()
    }
}

fn x_ticker_impact_container(theme: &Theme, color: Color) -> container_style::Style {
    container_style::Style {
        background: Some(theme.extended_palette().background.weak.color.into()),
        border: iced::Border {
            radius: 4.0.into(),
            color,
            width: 1.0,
        },
        ..Default::default()
    }
}

fn x_post_container(theme: &Theme, heat: f32) -> container_style::Style {
    let palette = theme.extended_palette();
    let heat = heat.clamp(0.0, 1.0);
    let background = if heat > 0.0 {
        let base = palette.background.base.color;
        let primary = palette.primary.weak.color;
        Color {
            r: base.r + (primary.r - base.r) * heat * 0.35,
            g: base.g + (primary.g - base.g) * heat * 0.35,
            b: base.b + (primary.b - base.b) * heat * 0.35,
            a: 1.0,
        }
    } else {
        palette.background.base.color
    };

    container_style::Style {
        background: Some(background.into()),
        border: iced::Border {
            radius: 6.0.into(),
            color: palette.background.strong.color,
            width: 1.0,
        },
        ..Default::default()
    }
}
