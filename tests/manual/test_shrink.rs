use iced::{widget::{container, scrollable, column, text}, Length};
fn test() {
    let _c = container(scrollable(column![text("hello")]))
        .width(Length::Shrink)
        .height(Length::Shrink)
        .max_height(250.0);
}
