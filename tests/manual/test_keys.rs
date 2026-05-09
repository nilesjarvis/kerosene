use iced::{event, keyboard, window, Event, Subscription, subscription};

#[derive(Clone, Debug, PartialEq)]
enum Message {
    Test(usize),
}

fn subs(action: usize) -> Subscription<Message> {
    iced::event::listen_with(|event, _status, _window| {
        match event {
            Event::Keyboard(keyboard::Event::KeyPressed { .. }) => Some(Message::Test(123)),
            _ => None,
        }
    }).map(move |msg| {
        if msg == Message::Test(123) {
            Message::Test(action)
        } else {
            msg
        }
    })
}

fn main() {}
