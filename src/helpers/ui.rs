mod buttons;
mod colors;
mod inputs;
mod labels;
mod panes;

pub use buttons::{buy_button, order_type_button, sell_button};
pub use colors::{optional_value_color, signed_number_color, text_color_for_bg};
pub use inputs::text_input_style;
pub use labels::{label_value, label_value_colored, vertical_spacer};
pub use panes::pane_title;

pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let inv = 1.0 - t;
    1.0 - inv * inv * inv
}
