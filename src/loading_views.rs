use crate::app_state::TradingTerminal;
use crate::message::Message;

use crate::chart::ChartStatus;
use iced::widget::canvas;
use iced::widget::container as container_style;
use iced::widget::{column, container, text};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Theme};

struct LoadingSpinner {
    phase: f32,
    color: Color,
    thickness: f32,
}

impl canvas::Program<Message> for LoadingSpinner {
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

        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = bounds.width.min(bounds.height) / 2.0 - self.thickness;

        if radius <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let path = canvas::Path::new(|p| {
            p.arc(canvas::path::Arc {
                center,
                radius,
                start_angle: iced::Radians(self.phase),
                end_angle: iced::Radians(self.phase + std::f32::consts::PI * 1.5), // 270 degrees
            });
        });

        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_color(self.color)
                .with_width(self.thickness)
                .with_line_cap(canvas::LineCap::Round),
        );

        vec![frame.into_geometry()]
    }
}

impl TradingTerminal {
    pub(crate) fn has_loading_activity(&self) -> bool {
        let _theme = self.theme();
        self.symbols_loading
            || self.order_books.values().any(|b| b.book_loading)
            || self.account_loading
            || self.portfolio.loading
            || self.income.loading
            || self.calendar_loading
            || self.hype_etfs.loading
            || self
                .wallet_tracker
                .rows
                .values()
                .any(|r| r.loading || r.order_loading)
            || self
                .wallet_detail_windows
                .values()
                .any(|state| state.loading)
            || self.live_watchlist_contexts_loading
            || self.live_watchlist_history_loading
            || self.active_move_order_drag.is_some()
            || !self.pending_move_order_contexts.is_empty()
            || !self.pending_order_indicators.is_empty()
            || !self.chase_orders.is_empty()
            || self.charts.values().any(|inst| {
                matches!(inst.chart.status, ChartStatus::Loading)
                    || inst.candle_fetch_request.is_some()
                    || inst.funding_fetch_request.is_some()
                    || inst.liquidation_fetching
                    || inst.heatmap_fetching
                    || inst.chart.quick_order_limit_price.is_some()
            })
    }

    pub(crate) fn view_spinner(&self, size: u32) -> Element<'_, Message> {
        let theme = self.theme();
        let thickness = (size as f32 * 0.15).clamp(1.5, 4.0);

        container(
            iced::widget::canvas(LoadingSpinner {
                phase: self.spinner_phase,
                color: theme.palette().primary,
                thickness,
            })
            .width(size as f32)
            .height(size as f32),
        )
        .width(size)
        .height(size)
        .center(Fill)
        .into()
    }

    pub(crate) fn loading_overlay(&self, label: &'static str) -> Element<'_, Message> {
        let theme = self.theme();
        container(
            column![
                self.view_spinner(34),
                text(label).size(12).color(theme.palette().text),
            ]
            .spacing(8)
            .align_x(iced::Alignment::Center),
        )
        .width(Fill)
        .height(Fill)
        .center(Fill)
        .style(|theme: &Theme| container_style::Style {
            background: Some(
                Color {
                    a: 0.72,
                    ..theme.extended_palette().background.strong.color
                }
                .into(),
            ),
            ..Default::default()
        })
        .into()
    }
}
