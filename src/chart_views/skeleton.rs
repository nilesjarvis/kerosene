use crate::chart::{CandlestickChart, TIME_AXIS_HEIGHT};
use crate::message::Message;

use iced::widget::canvas;
use iced::widget::container as container_style;
use iced::widget::container;
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Size, Theme};

mod drawing;
mod sample;
mod style;

use drawing::{
    draw_axis_borders, draw_chart_grid, draw_funding_panel, draw_funding_panel_shimmer,
    draw_price_axis, draw_price_axis_shimmer, draw_skeleton_candles, draw_skeleton_candles_shimmer,
    draw_time_axis, draw_time_axis_shimmer,
};
use style::{Shimmer, SkeletonPalette};

// ---------------------------------------------------------------------------
// Chart Skeleton Loader
// ---------------------------------------------------------------------------

const FUNDING_PANEL_MIN_HEIGHT: f32 = 44.0;
const FUNDING_PANEL_MAX_HEIGHT: f32 = 160.0;

pub(super) fn chart_skeleton_overlay(
    chart: &CandlestickChart,
    phase: f32,
) -> Element<'static, Message> {
    let price_axis_width = chart.price_axis_width();
    let funding_panel_height = chart
        .macro_indicators
        .show_funding_rate
        .then_some(chart.funding_panel_height);

    container(
        iced::widget::canvas(ChartSkeleton {
            phase,
            price_axis_width,
            funding_panel_height,
        })
        .width(Fill)
        .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .style(|theme: &Theme| container_style::Style {
        background: Some(
            Color {
                a: 0.86,
                ..theme.extended_palette().background.strong.color
            }
            .into(),
        ),
        ..Default::default()
    })
    .into()
}

struct ChartSkeleton {
    phase: f32,
    price_axis_width: f32,
    funding_panel_height: Option<f32>,
}

impl canvas::Program<Message> for ChartSkeleton {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let width = bounds.width.max(0.0);
        let height = bounds.height.max(0.0);
        if width <= 0.0 || height <= 0.0 || !width.is_finite() || !height.is_finite() {
            return vec![frame.into_geometry()];
        }

        let palette = SkeletonPalette::new(theme);
        frame.fill_rectangle(Point::ORIGIN, Size::new(width, height), palette.background);

        let price_axis_w = if width >= 52.0 {
            self.price_axis_width.clamp(52.0, width.min(96.0))
        } else {
            width
        };
        let chart_w = (width - price_axis_w).max(0.0);
        let available_chart_h = (height - TIME_AXIS_HEIGHT).max(0.0);
        let funding_h = self
            .funding_panel_height
            .map(|height| {
                height
                    .clamp(FUNDING_PANEL_MIN_HEIGHT, FUNDING_PANEL_MAX_HEIGHT)
                    .min((available_chart_h * 0.35).max(0.0))
            })
            .unwrap_or(0.0);
        let main_h = (available_chart_h - funding_h).max(0.0);

        if chart_w <= 0.0 || main_h <= 0.0 {
            return vec![frame.into_geometry()];
        }

        let shimmer = Shimmer::new(width, self.phase, &palette);
        draw_chart_grid(&mut frame, chart_w, main_h, &palette);
        draw_skeleton_candles(&mut frame, chart_w, main_h, &palette);
        draw_price_axis(&mut frame, width, price_axis_w, main_h, &palette);

        if funding_h > 0.0 {
            draw_funding_panel(&mut frame, chart_w, main_h, funding_h, &palette);
        }

        draw_time_axis(
            &mut frame,
            chart_w,
            main_h + funding_h,
            TIME_AXIS_HEIGHT,
            &palette,
        );
        draw_axis_borders(&mut frame, chart_w, main_h, funding_h, height, &palette);
        draw_skeleton_candles_shimmer(&mut frame, chart_w, main_h, &shimmer);
        draw_price_axis_shimmer(&mut frame, width, price_axis_w, main_h, &shimmer);
        if funding_h > 0.0 {
            draw_funding_panel_shimmer(&mut frame, chart_w, main_h, funding_h, &shimmer);
        }
        draw_time_axis_shimmer(
            &mut frame,
            chart_w,
            main_h + funding_h,
            TIME_AXIS_HEIGHT,
            &shimmer,
        );

        vec![frame.into_geometry()]
    }
}
