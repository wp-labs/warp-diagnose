use std::cell::RefCell;
use std::rc::Rc;

use slint::VecModel;
use warp_diagnose::config::runtime_config;
use warp_diagnose::data;

slint::include_modules!();

fn fallback_table_page_size(app: &AppWindow) -> usize {
    let table = &runtime_config().table;
    let window_height = app.get_logical_window_height() as usize;
    let computed = window_height
        .saturating_sub(table.window_chrome_px)
        .max(table.row_height_px)
        / table.row_height_px;
    computed.max(table.min_page_size)
}

fn table_page_size_from_visible_height(app: &AppWindow, visible_height: f32) -> usize {
    let table = &runtime_config().table;
    let fallback = fallback_table_page_size(app);
    if visible_height <= 0.0 {
        return fallback;
    }

    ((visible_height as usize) / table.row_height_px)
        .max(table.min_page_size)
        .min(fallback.max(table.min_page_size))
}

fn level_filter_to_ui_idx(filter: Option<data::LevelFilter>) -> i32 {
    match filter {
        None => 0,
        Some(data::LevelFilter::Info) => 1,
        Some(data::LevelFilter::Warn) => 2,
        Some(data::LevelFilter::Error) => 3,
    }
}

fn level_filter_from_ui_idx(idx: i32) -> Option<data::LevelFilter> {
    match idx {
        1 => Some(data::LevelFilter::Info),
        2 => Some(data::LevelFilter::Warn),
        3 => Some(data::LevelFilter::Error),
        _ => None,
    }
}

fn level_filter_label(filter: Option<data::LevelFilter>) -> String {
    match filter {
        None => "All levels".to_string(),
        Some(data::LevelFilter::Info) => "Level: INFO".to_string(),
        Some(data::LevelFilter::Warn) => "Level: WARN".to_string(),
        Some(data::LevelFilter::Error) => "Level: ERROR+".to_string(),
    }
}

fn risk_filter_to_ui_idx(filter: Option<data::RiskFilter>) -> i32 {
    match filter {
        None => 0,
        Some(data::RiskFilter::L1) => 1,
        Some(data::RiskFilter::L2) => 2,
        Some(data::RiskFilter::L3) => 3,
        Some(data::RiskFilter::L4) => 4,
        Some(data::RiskFilter::L5) => 5,
        Some(data::RiskFilter::L6) => 6,
        Some(data::RiskFilter::L7) => 7,
        Some(data::RiskFilter::L8) => 8,
        Some(data::RiskFilter::L9) => 9,
        Some(data::RiskFilter::L10) => 10,
    }
}

fn risk_filter_from_ui_idx(idx: i32) -> Option<data::RiskFilter> {
    match idx {
        1 => Some(data::RiskFilter::L1),
        2 => Some(data::RiskFilter::L2),
        3 => Some(data::RiskFilter::L3),
        4 => Some(data::RiskFilter::L4),
        5 => Some(data::RiskFilter::L5),
        6 => Some(data::RiskFilter::L6),
        7 => Some(data::RiskFilter::L7),
        8 => Some(data::RiskFilter::L8),
        9 => Some(data::RiskFilter::L9),
        10 => Some(data::RiskFilter::L10),
        _ => None,
    }
}

fn risk_filter_label(filter: Option<data::RiskFilter>) -> String {
    match filter {
        None => "All risks".to_string(),
        Some(data::RiskFilter::L1) => "Risk: L1 (0-9)".to_string(),
        Some(data::RiskFilter::L2) => "Risk: L2 (10-19)".to_string(),
        Some(data::RiskFilter::L3) => "Risk: L3 (20-29)".to_string(),
        Some(data::RiskFilter::L4) => "Risk: L4 (30-39)".to_string(),
        Some(data::RiskFilter::L5) => "Risk: L5 (40-49)".to_string(),
        Some(data::RiskFilter::L6) => "Risk: L6 (50-59)".to_string(),
        Some(data::RiskFilter::L7) => "Risk: L7 (60-69)".to_string(),
        Some(data::RiskFilter::L8) => "Risk: L8 (70-79)".to_string(),
        Some(data::RiskFilter::L9) => "Risk: L9 (80-89)".to_string(),
        Some(data::RiskFilter::L10) => "Risk: L10 (90-100)".to_string(),
    }
}

