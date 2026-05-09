use crate::app_state::TradingTerminal;
use crate::calendar_state::{CalendarImpactFilter, CalendarWindowFilter};
use crate::message::Message;
use iced::widget::{button, container, row, rule, text};
use iced::{Color, Element, Theme};

impl TradingTerminal {
    pub(crate) fn view_calendar_filters(&self) -> Element<'_, Message> {
        let filter_button = |label: &'static str, active: bool, msg: Message| {
            button(text(label).size(10).center())
                .on_press(msg)
                .padding([2, 7])
                .style(move |theme: &Theme, status| {
                    let bg = if active {
                        theme.extended_palette().background.strong.color
                    } else {
                        match status {
                            button::Status::Hovered => {
                                theme.extended_palette().background.strong.color
                            }
                            _ => Color::TRANSPARENT,
                        }
                    };
                    button::Style {
                        background: Some(bg.into()),
                        text_color: if active {
                            theme.palette().primary
                        } else {
                            theme.palette().text
                        },
                        border: iced::Border {
                            radius: 3.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
        };

        row![
            filter_button(
                "Upcoming",
                self.calendar_window_filter == CalendarWindowFilter::Upcoming,
                Message::CalendarWindowFilterChanged(CalendarWindowFilter::Upcoming),
            ),
            filter_button(
                "Today",
                self.calendar_window_filter == CalendarWindowFilter::Today,
                Message::CalendarWindowFilterChanged(CalendarWindowFilter::Today),
            ),
            filter_button(
                "Week",
                self.calendar_window_filter == CalendarWindowFilter::Week,
                Message::CalendarWindowFilterChanged(CalendarWindowFilter::Week),
            ),
            container(rule::vertical(1)).height(16).width(8),
            filter_button(
                "Med+",
                self.calendar_impact_filter == CalendarImpactFilter::MediumHigh,
                Message::CalendarImpactFilterChanged(CalendarImpactFilter::MediumHigh),
            ),
            filter_button(
                "High",
                self.calendar_impact_filter == CalendarImpactFilter::High,
                Message::CalendarImpactFilterChanged(CalendarImpactFilter::High),
            ),
            filter_button(
                "All",
                self.calendar_impact_filter == CalendarImpactFilter::All,
                Message::CalendarImpactFilterChanged(CalendarImpactFilter::All),
            ),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center)
        .wrap()
        .into()
    }
}
