use crate::annotations::DrawingTool;
use crate::app_state::TradingTerminal;
use crate::chart_state::{ChartInstance, ChartSurfaceId, DetachedChartWindowState};
use crate::message::Message;
use crate::order_execution::QuickOrderForm;
use crate::pane_state::PaneKind;
use crate::timeframe::Timeframe;

fn quick_order_form() -> QuickOrderForm {
    QuickOrderForm {
        price: 100.0,
        quantity: "1".to_string(),
        quantity_is_usd: false,
        percentage: 0.0,
        quantity_provenance: None,
        is_limit: true,
        click_x: 10.0,
        click_y: 20.0,
        chart_w: 300.0,
        chart_h: 200.0,
    }
}

#[test]
fn workspace_escape_clears_only_the_originating_workspace() {
    let mut terminal = TradingTerminal::boot().0;
    let main_window_id = iced::window::Id::unique();
    let canvas_window_id = iced::window::Id::unique();
    let main_chart_id = terminal
        .panes
        .iter()
        .find_map(|(_, kind)| match kind {
            PaneKind::Chart(id) => Some(*id),
            _ => None,
        })
        .expect("main chart");
    let canvas_chart_id = terminal.next_chart_id;
    terminal.next_chart_id = terminal.next_chart_id.saturating_add(1);
    terminal.main_window_id = Some(main_window_id);

    let main_chart = terminal.charts.get_mut(&main_chart_id).expect("main chart");
    main_chart
        .chart
        .set_surface_id(ChartSurfaceId::Docked(main_chart_id));
    main_chart.editor_open = true;
    main_chart.chart.active_tool = Some(DrawingTool::TrendLine);
    main_chart.set_quick_order(quick_order_form());

    let mut canvas_chart = ChartInstance::new(canvas_chart_id, "ETH".to_string(), Timeframe::H1);
    canvas_chart.editor_open = true;
    canvas_chart.chart.active_tool = Some(DrawingTool::HorizontalLevel);
    canvas_chart.set_quick_order(quick_order_form());
    terminal.charts.insert(canvas_chart_id, canvas_chart);
    terminal.insert_test_canvas_pane(7, PaneKind::Chart(canvas_chart_id));
    terminal.canvases.get_mut(&7).expect("Canvas").window_id = Some(canvas_window_id);
    terminal
        .chart_quick_order_surface
        .insert(main_chart_id, ChartSurfaceId::Docked(main_chart_id));
    terminal
        .chart_quick_order_surface
        .insert(canvas_chart_id, ChartSurfaceId::Docked(canvas_chart_id));
    terminal.chart_surface_active_tools.insert(
        ChartSurfaceId::Docked(main_chart_id),
        DrawingTool::TrendLine,
    );
    terminal.chart_surface_active_tools.insert(
        ChartSurfaceId::Docked(canvas_chart_id),
        DrawingTool::HorizontalLevel,
    );

    let _ = terminal.update_order(Message::EscapePressed(main_window_id));

    let main_chart = terminal.charts.get(&main_chart_id).expect("main chart");
    assert!(!main_chart.editor_open);
    assert!(main_chart.quick_order.is_none());
    assert_eq!(main_chart.chart.active_tool, None);
    let canvas_chart = terminal.charts.get(&canvas_chart_id).expect("Canvas chart");
    assert!(canvas_chart.editor_open);
    assert!(canvas_chart.quick_order.is_some());
    assert_eq!(
        canvas_chart.chart.active_tool,
        Some(DrawingTool::HorizontalLevel)
    );

    let _ = terminal.update_order(Message::EscapePressed(canvas_window_id));

    let canvas_chart = terminal.charts.get(&canvas_chart_id).expect("Canvas chart");
    assert!(!canvas_chart.editor_open);
    assert!(canvas_chart.quick_order.is_none());
    assert_eq!(canvas_chart.chart.active_tool, None);
}