fn source_filter_to_ui_idx(filter: Option<data::SourceFilter>) -> i32 {
    match filter {
        None => 0,
        Some(data::SourceFilter::Demo) => 1,
        Some(data::SourceFilter::Wparse) => 2,
        Some(data::SourceFilter::Wfusion) => 3,
    }
}

fn source_filter_from_ui_idx(idx: i32) -> Option<data::SourceFilter> {
    match idx {
        1 => Some(data::SourceFilter::Demo),
        2 => Some(data::SourceFilter::Wparse),
        3 => Some(data::SourceFilter::Wfusion),
        _ => None,
    }
}

fn source_filter_label(filter: Option<data::SourceFilter>) -> String {
    match filter {
        None => "All sources".to_string(),
        Some(data::SourceFilter::Demo) => "Source: demo".to_string(),
        Some(data::SourceFilter::Wparse) => "Source: wparse".to_string(),
        Some(data::SourceFilter::Wfusion) => "Source: wfusion".to_string(),
    }
}

#[derive(Default)]
struct GlobalFilterState {
    selected_level: Option<data::LevelFilter>,
    selected_risk: Option<data::RiskFilter>,
    selected_source: Option<data::SourceFilter>,
}

#[derive(Default)]
struct OverviewState {
    selected_point: Option<usize>,
    hover_point: Option<usize>,
    point_detail_summaries: Vec<String>,
    point_detail_rows: Vec<Vec<data::DetailRowVm>>,
    point_previews: Vec<String>,
    point_hint: String,
}

#[derive(Default)]
struct LogPageState {
    page_idx: usize,
    page_size: usize,
}

#[derive(Default)]
struct AlertPageState {
    page_idx: usize,
    page_size: usize,
}

struct UiState {
    dashboard: data::DashboardData,
    active_page: i32,
    filters: GlobalFilterState,
    overview: OverviewState,
    log_page: LogPageState,
    alert_page: AlertPageState,
}

impl UiState {
    fn reset_table_pages(&mut self) {
        self.log_page.page_idx = 0;
        self.alert_page.page_idx = 0;
    }

    fn clear_overview_selection(&mut self) {
        self.overview.selected_point = None;
        self.overview.hover_point = None;
    }

    fn clear_all_filters(&mut self) {
        self.filters = GlobalFilterState::default();
        self.reset_table_pages();
        self.clear_overview_selection();
    }
}

fn map_detail_row(row: &data::DetailRowVm) -> DetailRow {
    DetailRow {
        row_no: row.row_no.clone().into(),
        time: row.time.clone().into(),
        level: row.level.clone().into(),
        risk_tier: row.risk_tier,
        event_count: row.event_count.clone().into(),
        risk_score: row.risk_score.clone().into(),
        rule: row.rule.clone().into(),
        target: row.target.clone().into(),
        entity: row.entity.clone().into(),
        action: row.action.clone().into(),
        status: row.status.clone().into(),
        content: row.content.clone().into(),
    }
}

fn map_detail_rows(rows: &[data::DetailRowVm]) -> Vec<DetailRow> {
    rows.iter().map(map_detail_row).collect()
}

fn schedule_table_reset(app: &AppWindow, reset_log: bool, reset_alert: bool) {
    let weak = app.as_weak();
    let _ = slint::invoke_from_event_loop(move || {
        if let Some(app) = weak.upgrade() {
            if reset_log {
                app.set_log_table_reset_token(app.get_log_table_reset_token() + 1);
            }
            if reset_alert {
                app.set_alert_table_reset_token(app.get_alert_table_reset_token() + 1);
            }
        }
    });
}

