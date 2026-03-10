mod data;

use std::cell::RefCell;
use std::rc::Rc;

use slint::VecModel;

slint::include_modules!();

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
    selected_stage: Option<usize>,
    selected_level: Option<data::LevelFilter>,
    selected_risk: Option<data::RiskFilter>,
    selected_source: Option<data::SourceFilter>,
    selected_point: Option<usize>,
    hover_point: Option<usize>,
    point_details: Vec<String>,
    point_previews: Vec<String>,
    point_hint: String,
}

fn apply_view(app: &AppWindow, state: &mut UiState) {
    let view = state
        .dashboard
        .build_view(
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

    state.point_details = view.point_details;
    state.point_previews = view.point_previews;
    state.point_hint = view.point_hint_text;
    state.hover_point = None;
    app.set_hover_detail_text("Hover a point to preview.".into());

    let point_text = match state.selected_point {
        Some(idx) if idx < state.point_details.len() => state.point_details[idx].clone(),
        _ => {
            state.selected_point = None;
            state.point_hint.clone()
        }
    };

    app.set_point_detail_text(point_text.into());
}

fn reload_data(app: &AppWindow, state: &mut UiState) {
    state.dashboard = data::load_default_sources();
    state.selected_stage = None;
    state.selected_level = None;
    state.selected_risk = None;
    state.selected_source = None;
    state.selected_point = None;
    state.hover_point = None;
    state.point_details.clear();
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
        selected_stage: None,
        selected_level: None,
        selected_risk: None,
        selected_source: None,
        selected_point: None,
        hover_point: None,
        point_details: Vec::new(),
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
                st.selected_level = if st.selected_level == next { None } else { next };
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
                st.selected_source = if st.selected_source == next { None } else { next };
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
                st.selected_point = None;
                st.hover_point = None;
                apply_view(&app, &mut st);
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
                if idx < st.point_details.len() {
                    app.set_point_detail_text(st.point_details[idx].clone().into());
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
                app.set_point_detail_text(st.point_hint.clone().into());
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