#[test]
fn detached_window_escape_clears_only_matching_chart_transient_state() {
    let mut terminal = TradingTerminal::boot().0;
    let main_window_id = iced::window::Id::unique();
    let detached_window_id = iced::window::Id::unique();
    let other_window_id = iced::window::Id::unique();
    let detached_chart_id = 7;
    let other_chart_id = 8;
    let detached_surface = ChartSurfaceId::Detached(detached_window_id);
    let other_surface = ChartSurfaceId::Detached(other_window_id);
    terminal.main_window_id = Some(main_window_id);
    terminal.charts.clear();
    terminal.detached_chart_windows.clear();
    terminal.chart_quick_order_surface.clear();
    terminal.chart_surface_active_tools.clear();

    let mut detached_chart =
        ChartInstance::new(detached_chart_id, "BTC".to_string(), Timeframe::H1);
    detached_chart.chart.set_surface_id(detached_surface);
    detached_chart.editor_open = true;
    detached_chart.editor_search_query = "bt".to_string();
    detached_chart.editor_selected_index = Some(0);
    detached_chart.chart.active_tool = Some(DrawingTool::TrendLine);
    detached_chart.set_quick_order(quick_order_form());

    let mut other_chart = ChartInstance::new(other_chart_id, "ETH".to_string(), Timeframe::H1);
    other_chart.chart.set_surface_id(other_surface);
    other_chart.editor_open = true;
    other_chart.editor_search_query = "et".to_string();
    other_chart.editor_selected_index = Some(0);
    other_chart.chart.active_tool = Some(DrawingTool::HorizontalLevel);
    other_chart.set_quick_order(quick_order_form());

    terminal.charts.insert(detached_chart_id, detached_chart);
    terminal.charts.insert(other_chart_id, other_chart);
    terminal.detached_chart_windows.insert(
        detached_window_id,
        DetachedChartWindowState::new(detached_chart_id),
    );
    terminal.detached_chart_windows.insert(
        other_window_id,
        DetachedChartWindowState::new(other_chart_id),
    );
    terminal
        .chart_quick_order_surface
        .insert(detached_chart_id, detached_surface);
    terminal
        .chart_quick_order_surface
        .insert(other_chart_id, other_surface);
    terminal
        .chart_surface_active_tools
        .insert(detached_surface, DrawingTool::TrendLine);
    terminal
        .chart_surface_active_tools
        .insert(other_surface, DrawingTool::HorizontalLevel);

    let _ = terminal.update_order(Message::EscapePressed(detached_window_id));

    let detached_chart = terminal
        .charts
        .get(&detached_chart_id)
        .expect("detached chart");
    assert!(!detached_chart.editor_open);
    assert!(detached_chart.editor_search_query.is_empty());
    assert_eq!(detached_chart.editor_selected_index, None);
    assert!(detached_chart.quick_order.is_none());
    assert!(!detached_chart.chart.quick_order_open);
    assert_eq!(detached_chart.chart.active_tool, None);
    assert!(
        !terminal
            .chart_quick_order_surface
            .contains_key(&detached_chart_id)
    );
    assert!(
        !terminal
            .chart_surface_active_tools
            .contains_key(&detached_surface)
    );

    let other_chart = terminal.charts.get(&other_chart_id).expect("other chart");
    assert!(other_chart.editor_open);
    assert_eq!(other_chart.editor_search_query, "et");
    assert_eq!(other_chart.editor_selected_index, Some(0));
    assert!(other_chart.quick_order.is_some());
    assert!(other_chart.chart.quick_order_open);
    assert_eq!(
        other_chart.chart.active_tool,
        Some(DrawingTool::HorizontalLevel)
    );
    assert_eq!(
        terminal
            .chart_quick_order_surface
            .get(&other_chart_id)
            .copied(),
        Some(other_surface)
    );
    assert_eq!(
        terminal
            .chart_surface_active_tools
            .get(&other_surface)
            .copied(),
        Some(DrawingTool::HorizontalLevel)
    );
}