fn apply_global_shell(app: &AppWindow, state: &UiState, report: &data::LoadReport) {
    app.set_total_events(report.total_rows as i32);
    app.set_risk_low_events(report.risk_low_rows as i32);
    app.set_risk_mid_events(report.risk_mid_rows as i32);
    app.set_risk_high_events(report.risk_high_rows as i32);
    app.set_top_targets_text(report.top_targets_text.clone().into());
    app.set_source_text(report.source_text.clone().into());
    app.set_status_text(report.to_status_text().into());
    app.set_active_level_filter(level_filter_to_ui_idx(state.filters.selected_level));
    app.set_active_risk_filter(risk_filter_to_ui_idx(state.filters.selected_risk));
    app.set_active_source_filter(source_filter_to_ui_idx(state.filters.selected_source));
    app.set_has_level_filter(state.filters.selected_level.is_some());
    app.set_has_risk_filter(state.filters.selected_risk.is_some());
    app.set_has_source_filter(state.filters.selected_source.is_some());
    app.set_active_level_filter_text(level_filter_label(state.filters.selected_level).into());
    app.set_active_risk_filter_text(risk_filter_label(state.filters.selected_risk).into());
    app.set_active_source_filter_text(source_filter_label(state.filters.selected_source).into());
    app.set_active_page(state.active_page);
}

fn apply_overview_page(app: &AppWindow, state: &mut UiState, view: data::DashboardView) {
    let point_rows: Vec<TimelinePoint> = view
        .timeline_points
        .iter()
        .map(|p| TimelinePoint {
            x_pct: p.x_pct,
            y_pct: p.y_pct,
            risk: p.risk,
            risk_tier: p.risk_tier,
            size_norm: p.size_norm,
            entity: p.entity.clone().into(),
        })
        .collect();

    let tick_rows: Vec<TimeTick> = view
        .time_ticks
        .iter()
        .map(|t| TimeTick {
            x_pct: t.x_pct,
            label: t.label.clone().into(),
        })
        .collect();

    let lane_rows: Vec<LaneLabel> = view
        .lane_labels
        .iter()
        .map(|l| LaneLabel {
            y_pct: l.y_pct,
            label: l.label.clone().into(),
        })
        .collect();

    app.set_lane_legend_text(view.lane_legend_text.into());
    app.set_timeline_points(Rc::new(VecModel::from(point_rows)).into());
    app.set_time_ticks(Rc::new(VecModel::from(tick_rows)).into());
    app.set_first_event_x_pct(view.first_event_x_pct);
    app.set_last_event_x_pct(view.last_event_x_pct);
    app.set_timeline_content_px(view.timeline_content_px);
    app.set_lane_labels(Rc::new(VecModel::from(lane_rows)).into());
    app.set_hovered_timeline_point_index(-1);
    app.set_selected_timeline_point_index(
        state
            .overview
            .selected_point
            .map(|idx| idx as i32)
            .unwrap_or(-1),
    );

    state.overview.point_detail_summaries = view.point_detail_summaries;
    state.overview.point_detail_rows = view.point_detail_rows;
    state.overview.point_previews = view.point_previews;
    state.overview.point_hint = view.point_hint_text;
    state.overview.hover_point = None;
    app.set_hover_detail_text("Hover a point to preview.".into());

    let point_summary = match state.overview.selected_point {
        Some(idx) if idx < state.overview.point_detail_summaries.len() => {
            state.overview.point_detail_summaries[idx].clone()
        }
        _ => {
            state.overview.selected_point = None;
            app.set_selected_timeline_point_index(-1);
            state.overview.point_hint.clone()
        }
    };

    let point_rows = match state.overview.selected_point {
        Some(idx) if idx < state.overview.point_detail_rows.len() => {
            map_detail_rows(&state.overview.point_detail_rows[idx])
        }
        _ => Vec::new(),
    };

    app.set_point_detail_summary(point_summary.into());
    app.set_point_detail_rows(Rc::new(VecModel::from(point_rows)).into());
}

