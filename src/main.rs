use std::cell::RefCell;
use std::rc::Rc;

use slint::VecModel;
use warp_diagnose::data;

slint::include_modules!();

const TABLE_WINDOW_CHROME_PX: usize = 308;
const TABLE_ROW_HEIGHT_PX: usize = 34;
const TABLE_MIN_PAGE_SIZE: usize = 20;

fn table_page_size(app: &AppWindow) -> usize {
    let window_height = app.window().size().height as usize;
    let computed = window_height
        .saturating_sub(TABLE_WINDOW_CHROME_PX)
        .max(TABLE_ROW_HEIGHT_PX)
        / TABLE_ROW_HEIGHT_PX;
    computed.max(TABLE_MIN_PAGE_SIZE)
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
        Some(data::RiskFilter::Low) => 1,
        Some(data::RiskFilter::Mid) => 2,
        Some(data::RiskFilter::High) => 3,
    }
}

fn risk_filter_from_ui_idx(idx: i32) -> Option<data::RiskFilter> {
    match idx {
        1 => Some(data::RiskFilter::Low),
        2 => Some(data::RiskFilter::Mid),
        3 => Some(data::RiskFilter::High),
        _ => None,
    }
}

fn risk_filter_label(filter: Option<data::RiskFilter>) -> String {
    match filter {
        None => "All risks".to_string(),
        Some(data::RiskFilter::Low) => "Risk: < 0.60".to_string(),
        Some(data::RiskFilter::Mid) => "Risk: 0.60-0.84".to_string(),
        Some(data::RiskFilter::High) => "Risk: >= 0.85".to_string(),
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

struct UiState {
    dashboard: data::DashboardData,
    active_page: i32,
    log_page: usize,
    alert_page: usize,
    selected_stage: Option<usize>,
    selected_level: Option<data::LevelFilter>,
    selected_risk: Option<data::RiskFilter>,
    selected_source: Option<data::SourceFilter>,
    selected_point: Option<usize>,
    hover_point: Option<usize>,
    point_detail_summaries: Vec<String>,
    point_detail_rows: Vec<Vec<data::DetailRowVm>>,
    point_previews: Vec<String>,
    point_hint: String,
}

fn apply_log_page(app: &AppWindow, state: &mut UiState) {
    let page_size = table_page_size(app);
    let page = state.dashboard.build_log_page(
        state.selected_stage,
        state.selected_level,
        state.selected_risk,
        state.selected_source,
        state.log_page,
        page_size,
    );
    state.log_page = page.page_idx;
    app.set_filtered_log_summary(page.summary.into());
    app.set_filtered_log_rows(
        Rc::new(VecModel::from(
            page.rows
                .iter()
                .map(|row| DetailRow {
                    row_no: row.row_no.clone().into(),
                    time: row.time.clone().into(),
                    level: row.level.clone().into(),
                    risk_score: row.risk_score.clone().into(),
                    rule: row.rule.clone().into(),
                    target: row.target.clone().into(),
                    entity: row.entity.clone().into(),
                    action: row.action.clone().into(),
                    status: row.status.clone().into(),
                    content: row.content.clone().into(),
                })
                .collect::<Vec<_>>(),
        ))
        .into(),
    );
    app.set_log_page_text(
        format!(
            "Page {}/{} · {} rows",
            page.page_idx + 1,
            page.total_pages,
            page.total_rows
        )
        .into(),
    );
    app.set_has_prev_log_page(page.page_idx > 0);
    app.set_has_next_log_page(page.page_idx + 1 < page.total_pages);
}

fn apply_alert_page(app: &AppWindow, state: &mut UiState) {
    let page_size = table_page_size(app);
    let page = state.dashboard.build_alert_page(
        state.selected_stage,
        state.selected_level,
        state.selected_risk,
        state.selected_source,
        state.alert_page,
        page_size,
    );
    state.alert_page = page.page_idx;
    app.set_filtered_alert_summary(page.summary.into());
    app.set_filtered_alert_rows(
        Rc::new(VecModel::from(
            page.rows
                .iter()
                .map(|row| DetailRow {
                    row_no: row.row_no.clone().into(),
                    time: row.time.clone().into(),
                    level: row.level.clone().into(),
                    risk_score: row.risk_score.clone().into(),
                    rule: row.rule.clone().into(),
                    target: row.target.clone().into(),
                    entity: row.entity.clone().into(),
                    action: row.action.clone().into(),
                    status: row.status.clone().into(),
                    content: row.content.clone().into(),
                })
                .collect::<Vec<_>>(),
        ))
        .into(),
    );
    app.set_alert_page_text(
        format!(
            "Page {}/{} · {} rows",
            page.page_idx + 1,
            page.total_pages,
            page.total_rows
        )
        .into(),
    );
    app.set_has_prev_alert_page(page.page_idx > 0);
    app.set_has_next_alert_page(page.page_idx + 1 < page.total_pages);
}

fn apply_view(app: &AppWindow, state: &mut UiState) {
    let view = state.dashboard.build_view(
        state.selected_stage,
        state.selected_level,
        state.selected_risk,
        state.selected_source,
    );

    app.set_total_events(view.report.total_rows as i32);
    app.set_risk_low_events(view.report.risk_low_rows as i32);
    app.set_risk_mid_events(view.report.risk_mid_rows as i32);
    app.set_risk_high_events(view.report.risk_high_rows as i32);

    let status_text = view.report.to_status_text();

    app.set_recent_events_text(view.report.recent_events_text.into());
    app.set_top_targets_text(view.report.top_targets_text.into());
    app.set_top_entities_text(view.report.top_entities_text.into());
    app.set_source_text(view.report.source_text.into());
    app.set_status_text(status_text.into());
    app.set_active_level_filter(level_filter_to_ui_idx(state.selected_level));
    app.set_active_risk_filter(risk_filter_to_ui_idx(state.selected_risk));
    app.set_active_source_filter(source_filter_to_ui_idx(state.selected_source));
    app.set_has_level_filter(state.selected_level.is_some());
    app.set_has_risk_filter(state.selected_risk.is_some());
    app.set_has_source_filter(state.selected_source.is_some());
    app.set_has_stage_filter(state.selected_stage.is_some());
    app.set_active_level_filter_text(level_filter_label(state.selected_level).into());
    app.set_active_risk_filter_text(risk_filter_label(state.selected_risk).into());
    app.set_active_source_filter_text(source_filter_label(state.selected_source).into());
    let active_stage = state
        .selected_stage
        .and_then(|idx| state.dashboard.stage_label(idx))
        .map(|label| format!("Stage: {label}"))
        .unwrap_or_else(|| "All stages".to_string());
    app.set_active_stage_filter_text(active_stage.into());

    app.set_stage_detail_text(view.stage_detail_text.into());
    app.set_lane_legend_text(view.lane_legend_text.into());

    let stage_rows: Vec<StageBand> = view
        .stage_bands
        .iter()
        .map(|s| StageBand {
            label: s.label.clone().into(),
            summary: s.summary.clone().into(),
            start_pct: s.start_pct,
            end_pct: s.end_pct,
            selected: s.selected,
        })
        .collect();

    let point_rows: Vec<TimelinePoint> = view
        .timeline_points
        .iter()
        .map(|p| TimelinePoint {
            x_pct: p.x_pct,
            y_pct: p.y_pct,
            risk: p.risk,
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

    let card_rows: Vec<StageCard> = view
        .stage_cards
        .iter()
        .map(|c| StageCard {
            idx: c.idx as i32,
            label: c.label.clone().into(),
            action: c.action.clone().into(),
            summary: c.summary.clone().into(),
            selected: c.selected,
        })
        .collect();

    app.set_stage_bands(Rc::new(VecModel::from(stage_rows)).into());
    app.set_timeline_points(Rc::new(VecModel::from(point_rows)).into());
    app.set_time_ticks(Rc::new(VecModel::from(tick_rows)).into());
    app.set_timeline_content_px(view.timeline_content_px);
    app.set_lane_labels(Rc::new(VecModel::from(lane_rows)).into());
    app.set_stage_cards(Rc::new(VecModel::from(card_rows)).into());
    app.set_active_page(state.active_page);
    if state.active_page == 1 {
        apply_log_page(app, state);
    } else if state.active_page == 2 {
        apply_alert_page(app, state);
    }

    state.point_detail_summaries = view.point_detail_summaries;
    state.point_detail_rows = view.point_detail_rows;
    state.point_previews = view.point_previews;
    state.point_hint = view.point_hint_text;
    state.hover_point = None;
    app.set_hover_detail_text("Hover a point to preview.".into());

    let point_summary = match state.selected_point {
        Some(idx) if idx < state.point_detail_summaries.len() => {
            state.point_detail_summaries[idx].clone()
        }
        _ => {
            state.selected_point = None;
            state.point_hint.clone()
        }
    };

    let point_rows = match state.selected_point {
        Some(idx) if idx < state.point_detail_rows.len() => state.point_detail_rows[idx]
            .iter()
            .map(|row| DetailRow {
                row_no: row.row_no.clone().into(),
                time: row.time.clone().into(),
                level: row.level.clone().into(),
                risk_score: row.risk_score.clone().into(),
                rule: row.rule.clone().into(),
                target: row.target.clone().into(),
                entity: row.entity.clone().into(),
                action: row.action.clone().into(),
                status: row.status.clone().into(),
                content: row.content.clone().into(),
            })
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };

    app.set_point_detail_summary(point_summary.into());
    app.set_point_detail_rows(Rc::new(VecModel::from(point_rows)).into());
}

fn reload_data(app: &AppWindow, state: &mut UiState) {
    state.dashboard = data::load_default_sources();
    state.log_page = 0;
    state.alert_page = 0;
    state.selected_stage = None;
    state.selected_level = None;
    state.selected_risk = None;
    state.selected_source = None;
    state.selected_point = None;
    state.hover_point = None;
    state.point_detail_summaries.clear();
    state.point_detail_rows.clear();
    state.point_previews.clear();
    state.point_hint.clear();
    apply_view(app, state);
}

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;
    app.window()
        .set_size(slint::LogicalSize::new(1420.0, 960.0));

    let state = Rc::new(RefCell::new(UiState {
        dashboard: data::load_default_sources(),
        active_page: 0,
        log_page: 0,
        alert_page: 0,
        selected_stage: None,
        selected_level: None,
        selected_risk: None,
        selected_source: None,
        selected_point: None,
        hover_point: None,
        point_detail_summaries: Vec::new(),
        point_detail_rows: Vec::new(),
        point_previews: Vec::new(),
        point_hint: String::new(),
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
                st.selected_level = if st.selected_level == next {
                    None
                } else {
                    next
                };
                st.log_page = 0;
                st.alert_page = 0;
                st.selected_point = None;
                st.hover_point = None;
                apply_view(&app, &mut st);
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
                st.selected_risk = if st.selected_risk == next { None } else { next };
                st.log_page = 0;
                st.alert_page = 0;
                st.selected_point = None;
                st.hover_point = None;
                apply_view(&app, &mut st);
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
                st.selected_source = if st.selected_source == next {
                    None
                } else {
                    next
                };
                st.log_page = 0;
                st.alert_page = 0;
                st.selected_point = None;
                st.hover_point = None;
                apply_view(&app, &mut st);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_clear_level_filter(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                st.selected_level = None;
                st.log_page = 0;
                st.alert_page = 0;
                st.selected_point = None;
                st.hover_point = None;
                apply_view(&app, &mut st);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_clear_risk_filter(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                st.selected_risk = None;
                st.log_page = 0;
                st.alert_page = 0;
                st.selected_point = None;
                st.hover_point = None;
                apply_view(&app, &mut st);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_clear_source_filter(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                st.selected_source = None;
                st.log_page = 0;
                st.alert_page = 0;
                st.selected_point = None;
                st.hover_point = None;
                apply_view(&app, &mut st);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_clear_all_filters(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                st.selected_stage = None;
                st.selected_level = None;
                st.selected_risk = None;
                st.selected_source = None;
                st.log_page = 0;
                st.alert_page = 0;
                st.selected_point = None;
                st.hover_point = None;
                apply_view(&app, &mut st);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_stage_clicked(move |idx| {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                let idx = idx.max(0) as usize;
                if st.selected_stage == Some(idx) {
                    st.selected_stage = None;
                } else {
                    st.selected_stage = Some(idx);
                }
                st.log_page = 0;
                st.alert_page = 0;
                st.selected_point = None;
                st.hover_point = None;
                apply_view(&app, &mut st);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_clear_stage_filter(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                st.selected_stage = None;
                st.log_page = 0;
                st.alert_page = 0;
                st.selected_point = None;
                st.hover_point = None;
                apply_view(&app, &mut st);
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
                app.set_active_page(st.active_page);
                if st.active_page == 1 {
                    apply_log_page(&app, &mut st);
                } else if st.active_page == 2 {
                    apply_alert_page(&app, &mut st);
                }
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_log_prev_page(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                if st.log_page > 0 {
                    st.log_page -= 1;
                }
                apply_log_page(&app, &mut st);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_log_next_page(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                st.log_page += 1;
                apply_log_page(&app, &mut st);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_alert_prev_page(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                if st.alert_page > 0 {
                    st.alert_page -= 1;
                }
                apply_alert_page(&app, &mut st);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_alert_next_page(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                st.alert_page += 1;
                apply_alert_page(&app, &mut st);
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_window_resized(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                if st.active_page == 1 {
                    apply_log_page(&app, &mut st);
                } else if st.active_page == 2 {
                    apply_alert_page(&app, &mut st);
                }
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
                st.selected_point = Some(idx);
                if idx < st.point_detail_summaries.len() {
                    let rows = st
                        .point_detail_rows
                        .get(idx)
                        .map(|rows| {
                            rows.iter()
                                .map(|row| DetailRow {
                                    row_no: row.row_no.clone().into(),
                                    time: row.time.clone().into(),
                                    level: row.level.clone().into(),
                                    risk_score: row.risk_score.clone().into(),
                                    rule: row.rule.clone().into(),
                                    target: row.target.clone().into(),
                                    entity: row.entity.clone().into(),
                                    action: row.action.clone().into(),
                                    status: row.status.clone().into(),
                                    content: row.content.clone().into(),
                                })
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    app.set_point_detail_summary(st.point_detail_summaries[idx].clone().into());
                    app.set_point_detail_rows(Rc::new(VecModel::from(rows)).into());
                }
            }
        });
    }

    {
        let weak = app.as_weak();
        let state = Rc::clone(&state);
        app.on_clear_point_selection(move || {
            if let Some(app) = weak.upgrade() {
                let mut st = state.borrow_mut();
                st.selected_point = None;
                app.set_point_detail_summary(st.point_hint.clone().into());
                app.set_point_detail_rows(Rc::new(VecModel::from(Vec::<DetailRow>::new())).into());
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
                st.hover_point = Some(idx);
                if idx < st.point_previews.len() {
                    app.set_hover_detail_text(st.point_previews[idx].clone().into());
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
                st.hover_point = None;
                app.set_hover_detail_text("Hover a point to preview.".into());
            }
        });
    }

    app.run()
}
