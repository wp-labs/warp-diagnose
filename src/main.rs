mod data;

use std::cell::RefCell;
use std::rc::Rc;

use slint::VecModel;

slint::include_modules!();

struct UiState {
    dashboard: data::DashboardData,
    selected_stage: Option<usize>,
    selected_point: Option<usize>,
    hover_point: Option<usize>,
    point_details: Vec<String>,
    point_previews: Vec<String>,
    point_hint: String,
}

fn apply_view(app: &AppWindow, state: &mut UiState) {
    let view = state.dashboard.build_view(state.selected_stage);

    app.set_total_events(view.report.total_rows as i32);
    app.set_info_events(view.report.info_rows as i32);
    app.set_warn_events(view.report.warn_rows as i32);
    app.set_error_events(view.report.error_rows as i32);

    let status_text = view.report.to_status_text();

    app.set_recent_events_text(view.report.recent_events_text.into());
    app.set_top_targets_text(view.report.top_targets_text.into());
    app.set_top_entities_text(view.report.top_entities_text.into());
    app.set_source_text(view.report.source_text.into());
    app.set_status_text(status_text.into());

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
    state.selected_point = None;
    state.hover_point = None;
    state.point_details.clear();
    state.point_previews.clear();
    state.point_hint.clear();
    apply_view(app, state);
}

fn main() -> Result<(), slint::PlatformError> {
    let app = AppWindow::new()?;

    let state = Rc::new(RefCell::new(UiState {
        dashboard: data::load_default_sources(),
        selected_stage: None,
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