fn apply_log_page(app: &AppWindow, state: &mut UiState) {
    let visible_height = app.get_log_table_body_viewport_height();
    let page_size = table_page_size_from_visible_height(app, visible_height);
    if state.log_page.page_size != page_size {
        let first_row_idx = state.log_page.page_idx * state.log_page.page_size.max(1);
        state.log_page.page_idx = first_row_idx / page_size;
        state.log_page.page_size = page_size;
    }
    let page = state.dashboard.build_log_page(
        state.filters.selected_level,
        state.filters.selected_risk,
        state.filters.selected_source,
        state.log_page.page_idx,
        page_size,
    );
    state.log_page.page_idx = page.page_idx;
    app.set_filtered_log_summary(page.summary.into());
    app.set_filtered_log_rows(
        Rc::new(VecModel::from(map_detail_rows(&page.rows))).into(),
    );
    app.set_log_page_text(
        format!(
            "Page {}/{} · {} rows",
            page.page_idx + 1,
            page.total_pages,
            page.total_rows,
        )
        .into(),
    );
    app.set_has_prev_log_page(page.page_idx > 0);
    app.set_has_next_log_page(page.page_idx + 1 < page.total_pages);
}

fn apply_alert_page(app: &AppWindow, state: &mut UiState) {
    let visible_height = app.get_alert_table_body_viewport_height();
    let page_size = table_page_size_from_visible_height(app, visible_height);
    if state.alert_page.page_size != page_size {
        let first_row_idx = state.alert_page.page_idx * state.alert_page.page_size.max(1);
        state.alert_page.page_idx = first_row_idx / page_size;
        state.alert_page.page_size = page_size;
    }
    let page = state.dashboard.build_alert_page(
        state.filters.selected_level,
        state.filters.selected_risk,
        state.filters.selected_source,
        state.alert_page.page_idx,
        page_size,
    );
    state.alert_page.page_idx = page.page_idx;
    app.set_filtered_alert_summary(page.summary.into());
    app.set_filtered_alert_rows(
        Rc::new(VecModel::from(map_detail_rows(&page.rows))).into(),
    );
    app.set_alert_page_text(
        format!(
            "Page {}/{} · {} rows",
            page.page_idx + 1,
            page.total_pages,
            page.total_rows,
        )
        .into(),
    );
    app.set_has_prev_alert_page(page.page_idx > 0);
    app.set_has_next_alert_page(page.page_idx + 1 < page.total_pages);
}

fn apply_current_page(app: &AppWindow, state: &mut UiState) {
    if state.active_page == 1 {
        apply_log_page(app, state);
    } else if state.active_page == 2 {
        apply_alert_page(app, state);
    }
}

fn apply_view(app: &AppWindow, state: &mut UiState) {
    let view = state.dashboard.build_view(
        state.filters.selected_level,
        state.filters.selected_risk,
        state.filters.selected_source,
    );
    apply_global_shell(app, state, &view.report);
    apply_overview_page(app, state, view);
    apply_current_page(app, state);
}

