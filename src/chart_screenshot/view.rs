mod components;
mod window;

use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartId, ChartSurfaceId};
use crate::message::Message;

use components::{
    CAMERA_ICON_SVG, CHEVRON_DOWN_ICON_SVG, chart_screenshot_icon_button,
    chart_screenshot_svg_icon, screenshot_menu_separator,
};
use iced::advanced::widget::Id;
use iced::widget::container as container_style;
use iced::widget::{button, checkbox, column, container, row, scrollable, stack, text, tooltip};
use iced::{Alignment, Color, Element, Fill, Length, Theme};

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_chart_screenshot_button(
        &self,
        chart_id: ChartId,
        surface_id: ChartSurfaceId,
    ) -> Element<'static, Message> {
        let capture = tooltip(
            chart_screenshot_icon_button(
                chart_screenshot_svg_icon(CAMERA_ICON_SVG, 14.0),
                Message::OpenChartScreenshot(chart_id, surface_id),
                false,
                [3, 6],
            ),
            text("Capture chart")
                .size(10)
                .font(crate::app_fonts::monospace_font()),
            tooltip::Position::Top,
        );

        let settings = tooltip(
            chart_screenshot_icon_button(
                chart_screenshot_svg_icon(CHEVRON_DOWN_ICON_SVG, 12.0),
                Message::ToggleChartScreenshotMenu(chart_id, surface_id),
                self.chart_screenshot_menu_open == Some(surface_id),
                [3, 3],
            ),
            text("Screenshot settings")
                .size(10)
                .font(crate::app_fonts::monospace_font()),
            tooltip::Position::Top,
        );

        row![capture, settings]
            .spacing(1)
            .align_y(Alignment::Center)
            .into()
    }

    pub(crate) fn view_chart_screenshot_menu(
        &self,
        _surface_id: ChartSurfaceId,
    ) -> Element<'_, Message> {
        let menu_col = column![
            text("Screenshot")
                .size(10)
                .font(crate::app_fonts::monospace_font())
                .color(Color::from_rgb8(0x88, 0x88, 0x88)),
            screenshot_menu_separator(),
            checkbox(self.chart_screenshot_settings.obscure_position_entry)
                .label("Obscure position entry")
                .on_toggle(Message::ToggleChartScreenshotObscurePositionEntry)
                .size(10)
                .spacing(4)
                .text_size(10)
                .font(crate::app_fonts::monospace_font())
                .width(Fill),
            checkbox(self.chart_screenshot_settings.hide_positions_and_orders)
                .label("Hide positions/orders")
                .on_toggle(Message::ToggleChartScreenshotHidePositionsAndOrders)
                .size(10)
                .spacing(4)
                .text_size(10)
                .font(crate::app_fonts::monospace_font())
                .width(Fill),
        ]
        .spacing(5)
        .padding(6)
        .width(Fill);

        let menu_card = container(scrollable(menu_col).height(Length::Shrink))
            .width(220.0)
            .max_height(116.0)
            .style(|theme: &Theme| container_style::Style {
                background: Some(theme.extended_palette().background.strong.color.into()),
                border: iced::Border {
                    color: theme.extended_palette().background.weak.color,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            });

        let bg_overlay = button("")
            .width(Fill)
            .height(Fill)
            .on_press(Message::CloseAllMenus)
            .style(|_theme: &Theme, _status| button::Style {
                background: Some(Color::TRANSPARENT.into()),
                ..Default::default()
            });

        stack![
            bg_overlay,
            container(menu_card)
                .width(Fill)
                .height(Fill)
                .padding([32, 20])
                .align_x(Alignment::End)
                .align_y(Alignment::Start)
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }

    pub(crate) fn chart_screenshot_canvas_id(surface_id: ChartSurfaceId) -> Id {
        Id::from(format!(
            "chart_screenshot_canvas_{}",
            surface_id.widget_suffix()
        ))
    }
}
