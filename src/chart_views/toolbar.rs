mod drawing;
mod sections;
mod selection;

pub(super) use selection::view_annotation_style_bar;

use crate::account_views::{AnchoredAccountMenu, AnchoredMenuLayer, MenuAlignment, MenuKind};
use crate::annotations::DrawingMenu;
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartInstance, ChartSurfaceId};
use crate::message::Message;
use crate::timeframe::chart_timeframe_options;
use iced::widget::{pick_list, row};
use iced::{Color, Element, Length, Theme};

impl TradingTerminal {
    pub(crate) fn view_chart_toolbar(
        &self,
        chart_id: ChartId,
        instance: &ChartInstance,
        surface_id: ChartSurfaceId,
    ) -> Element<'_, Message> {
        let theme = self.theme();
        let has_candles = !instance.chart.candles.is_empty();
        let active = instance.interval;
        let active_tool = self.active_chart_surface_tool(chart_id, surface_id);
        let hydromancer_one_second_available = !self.hydromancer_api_key.trim().is_empty();
        let timeframe_options = chart_timeframe_options(hydromancer_one_second_available);
        let selected_timeframe = timeframe_options.contains(&active).then_some(active);
        let tf_picker = pick_list(timeframe_options, selected_timeframe, move |tf| {
            Message::ChartSwitchTimeframe(chart_id, tf)
        })
        .width(Length::Shrink)
        .padding([3, 8])
        .text_size(11)
        .style(chart_toolbar_pick_list_style);

        let reload_btn = sections::chart_reload_button(chart_id);
        let reset_view_btn = sections::chart_reset_view_button(chart_id, surface_id);
        let header_collapse_btn = match surface_id {
            ChartSurfaceId::Detached(_) => {
                Some(self.view_chart_header_collapse_button(chart_id, instance.header_collapsed))
            }
            ChartSurfaceId::Docked(_) => None,
        };

        let mut controls_row = row![
            tf_picker,
            sections::chart_toolbar_separator(),
            self.view_indicator_dropdown(chart_id, instance),
            sections::chart_toolbar_separator(),
            drawing::view_drawing_button(chart_id, surface_id, instance, active_tool),
            sections::chart_toolbar_separator(),
            reload_btn,
            sections::chart_toolbar_separator(),
            reset_view_btn,
        ]
        .spacing(0)
        .align_y(iced::Alignment::Center);
        if let Some(btn) = header_collapse_btn {
            controls_row = controls_row
                .push(sections::chart_toolbar_separator())
                .push(btn);
        }

        if let Some(status) = sections::chart_fetch_status_label(has_candles, instance, &theme) {
            controls_row = controls_row
                .push(sections::chart_toolbar_separator())
                .push(status);
        }
        controls_row = sections::push_chart_mode_buttons(controls_row, chart_id, instance);

        let menu = (instance.drawing_menu == Some((surface_id, DrawingMenu::Tools))).then(|| {
            AnchoredMenuLayer {
                kind: MenuKind::DrawingTools,
                alignment: MenuAlignment::Start,
                content: drawing::tool_palette(chart_id, surface_id, active_tool),
            }
        });
        AnchoredAccountMenu::new(sections::chart_toolbar_strip(controls_row), menu).into()
    }

    fn view_indicator_dropdown(
        &self,
        chart_id: ChartId,
        instance: &ChartInstance,
    ) -> Element<'_, Message> {
        let button = sections::chart_toolbar_button(
            "IND ▾",
            instance.macro_menu_open,
            Message::ToggleMacroMenu(chart_id),
        );
        let menu = instance.macro_menu_open.then(|| AnchoredMenuLayer {
            kind: MenuKind::ChartIndicators,
            alignment: MenuAlignment::FlushStart,
            content: self.view_macro_indicator_menu(chart_id, instance),
        });
        AnchoredAccountMenu::new(button, menu).into()
    }
}

