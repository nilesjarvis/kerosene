use super::*;
use crate::config::KeroseneConfig;
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Font, Pixels, Rectangle, Size};

#[test]
fn liquidation_summary_hides_timeframes_from_the_outside_in() {
    for (width, minutes) in [
        (900.0, vec![1, 5, 15, 60]),
        (768.0, vec![1, 5, 15, 60]),
        (767.0, vec![1, 5, 15]),
        (572.0, vec![1, 5, 15]),
        (571.0, vec![5, 15]),
        (376.0, vec![5, 15]),
        (375.0, vec![5]),
        (180.0, vec![5]),
        (120.0, vec![5]),
        (0.0, vec![5]),
    ] {
        assert_eq!(
            visible_summary_timeframes(width)
                .iter()
                .map(|&(minutes, _)| minutes)
                .collect::<Vec<_>>(),
            minutes
        );
    }
}

#[tokio::test]
async fn liquidation_summary_totals_fit_when_compressed() {
    let mut terminal = TradingTerminal::boot_from_config(KeroseneConfig::default()).0;
    terminal
        .liquidation_summary_buckets
        .insert(100, (1_234_567_890.0, 987_654_321.0));
    let now_ms = 100 * 60_000;
    let mut renderer = iced::Renderer::new(Font::DEFAULT, Pixels(12.0), Some("tiny-skia"))
        .await
        .expect("software renderer");
    let theme = terminal.theme();
    for width in [
        120.0, 180.0, 260.0, 375.0, 376.0, 571.0, 572.0, 767.0, 768.0, 900.0,
    ] {
        let mut view = terminal.view_liquidations_summary(now_ms, width);
        let mut tree = Tree::new(view.as_widget());
        let size = Size::new(width, 64.0);
        let bounds = Rectangle::with_size(size);
        let node = view.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, size),
        );
        let summary = Layout::new(&node).children().next().expect("summary row");
        let columns: Vec<_> = summary.children().collect();
        assert_eq!(columns.len(), visible_summary_timeframes(width).len());
        for column in &columns {
            let column_bounds = column.bounds();
            let text_block = column.children().next().expect("text block");
            for line in text_block.children() {
                let cells: Vec<_> = line.children().collect();
                for cell in &cells {
                    assert!(
                        cell.bounds().x >= column_bounds.x - 0.01,
                        "left overflow at {width}"
                    );
                    assert!(
                        cell.bounds().x + cell.bounds().width
                            <= column_bounds.x + column_bounds.width + 0.01,
                        "right overflow at {width}"
                    );
                }
                assert!(
                    cells
                        .windows(2)
                        .all(|pair| pair[0].bounds().x + pair[0].bounds().width
                            <= pair[1].bounds().x + 0.01),
                    "overlap at {width}"
                );
            }
        }
        iced::advanced::Renderer::reset(&mut renderer, bounds);
        view.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style {
                text_color: theme.palette().text,
            },
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &bounds,
        );
        if let Some(directory) = std::env::var_os("KEROSENE_LIQUIDATION_SUMMARY_PREVIEW_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).expect("preview directory");
            let pixels =
                renderer.screenshot(Size::new(width as u32, 64), 1.0, theme.palette().background);
            image::save_buffer(
                directory.join(format!("{width}.png")),
                &pixels,
                width as u32,
                64,
                image::ColorType::Rgba8,
            )
            .expect("synthetic summary preview");
        }
    }
}