fn reload_data(app: &AppWindow, state: &mut UiState) {
    state.dashboard = data::load_default_sources();
    state.filters = GlobalFilterState::default();
    state.overview = OverviewState::default();
    state.log_page = LogPageState::default();
    state.alert_page = AlertPageState::default();
    apply_view(app, state);
    schedule_table_reset(app, true, true);
}

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    let window = &runtime_config().window;
    app.window()
        .set_size(slint::LogicalSize::new(window.width, window.height));

    let state = Rc::new(RefCell::new(UiState {
        dashboard: data::load_default_sources(),
        active_page: 0,
        filters: GlobalFilterState::default(),
        overview: OverviewState::default(),
        log_page: LogPageState::default(),
        alert_page: AlertPageState::default(),
    }));

    {
        let mut st = state.borrow_mut();
        apply_view(&app, &mut st);
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_reload_data(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                reload_data(&app, &mut st);
                eprintln!("[warp-diagnose] data reloaded");
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_level_filter_clicked(move |idx| {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                let next = level_filter_from_ui_idx(idx);
                st.filters.selected_level = if st.filters.selected_level == next {
                    None
                } else {
                    next
                };
                st.reset_table_pages();
                st.clear_overview_selection();
                apply_view(&app, &mut st);
                schedule_table_reset(&app, true, true);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_risk_filter_clicked(move |idx| {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                let next = risk_filter_from_ui_idx(idx);
                st.filters.selected_risk = if st.filters.selected_risk == next {
                    None
                } else {
                    next
                };
                st.reset_table_pages();
                st.clear_overview_selection();
                apply_view(&app, &mut st);
                schedule_table_reset(&app, true, true);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_source_filter_clicked(move |idx| {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                let next = source_filter_from_ui_idx(idx);
                st.filters.selected_source = if st.filters.selected_source == next {
                    None
                } else {
                    next
                };
                st.reset_table_pages();
                st.clear_overview_selection();
                apply_view(&app, &mut st);
                schedule_table_reset(&app, true, true);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_clear_level_filter(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                st.filters.selected_level = None;
                st.reset_table_pages();
                st.clear_overview_selection();
                apply_view(&app, &mut st);
                schedule_table_reset(&app, true, true);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_clear_risk_filter(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                st.filters.selected_risk = None;
                st.reset_table_pages();
                st.clear_overview_selection();
                apply_view(&app, &mut st);
                schedule_table_reset(&app, true, true);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_clear_source_filter(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                st.filters.selected_source = None;
                st.reset_table_pages();
                st.clear_overview_selection();
                apply_view(&app, &mut st);
                schedule_table_reset(&app, true, true);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_clear_all_filters(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                st.clear_all_filters();
                apply_view(&app, &mut st);
                schedule_table_reset(&app, true, true);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_page_changed(move |page| {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                st.active_page = page.clamp(0, 2);
                if st.active_page == 1 {
                    st.log_page.page_idx = 0;
                } else if st.active_page == 2 {
                    st.alert_page.page_idx = 0;
                }
                app.set_active_page(st.active_page);
                apply_current_page(&app, &mut st);
                schedule_table_reset(&app, st.active_page == 1, st.active_page == 2);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_log_prev_page(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                if st.log_page.page_idx > 0 {
                    st.log_page.page_idx -= 1;
                }
                apply_log_page(&app, &mut st);
                schedule_table_reset(&app, true, false);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_log_next_page(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                st.log_page.page_idx += 1;
                apply_log_page(&app, &mut st);
                schedule_table_reset(&app, true, false);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_alert_prev_page(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                if st.alert_page.page_idx > 0 {
                    st.alert_page.page_idx -= 1;
                }
                apply_alert_page(&app, &mut st);
                schedule_table_reset(&app, false, true);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_alert_next_page(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                st.alert_page.page_idx += 1;
                apply_alert_page(&app, &mut st);
                schedule_table_reset(&app, false, true);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_window_resized(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                apply_current_page(&app, &mut st);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_point_clicked(move |idx| {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                let idx = idx.max(0) as usize;
                st.overview.selected_point = Some(idx);
                app.set_selected_timeline_point_index(idx as i32);
                if idx < st.overview.point_detail_summaries.len() {
                    let rows = st
                        .overview
                        .point_detail_rows
                        .get(idx)
                        .map(|rows| map_detail_rows(rows))
                        .unwrap_or_default();
                    app.set_point_detail_summary(
                        st.overview.point_detail_summaries[idx].clone().into(),
                    );
                    app.set_point_detail_rows(Rc::new(VecModel::from(rows)).into());
                }
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_point_hovered(move |idx| {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                let idx = idx.max(0) as usize;
                st.overview.hover_point = Some(idx);
                app.set_hovered_timeline_point_index(idx as i32);
                if idx < st.overview.point_previews.len() {
                    app.set_hover_detail_text(st.overview.point_previews[idx].clone().into());
                }
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_point_unhovered(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                st.overview.hover_point = None;
                app.set_hovered_timeline_point_index(-1);
                app.set_hover_detail_text("Hover a point to preview.".into());
            }
        });
    }

    app.run()
}
