use super::*;

#[test]
fn pane_dividers_hide_lines_without_changing_surfaces_or_geometry() {
    for theme in [Theme::Dark, Theme::Light] {
        let visible = pane_content_style(&theme, 12.0, true);
        let hidden = pane_content_style(&theme, 12.0, false);
        assert!(visible.border.color.a > 0.0);
        assert_eq!(hidden.border.color, Color::TRANSPARENT);
        assert_eq!(hidden.background, visible.background);
        assert_eq!(hidden.border.width, visible.border.width);
        assert_eq!(hidden.border.radius, visible.border.radius);

        let visible_title = pane_title_bar_style(&theme, 12.0, true);
        let hidden_title = pane_title_bar_style(&theme, 12.0, false);
        assert_eq!(
            visible_title.background,
            Some(theme.extended_palette().background.strong.color.into())
        );
        assert_eq!(hidden_title.background, visible_title.background);
        // The header must share the pane outline: same width and color as the
        // content border, rounded only on the top corners.
        assert_eq!(visible_title.border.width, visible.border.width);
        assert_eq!(visible_title.border.color, visible.border.color);
        assert_eq!(
            visible_title.border.radius,
            iced::border::Radius::default().top(12.0)
        );
        assert_eq!(hidden_title.border.color, Color::TRANSPARENT);
        assert_eq!(hidden_title.border.width, visible_title.border.width);
        assert_eq!(hidden_title.border.radius, visible_title.border.radius);

        let visible_body = pane_body_style(&theme, 12.0, true);
        let hidden_body = pane_body_style(&theme, 12.0, false);
        assert_eq!(visible_body.background, visible.background);
        assert_eq!(hidden_body.background, visible_body.background);
        // The body wrapper must complete the pane outline: same width and
        // color as the header border, rounded only on the bottom corners.
        assert_eq!(visible_body.border.width, visible_title.border.width);
        assert_eq!(visible_body.border.color, visible_title.border.color);
        assert_eq!(
            visible_body.border.radius,
            iced::border::Radius::default().bottom(12.0)
        );
        assert_eq!(hidden_body.border.color, Color::TRANSPARENT);
        assert_eq!(hidden_body.border.width, visible_body.border.width);
        assert_eq!(hidden_body.border.radius, visible_body.border.radius);

        let visible_grid = pane_grid_style(&theme, 12.0, 8.0, true);
        let hidden_grid = pane_grid_style(&theme, 12.0, 8.0, false);
        assert_eq!(visible_grid.hovered_split.color, theme.palette().primary);
        assert_eq!(hidden_grid.hovered_split.color, Color::TRANSPARENT);
        assert_eq!(hidden_grid.picked_split.color, Color::TRANSPARENT);
        assert_eq!(
            hidden_grid.hovered_split.width,
            visible_grid.hovered_split.width
        );
        assert_eq!(
            hidden_grid.picked_split.width,
            visible_grid.picked_split.width
        );
        // Drag-and-drop placement feedback remains visible.
        assert_eq!(
            hidden_grid.hovered_region.background,
            visible_grid.hovered_region.background
        );
    }
}
