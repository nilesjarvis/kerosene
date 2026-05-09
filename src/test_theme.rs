use iced::{Color, Theme};

fn main() {
    let t = Theme::Dark;
    let p = t.palette();
    println!("{:?}", p.background);
}
