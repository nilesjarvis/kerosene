use crate::app_state::TradingTerminal;
use crate::message::Message;

use super::components::chart_screenshot_button;

use iced::widget::{column, container, image as image_widget, row, text};
use iced::{ContentFit, Element, Fill, Length, Theme};

// ---------------------------------------------------------------------------
// Screenshot Window
// ---------------------------------------------------------------------------

impl TradingTerminal {
    pub(crate) fn view_chart_screenshot_window(&self) -> Element<'_, Message> {
        let theme = self.theme();
        if self.chart_screenshot_capture_in_progress {
            let content = column![
                text("Capturing chart...")
                    .size(14)
                    .color(theme.palette().text),
                text("Preparing high-resolution image")
                    .size(11)
                    .color(theme.extended_palette().background.weak.text),
                chart_screenshot_button("Close", Message::CloseChartScreenshotWindow),
            ]
            .spacing(10)
            .align_x(iced::Alignment::Center);

            return container(content)
                .width(Fill)
                .height(Fill)
                .center(Fill)
                .padding(16)
                .into();
        }

        let Some(state) = &self.chart_screenshot else {
            let message = self
                .chart_screenshot_error
                .as_deref()
                .unwrap_or("No chart screenshot available");
            let content = column![
                text(message)
                    .size(14)
                    .color(if self.chart_screenshot_error.is_some() {
                        theme.palette().danger
                    } else {
                        theme.palette().text
                    }),
                chart_screenshot_button("Close", Message::CloseChartScreenshotWindow),
            ]
            .spacing(12)
            .align_x(iced::Alignment::Center);

            return container(content)
                .width(Fill)
                .height(Fill)
                .center(Fill)
                .padding(16)
                .into();
        };

        let preview = image_widget(state.preview_handle.clone())
            .content_fit(ContentFit::Contain)
            .width(Fill)
            .height(Fill);

        let metadata = text(format!(
            "{} {}  {}x{}  {}",
            state.symbol,
            state.timeframe,
            state.width,
            state.height,
            state.captured_at.format("%Y-%m-%d %H:%M:%S")
        ))
        .size(11)
        .font(crate::app_fonts::monospace_font())
        .color(theme.extended_palette().background.weak.text);

        let actions = row![
            chart_screenshot_button("Copy Image", Message::CopyChartScreenshot),
            chart_screenshot_button("Save PNG", Message::SaveChartScreenshot),
            chart_screenshot_button("Close", Message::CloseChartScreenshotWindow),
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center);

        let content = column![
            row![
                text("Chart Screenshot")
                    .size(16)
                    .color(theme.palette().text),
                iced::widget::Space::new().width(Fill),
                metadata,
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center),
            container(preview)
                .width(Fill)
                .height(Length::Fill)
                .style(|theme: &Theme| {
                    let ext = theme.extended_palette();
                    container::Style {
                        background: Some(ext.background.base.color.into()),
                        border: iced::Border {
                            color: ext.background.strong.color,
                            width: 1.0,
                            radius: 4.0.into(),
                        },
                        ..Default::default()
                    }
                }),
            actions,
        ]
        .spacing(12);

        container(content)
            .width(Fill)
            .height(Fill)
            .padding(14)
            .into()
    }
}
