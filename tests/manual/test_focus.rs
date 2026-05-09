use iced::widget::text_input;
use iced::Task;

fn main() {
    let id = iced::widget::text_input::Id::new("test");
    let task: Task<()> = text_input::focus(id);
}