fn chart_toolbar_pick_list_style(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let background = match status {
        pick_list::Status::Hovered | pick_list::Status::Opened { .. } => Color {
            a: 0.55,
            ..theme.extended_palette().background.strong.color
        },
        pick_list::Status::Active => Color::TRANSPARENT,
    };

    pick_list::Style {
        text_color: theme.extended_palette().background.weak.text,
        placeholder_color: theme.extended_palette().background.weak.text,
        handle_color: theme.extended_palette().background.weak.text,
        background: background.into(),
        border: iced::Border {
            radius: 0.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::annotations::DrawingTool;
    use crate::timeframe::Timeframe;
    use iced::advanced::renderer::Headless;
    use iced::advanced::{layout, widget::Tree};
    use iced::{Font, Pixels, Renderer, Size};

    #[tokio::test]
    async fn indicators_animate_from_the_button_and_fit_short_windows() {
        use iced::advanced::{Layout, Shell, clipboard, mouse, renderer};
        use iced::{Event, Point, Rectangle, Vector, window};
        use std::time::{Duration, Instant};

        let mut renderer = Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
            .await
            .expect("software renderer");
        let mut terminal = TradingTerminal::boot().0;
        terminal
            .charts
            .insert(7, ChartInstance::new(7, "BTC".into(), Timeframe::H1));
        let _ = terminal.update(Message::ToggleMacroMenu(7));
        for (height, button_y, below) in [
            (600.0, 64.0, true),
            (205.0, 30.0, true),
            (205.0, 160.0, false),
        ] {
            let size = Size::new(640.0, height);
            let mut dropdown = terminal.view_indicator_dropdown(7, &terminal.charts[&7]);
            let mut tree = Tree::new(dropdown.as_widget());
            let button = dropdown
                .as_widget_mut()
                .layout(&mut tree, &renderer, &layout::Limits::new(Size::ZERO, size))
                .move_to(Point::new(91.0, button_y));
            let mut overlay = dropdown
                .as_widget_mut()
                .overlay(
                    &mut tree,
                    Layout::new(&button),
                    &renderer,
                    &Rectangle::with_size(size),
                    Vector::ZERO,
                )
                .expect("open indicator dropdown");
            let menu = overlay.as_overlay_mut();
            let opening = menu.layout(&renderer, size);
            iced::advanced::Renderer::reset(&mut renderer, Rectangle::with_size(size));
            menu.draw(
                &mut renderer,
                &Theme::Dark,
                &renderer::Style::default(),
                Layout::new(&opening),
                mouse::Cursor::Unavailable,
            );
            let pixels = renderer.screenshot(
                Size::new(size.width as u32, size.height as u32),
                1.0,
                Color::BLACK,
            );
            assert!(
                pixels.chunks_exact(4).all(|pixel| pixel == &pixels[..4]),
                "scrollable controls must stay hidden until the reveal starts"
            );
            let now = Instant::now();
            for frame in [now, now + Duration::from_millis(200)] {
                menu.update(
                    &Event::Window(window::Event::RedrawRequested(frame)),
                    Layout::new(&opening),
                    mouse::Cursor::Unavailable,
                    &renderer,
                    &mut clipboard::Null,
                    &mut Shell::new(&mut Vec::new()),
                );
            }
            let settled = menu.layout(&renderer, size).bounds();
            assert_eq!(settled.x, button.bounds().x);
            assert!(opening.bounds().y < settled.y);
            assert!(settled.y >= 0.0 && settled.y + settled.height <= height);
            if below {
                assert!(settled.y >= button_y + button.size().height);
            } else {
                assert!(settled.y + settled.height <= button_y);
            }
        }
    }

    #[tokio::test]
    async fn draw_button_fits_the_selected_tool_label() {
        let renderer = Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
            .await
            .expect("software renderer");
        let chart = ChartInstance::new(7, "BTC".into(), Timeframe::H1);
        let button_width = |tool| {
            let mut button =
                drawing::view_drawing_button(7, ChartSurfaceId::Docked(7), &chart, tool);
            let mut tree = Tree::new(button.as_widget());
            button
                .as_widget_mut()
                .layout(
                    &mut tree,
                    &renderer,
                    &layout::Limits::new(Size::ZERO, Size::new(320.0, 40.0)),
                )
                .size()
                .width
        };
        assert!(button_width(None) < button_width(Some(DrawingTool::FibRetracement)));
    }

    #[tokio::test]
    async fn drawing_tools_and_open_palette_keep_the_chart_toolbar_height_stable() {
        let renderer = Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
            .await
            .expect("software renderer");
        let mut terminal = TradingTerminal::boot().0;
        let surface = ChartSurfaceId::Docked(7);
        terminal
            .charts
            .insert(7, ChartInstance::new(7, "BTC".into(), Timeframe::H1));
        for width in [260.0, 320.0, 680.0] {
            let mut idle_height = None;
            for (tool, open) in [
                (None, false),
                (None, true),
                (Some(DrawingTool::HorizontalLevel), false),
                (Some(DrawingTool::FibRetracement), false),
                (Some(DrawingTool::VerticalLine), true),
            ] {
                let _ = terminal.update_annotations(Message::SetDrawingTool(7, surface, tool));
                if open {
                    let _ = terminal.update_annotations(Message::ToggleDrawingMenu(
                        7,
                        surface,
                        DrawingMenu::Tools,
                    ));
                }
                let mut view = terminal.view_chart_toolbar(7, &terminal.charts[&7], surface);
                let mut tree = Tree::new(view.as_widget());
                let node = view.as_widget_mut().layout(
                    &mut tree,
                    &renderer,
                    &layout::Limits::new(Size::ZERO, Size::new(width, 300.0)),
                );
                let height = node.size().height;
                assert_eq!(
                    height,
                    *idle_height.get_or_insert(height),
                    "width {width}, tool {tool:?}, open {open}"
                );
            }
        }
    }
}
