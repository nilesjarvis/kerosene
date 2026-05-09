import re

with open('src/main.rs', 'r') as f:
    content = f.read()

# Make the divider bright white to see if it even renders
old_code = """        let divider = container(Space::new().width(Fill).height(2))
            .style(|theme: &Theme| iced::widget::container::Style {
                background: Some(theme.extended_palette().background.strong.color.into()),
                ..Default::default()
            });"""

new_code = """        let divider = container(Space::new().width(Fill).height(2))
            .style(|_theme: &Theme| iced::widget::container::Style {
                background: Some(iced::Color::from_rgb(1.0, 1.0, 1.0).into()),
                ..Default::default()
            });"""

content = content.replace(old_code, new_code)

with open('src/main.rs', 'w') as f:
    f.write(content)
