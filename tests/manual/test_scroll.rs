use iced::widget::scrollable;

fn main() {
    let _s: iced::widget::Scrollable<'_, ()> = iced::widget::scrollable(iced::widget::text("a"))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().spacing(5)
        ));
}
