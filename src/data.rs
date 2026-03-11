use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use arrow::array::{
    Array, Float32Array, Float64Array, Int32Array, Int64Array, StringArray,
    TimestampNanosecondArray, UInt32Array, UInt64Array,
};
use arrow::record_batch::RecordBatch;
use chrono::{NaiveDateTime, TimeZone, Utc};
use regex::Regex;
use serde_json::Value;

use crate::arrow_frame::read_arrow_frames;

const LOCAL_CASE_LOG_WFU_ARROW: &str = "case/wparse/data/out_dat/log_wfu.arrow";
const LOCAL_CASE_WP_LOG_ARROW: &str = "case/wparse/data/out_dat/wp-log.arrow";
const LOCAL_CASE_WF_ALERT_ARROW: &str = "case/wfusion/alerts/wf-alert.arrow";
const LOCAL_CASE_WF_ALERT_DIR: &str = "case/wfusion/alerts";
const LOCAL_CASE_DEMO_JSON: &str = "case/wparse/data/out_dat/demo.json";
const LOCAL_CASE_DEMO_JSON_LEGACY: &str = "case/wparse/data/demo.json";
const LOCAL_CASE_WPARSE_LOG: &str = "case/target_data/raw_log.dat";
const LOCAL_CASE_WFUSION_ALERTS: &str = "case/wfusion/alerts/wf-alert.jsonl";
const LOCAL_CASE_WFUSION_ALERTS_LEGACY: &str = "case/wparse/alerts/all.jsonl";

const ENV_LOG_WFU: &str = "WARP_DIAGNOSE_LOG_WFU";
const ENV_DEMO_JSON: &str = "WARP_DIAGNOSE_DEMO_JSON";
const ENV_WPARSE_LOG: &str = "WARP_DIAGNOSE_WPARSE_LOG";
const ENV_USE_WFUSION: &str = "WARP_DIAGNOSE_USE_WFUSION";
const ENV_ALERT_WFU_DIR: &str = "WARP_DIAGNOSE_ALERT_WFU_DIR";
const ENV_WFUSION_ALERTS: &str = "WARP_DIAGNOSE_WFUSION_ALERTS";

const DEFAULT_BUCKETS: usize = 72;
const MAX_LANES: usize = 9;
const MIN_TIMELINE_WIDTH_PX: usize = 3600;
const MAX_TIMELINE_WIDTH_PX: usize = 24_000;
const TIMELINE_PX_PER_SEC: usize = 14;
const SECOND_NS: i128 = 1_000_000_000;
const TIMELINE_VERTICAL_PADDING_PCT: f32 = 0.08;

fn manifest_path(relative: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(relative)
        .to_string_lossy()
        .to_string()
}

fn default_demo_json_path() -> String {
    let computed = Path::new(env!("CARGO_MANIFEST_DIR")).join(LOCAL_CASE_LOG_WFU_ARROW);
    if computed.exists() {
        return computed.to_string_lossy().to_string();
    }
    let primary = Path::new(env!("CARGO_MANIFEST_DIR")).join(LOCAL_CASE_WP_LOG_ARROW);
    if primary.exists() {
        primary.to_string_lossy().to_string()
    } else {
        let fallback = Path::new(env!("CARGO_MANIFEST_DIR")).join(LOCAL_CASE_DEMO_JSON);
        if fallback.exists() {
            fallback.to_string_lossy().to_string()
        } else {
            manifest_path(LOCAL_CASE_DEMO_JSON_LEGACY)
        }
    }
}

fn default_wparse_log_path() -> String {
    manifest_path(LOCAL_CASE_WPARSE_LOG)
}

fn default_wfusion_alerts_path() -> String {
    let alert_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join(LOCAL_CASE_WF_ALERT_DIR);
    if alert_dir.exists() {
        return alert_dir.to_string_lossy().to_string();
    }
    let primary = Path::new(env!("CARGO_MANIFEST_DIR")).join(LOCAL_CASE_WF_ALERT_ARROW);
    if primary.exists() {
        primary.to_string_lossy().to_string()
    } else {
        let fallback = Path::new(env!("CARGO_MANIFEST_DIR")).join(LOCAL_CASE_WFUSION_ALERTS);
        if fallback.exists() {
            fallback.to_string_lossy().to_string()
        } else {
            manifest_path(LOCAL_CASE_WFUSION_ALERTS_LEGACY)
        }
    }
}

fn is_arrow_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("arrow"))
        .unwrap_or(false)
}

#[derive(Debug, Clone)]
struct EventRecord {
    seq: usize,
    source: String,
    time_text: String,
    epoch_ns: i128,
    window_bucket_ns: Option<i128>,
    level: String,
    rule: String,
    target: String,
    action: String,
    status: String,
    content: String,
    entity: String,
    risk: f32,
    stage_idx: usize,
    stage_boundary_prob: f32,
}

#[derive(Debug, Clone)]
struct StageSegment {
    idx: usize,
    label: String,
    family: String,
    top_action: String,
    start_ns: i128,
    end_ns: i128,
    start_ts: String,
    end_ts: String,
    duration_ms: i64,
    event_count: usize,
    incident_count: usize,
    confidence: f32,
}

#[derive(Debug, Default, Clone)]
pub struct LoadReport {
    pub compute_backend: String,
    pub demo_path: String,
    pub wparse_path: String,
    pub wfusion_alerts_path: String,
    pub demo_rows: usize,
    pub wparse_rows: usize,
    pub wfusion_rows: usize,
    pub total_rows: usize,
    pub risk_low_rows: usize,
    pub risk_mid_rows: usize,
    pub risk_high_rows: usize,
    pub unique_targets: usize,
    pub unique_entities: usize,
    pub first_ts: Option<String>,
    pub last_ts: Option<String>,
    pub stage_count: usize,
    pub demo_skipped: usize,
    pub wparse_skipped: usize,
    pub wfusion_skipped: usize,
    pub wfusion_enabled: bool,
    pub wfusion_used: bool,
    pub recent_events_text: String,
    pub top_targets_text: String,
    pub top_entities_text: String,
    pub source_text: String,
    pub errors: Vec<String>,
}

impl LoadReport {
    pub fn to_status_text(&self) -> String {
        let mut lines = vec![
            "Status: stage+entity timeline ready".to_string(),
            format!(
                "backend={} | wfusion_enabled={} | wfusion_used={}",
                self.compute_backend, self.wfusion_enabled, self.wfusion_used
            ),
            format!(
                "total={} | risk<0.60={} | 0.60-0.84={} | >=0.85={}",
                self.total_rows, self.risk_low_rows, self.risk_mid_rows, self.risk_high_rows
            ),
            format!(
                "targets={} | entities={} | stages={}",
                self.unique_targets, self.unique_entities, self.stage_count
            ),
            format!(
                "wfusion_rows={} (skip={})",
                self.wfusion_rows, self.wfusion_skipped
            ),
            format!(
                "demo_rows={} (skip={}) | wparse_rows={} (skip={})",
                self.demo_rows, self.demo_skipped, self.wparse_rows, self.wparse_skipped
            ),
        ];

        if let Some(first) = &self.first_ts {
            lines.push(format!("first_ts={first}"));
        }
        if let Some(last) = &self.last_ts {
            lines.push(format!("last_ts={last}"));
        }

        if self.errors.is_empty() {
            lines.push("source_errors=0".to_string());
        } else {
            lines.push(format!("source_errors={}", self.errors.len()));
            for err in &self.errors {
                lines.push(format!("- {err}"));
            }
        }

        lines.join("\n")
    }
}

#[derive(Debug, Clone)]
pub struct StageBandVm {
    pub label: String,
    pub summary: String,
    pub start_pct: f32,
    pub end_pct: f32,
    pub selected: bool,
}

#[derive(Debug, Clone)]
pub struct TimelinePointVm {
    pub x_pct: f32,
    pub y_pct: f32,
    pub risk: f32,
    pub size_norm: f32,
    pub entity: String,
}

#[derive(Debug, Clone)]
pub struct AxisTickVm {
    pub x_pct: f32,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct LaneLabelVm {
    pub y_pct: f32,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct StageCardVm {
    pub idx: usize,
    pub label: String,
    pub action: String,
    pub summary: String,
    pub selected: bool,
}

#[derive(Debug, Clone)]
pub struct DetailRowVm {
    pub row_no: String,
    pub time: String,
    pub level: String,
    pub risk_score: String,
    pub rule: String,
    pub target: String,
    pub entity: String,
    pub action: String,
    pub status: String,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelFilter {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskFilter {
    Low,
    Mid,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFilter {
    Demo,
    Wparse,
    Wfusion,
}

#[derive(Debug, Clone)]
pub struct DashboardView {
    pub report: LoadReport,
    pub stage_bands: Vec<StageBandVm>,
    pub timeline_points: Vec<TimelinePointVm>,
    pub time_ticks: Vec<AxisTickVm>,
    pub timeline_content_px: i32,
    pub lane_labels: Vec<LaneLabelVm>,
    pub stage_cards: Vec<StageCardVm>,
    pub point_detail_summaries: Vec<String>,
    pub point_detail_rows: Vec<Vec<DetailRowVm>>,
    pub point_previews: Vec<String>,
    pub stage_detail_text: String,
    pub point_hint_text: String,
    pub lane_legend_text: String,
}

#[derive(Debug, Clone)]
pub struct TablePageView {
    pub summary: String,
    pub rows: Vec<DetailRowVm>,
    pub page_idx: usize,
    pub total_pages: usize,
    pub total_rows: usize,
}

#[derive(Debug, Clone)]
pub struct DashboardData {
    report: LoadReport,
    events: Vec<EventRecord>,
    log_events: Vec<EventRecord>,
    stages: Vec<StageSegment>,
}

pub fn load_default_sources() -> DashboardData {
    let demo_path = env::var(ENV_LOG_WFU)
        .or_else(|_| env::var(ENV_DEMO_JSON))
        .unwrap_or_else(|_| default_demo_json_path());
    let wparse_path = env::var(ENV_WPARSE_LOG).unwrap_or_else(|_| default_wparse_log_path());
    let wfusion_alerts_path = env::var(ENV_ALERT_WFU_DIR)
        .or_else(|_| env::var(ENV_WFUSION_ALERTS))
        .unwrap_or_else(|_| default_wfusion_alerts_path());
    let wfusion_enabled = env_flag(ENV_USE_WFUSION, true);

    let mut report = LoadReport {
        compute_backend: if wfusion_enabled {
            "wfusion".to_string()
        } else {
            "local".to_string()
        },
        demo_path: demo_path.clone(),
        wparse_path: wparse_path.clone(),
        wfusion_alerts_path: wfusion_alerts_path.clone(),
        wfusion_enabled,
        ..LoadReport::default()
    };

    let mut events: Vec<EventRecord> = Vec::new();
    let mut log_events: Vec<EventRecord> = Vec::new();

    let mut loaded_primary_logs = false;
    match load_log_source(Path::new(&demo_path), log_events.len(), "wparse") {
        Ok((loaded, skipped)) => {
            report.demo_rows = loaded.len();
            report.demo_skipped = skipped;
            loaded_primary_logs = !loaded.is_empty();
            log_events.extend(loaded);
        }
        Err(err) => {
            report.errors.push(format!(
                "demo source '{}' load failed: {err}",
                report.demo_path
            ));
        }
    }

    if !loaded_primary_logs && Path::new(&wparse_path) != Path::new(&demo_path) {
        match load_log_source(Path::new(&wparse_path), log_events.len(), "wparse") {
            Ok((loaded, skipped)) => {
                report.wparse_rows = loaded.len();
                report.wparse_skipped = skipped;
                log_events.extend(loaded);
            }
            Err(err) => {
                report.errors.push(format!(
                    "wparse source '{}' load failed: {err}",
                    report.wparse_path
                ));
            }
        }
    }

    if wfusion_enabled {
        match load_wfusion_alerts(Path::new(&wfusion_alerts_path), events.len()) {
            Ok((loaded, skipped, resolved_paths)) => {
                report.wfusion_rows = loaded.len();
                report.wfusion_skipped = skipped;
                report.wfusion_used = !loaded.is_empty();
                if !resolved_paths.is_empty() {
                    report.wfusion_alerts_path = resolved_paths.join(", ");
                }
                events.extend(loaded);
            }
            Err(err) => {
                report.errors.push(format!(
                    "wfusion alerts '{}' load failed: {err}",
                    report.wfusion_alerts_path
                ));
            }
        }
    }

    if events.is_empty() {
        report.compute_backend = "local-fallback".to_string();
        if wfusion_enabled {
            report
                .errors
                .push("wfusion produced 0 rows, fallback to demo+wparse source".to_string());
        }
        events.extend(log_events.clone());
    }

    events.sort_by(compare_event_time);
    log_events.sort_by(compare_event_time);

    let stages = derive_stages(&mut events);
    assign_stage_indices(&mut log_events, &stages);

    enrich_report(&mut report, &events, &stages);

    DashboardData {
        report,
        events,
        log_events,
        stages,
    }
}

impl DashboardData {
    pub fn stage_label(&self, idx: usize) -> Option<&str> {
        self.stages.get(idx).map(|s| s.label.as_str())
    }

    pub fn build_view(
        &self,
        selected_stage: Option<usize>,
        level_filter: Option<LevelFilter>,
        risk_filter: Option<RiskFilter>,
        source_filter: Option<SourceFilter>,
    ) -> DashboardView {
        let mut report = self.report.clone();

        let (global_min_ns, global_max_ns) = timeline_axis_bounds_from_events(&self.events);
        let ns_span = (global_max_ns - global_min_ns).max(1) as f64;

        let mut stage_bands = Vec::new();
        for stage in &self.stages {
            let mut start_pct = ((stage.start_ns - global_min_ns) as f64 / ns_span) as f32;
            let mut end_pct = ((stage.end_ns - global_min_ns) as f64 / ns_span) as f32;
            start_pct = start_pct.clamp(0.0, 1.0);
            end_pct = end_pct.clamp(0.0, 1.0);
            if end_pct <= start_pct {
                end_pct = (start_pct + 0.02).min(1.0);
            }

            stage_bands.push(StageBandVm {
                label: stage.label.clone(),
                summary: format!(
                    "{} | action={} | incidents={} | {}ms",
                    stage.family, stage.top_action, stage.incident_count, stage.duration_ms
                ),
                start_pct,
                end_pct,
                selected: selected_stage == Some(stage.idx),
            });
        }

        let stage_filtered_events =
            filter_event_records(&self.events, selected_stage, None, None, None);

        report.total_rows = stage_filtered_events.len();
        let (risk_low_rows, risk_mid_rows, risk_high_rows) =
            count_risk_buckets(&stage_filtered_events);
        report.risk_low_rows = risk_low_rows;
        report.risk_mid_rows = risk_mid_rows;
        report.risk_high_rows = risk_high_rows;

        let filtered_events = filter_event_records(
            &self.events,
            selected_stage,
            level_filter,
            risk_filter,
            source_filter,
        );
        let (
            timeline_points,
            point_detail_summaries,
            point_detail_rows,
            point_previews,
            lane_legend_text,
            lane_labels,
        ) = build_timeline_points(&filtered_events, &self.log_events, &self.stages);
        let time_ticks = build_time_ticks(&filtered_events);
        let timeline_content_px = build_timeline_content_width_px(&filtered_events);
        let stage_cards = build_stage_cards(&self.stages, selected_stage);

        let stage_detail_text = build_stage_detail(&self.stages, selected_stage);
        let point_hint_text = if timeline_points.is_empty() {
            "No points in current selection.".to_string()
        } else {
            "Click any point to show input log details.".to_string()
        };

        DashboardView {
            report,
            stage_bands,
            timeline_points,
            time_ticks,
            timeline_content_px,
            lane_labels,
            stage_cards,
            point_detail_summaries,
            point_detail_rows,
            point_previews,
            stage_detail_text,
            point_hint_text,
            lane_legend_text,
        }
    }

    pub fn build_log_page(
        &self,
        selected_stage: Option<usize>,
        level_filter: Option<LevelFilter>,
        risk_filter: Option<RiskFilter>,
        source_filter: Option<SourceFilter>,
        page_idx: usize,
        page_size: usize,
    ) -> TablePageView {
        let filtered_logs = filter_event_records(
            &self.log_events,
            selected_stage,
            level_filter,
            risk_filter,
            source_filter,
        );
        build_table_page(filtered_logs, self.log_events.len(), page_idx, page_size, "log")
    }

    pub fn build_alert_page(
        &self,
        selected_stage: Option<usize>,
        level_filter: Option<LevelFilter>,
        risk_filter: Option<RiskFilter>,
        source_filter: Option<SourceFilter>,
        page_idx: usize,
        page_size: usize,
    ) -> TablePageView {
        let filtered_alerts = filter_event_records(
            &self.events,
            selected_stage,
            level_filter,
            risk_filter,
            source_filter,
        );
        build_table_page(
            filtered_alerts,
            self.events.len(),
            page_idx,
            page_size,
            "alert",
        )
    }
}

fn enrich_report(report: &mut LoadReport, events: &[EventRecord], stages: &[StageSegment]) {
    report.total_rows = events.len();
    report.stage_count = stages.len();

    let mut target_set = HashSet::new();
    let mut entity_set = HashSet::new();
    let mut target_counts: HashMap<String, usize> = HashMap::new();
    let mut entity_counts: HashMap<String, usize> = HashMap::new();

    for event in events {
        match risk_bucket(event.risk) {
            RiskBucket::Low => report.risk_low_rows += 1,
            RiskBucket::Mid => report.risk_mid_rows += 1,
            RiskBucket::High => report.risk_high_rows += 1,
        }

        if !event.target.is_empty() {
            target_set.insert(event.target.clone());
            *target_counts.entry(event.target.clone()).or_insert(0) += 1;
        }

        if !event.entity.is_empty() {
            entity_set.insert(event.entity.clone());
            *entity_counts.entry(event.entity.clone()).or_insert(0) += 1;
        }
    }

    report.unique_targets = target_set.len();
    report.unique_entities = entity_set.len();

    report.top_targets_text = format_top_counts(&target_counts, 12, "No target data");
    report.top_entities_text = format_top_counts(&entity_counts, 12, "No entity data");
    report.recent_events_text = format_recent_events(events, stages, 26);
    report.source_text = format_source_text(report);

    if let Some(first) = events.first() {
        report.first_ts = Some(first.time_text.clone());
    }
    if let Some(last) = events.last() {
        report.last_ts = Some(last.time_text.clone());
    }
}

#[derive(Debug, Clone, Copy)]
enum RiskBucket {
    Low,
    Mid,
    High,
}

fn risk_bucket(risk: f32) -> RiskBucket {
    if risk >= 0.85 {
        RiskBucket::High
    } else if risk >= 0.60 {
        RiskBucket::Mid
    } else {
        RiskBucket::Low
    }
}

fn count_risk_buckets(events: &[&EventRecord]) -> (usize, usize, usize) {
    let mut low = 0usize;
    let mut mid = 0usize;
    let mut high = 0usize;
    for event in events {
        match risk_bucket(event.risk) {
            RiskBucket::Low => low += 1,
            RiskBucket::Mid => mid += 1,
            RiskBucket::High => high += 1,
        }
    }
    (low, mid, high)
}

fn filter_event_records<'a>(
    records: &'a [EventRecord],
    selected_stage: Option<usize>,
    level_filter: Option<LevelFilter>,
    risk_filter: Option<RiskFilter>,
    source_filter: Option<SourceFilter>,
) -> Vec<&'a EventRecord> {
    records
        .iter()
        .filter(|event| {
            selected_stage.is_none_or(|idx| event.stage_idx == idx)
                && level_filter.is_none_or(|filter| match filter {
                    LevelFilter::Info => event.level == "INFO",
                    LevelFilter::Warn => event.level == "WARN",
                    LevelFilter::Error => event.level == "ERROR" || event.level == "FATAL",
                })
                && risk_filter.is_none_or(|filter| match filter {
                    RiskFilter::Low => matches!(risk_bucket(event.risk), RiskBucket::Low),
                    RiskFilter::Mid => matches!(risk_bucket(event.risk), RiskBucket::Mid),
                    RiskFilter::High => matches!(risk_bucket(event.risk), RiskBucket::High),
                })
                && source_filter.is_none_or(|filter| match filter {
                    SourceFilter::Demo => event.source == "demo",
                    SourceFilter::Wparse => event.source == "wparse",
                    SourceFilter::Wfusion => event.source == "wfusion",
                })
        })
        .collect()
}

fn detail_row_from_event(event: &EventRecord) -> DetailRowVm {
    DetailRowVm {
        row_no: String::new(),
        time: event.time_text.clone(),
        level: safe_text(&event.level).to_string(),
        risk_score: format!("{:.2}", event.risk),
        rule: safe_text(&event.rule).to_string(),
        target: safe_text(&event.target).to_string(),
        entity: safe_text(&event.entity).to_string(),
        action: safe_text(&event.action).to_string(),
        status: safe_text(&event.status).to_string(),
        content: truncate_text(&event.content.replace('\n', " | "), 220),
    }
}

fn build_filtered_table_summary(
    page_start: usize,
    page_end: usize,
    filtered_rows: usize,
    total_rows: usize,
    row_kind: &str,
) -> String {
    if filtered_rows == 0 {
        return format!("No {row_kind} rows matched the current filters.");
    }

    format!(
        "Showing rows {}-{} of {} matched {row_kind} rows (total {row_kind} rows: {}).",
        page_start + 1,
        page_end,
        filtered_rows,
        total_rows
    )
}

fn build_table_page(
    filtered_rows: Vec<&EventRecord>,
    total_rows: usize,
    page_idx: usize,
    page_size: usize,
    row_kind: &str,
) -> TablePageView {
    let filtered_count = filtered_rows.len();
    let page_size = page_size.max(1);
    let total_pages = filtered_count.max(1).div_ceil(page_size);
    let page_idx = page_idx.min(total_pages.saturating_sub(1));
    let start = page_idx * page_size;
    let end = (start + page_size).min(filtered_count);
    let rows = filtered_rows[start..end]
        .iter()
        .enumerate()
        .map(|(idx, event)| {
            let mut row = detail_row_from_event(event);
            row.row_no = (start + idx + 1).to_string();
            row
        })
        .collect::<Vec<_>>();

    TablePageView {
        summary: build_filtered_table_summary(start, end, filtered_count, total_rows, row_kind),
        rows,
        page_idx,
        total_pages,
        total_rows: filtered_count,
    }
}

fn load_log_source(
    path: &Path,
    seq_start: usize,
    source_label: &str,
) -> anyhow::Result<(Vec<EventRecord>, usize)> {
    if is_arrow_file(path) {
        load_log_arrow(path, seq_start, source_label)
    } else if path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            ext.eq_ignore_ascii_case("json")
                || ext.eq_ignore_ascii_case("jsonl")
                || ext.eq_ignore_ascii_case("ndjson")
        })
        .unwrap_or(false)
    {
        load_demo_ndjson(path, seq_start)
    } else {
        load_wparse_log(path, seq_start)
    }
}

fn load_log_arrow(
    path: &Path,
    seq_start: usize,
    source_label: &str,
) -> anyhow::Result<(Vec<EventRecord>, usize)> {
    let frames = read_arrow_frames(path)?;
    let mut events = Vec::new();
    let mut skipped = 0usize;
    let mut seq = seq_start;

    for frame in frames {
        let _tag = frame.tag;
        let batch = frame.batch;
        for row in 0..batch.num_rows() {
            match parse_log_arrow_row(&batch, row, seq, source_label) {
                Some(event) => {
                    events.push(event);
                    seq += 1;
                }
                None => skipped += 1,
            }
        }
    }

    Ok((events, skipped))
}

fn parse_log_arrow_row(
    batch: &RecordBatch,
    row: usize,
    seq: usize,
    source_label: &str,
) -> Option<EventRecord> {
    let time_text = batch_string(batch, row, "time")
        .or_else(|| batch_string(batch, row, "event_time"))?;
    let epoch_ns = batch_timestamp_ns(batch, row, "event_time")
        .or_else(|| batch_timestamp_ns(batch, row, "time"))
        .map(i128::from)
        .or_else(|| parse_epoch_ns(&time_text, batch_i64(batch, row, "ns")))?;

    let raw_score = batch_f64_any(batch, row, &["risk_score", "__wfu_score", "score"]);
    let mut computed_risk = raw_score.map(|score| {
        if score > 1.0 {
            (score / 100.0) as f32
        } else {
            score as f32
        }
    });
    if let Some(risk) = computed_risk.as_mut() {
        *risk = risk.clamp(0.0, 1.0);
    }

    let level = batch_string(batch, row, "level")
        .map(|s| normalize_level(&s))
        .or_else(|| computed_risk.map(infer_level_from_risk))
        .unwrap_or_else(|| "INFO".to_string());
    let target = batch_string(batch, row, "target")
        .map(|s| clean_text(&s))
        .unwrap_or_default();
    let subject = batch_string(batch, row, "meta.subject")
        .or_else(|| batch_string(batch, row, "subject"))
        .map(|s| clean_text(&s))
        .unwrap_or_default();
    let action = batch_string(batch, row, "meta.action")
        .or_else(|| batch_string(batch, row, "action"))
        .map(|s| clean_text(&s))
        .unwrap_or_default();
    let status = batch_string(batch, row, "meta.status")
        .or_else(|| batch_string(batch, row, "status"))
        .map(|s| clean_text(&s))
        .unwrap_or_default();
    let content = batch_string(batch, row, "content")
        .or_else(|| batch_string(batch, row, "message"))
        .map(|s| clean_text(&s))
        .unwrap_or_default();

    let entity = batch_string_any(batch, row, &["entity", "__wfu_entity_id"])
        .map(|s| clean_text(&s))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| first_non_empty(&subject, &target, "unknown").to_string());
    let risk = computed_risk.unwrap_or_else(|| score_risk(&level, &status, &content));
    let rule = batch_string_any(batch, row, &["rule_name", "__wfu_rule_name"])
        .map(|s| clean_text(&s))
        .unwrap_or_default();

    Some(EventRecord {
        seq,
        source: source_label.to_string(),
        time_text,
        epoch_ns,
        window_bucket_ns: None,
        level,
        rule,
        target,
        action,
        status,
        content,
        entity,
        risk,
        stage_idx: 0,
        stage_boundary_prob: 0.0,
    })
}

fn load_demo_ndjson(path: &Path, seq_start: usize) -> anyhow::Result<(Vec<EventRecord>, usize)> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let mut events = Vec::new();
    let mut skipped = 0;
    let mut seq = seq_start;

    for line in reader.lines() {
        let raw = line?;
        let text = raw.trim();
        if text.is_empty() {
            continue;
        }

        match serde_json::from_str::<Value>(text) {
            Ok(value) => {
                if let Some(event) = parse_demo_value(&value, seq) {
                    events.push(event);
                    seq += 1;
                } else {
                    skipped += 1;
                }
            }
            Err(_) => skipped += 1,
        }
    }

    Ok((events, skipped))
}

fn parse_demo_value(value: &Value, seq: usize) -> Option<EventRecord> {
    let time_text = value.get("time")?.as_str()?.to_string();
    let ns = value.get("ns").and_then(Value::as_i64);
    let epoch_ns = parse_epoch_ns(&time_text, ns)?;

    let level = value
        .get("level")
        .and_then(Value::as_str)
        .map(normalize_level)
        .unwrap_or_else(|| "INFO".to_string());

    let target = value
        .get("target")
        .and_then(Value::as_str)
        .map(clean_text)
        .unwrap_or_default();

    let subject = value
        .get("meta")
        .and_then(|v| v.get("subject"))
        .and_then(Value::as_str)
        .map(clean_text)
        .unwrap_or_default();

    let action = value
        .get("meta")
        .and_then(|v| v.get("action"))
        .and_then(Value::as_str)
        .map(clean_text)
        .unwrap_or_default();

    let status = value
        .get("meta")
        .and_then(|v| v.get("status"))
        .and_then(Value::as_str)
        .map(clean_text)
        .unwrap_or_default();

    let content = value
        .get("content")
        .and_then(Value::as_str)
        .map(clean_text)
        .unwrap_or_default();

    let entity = first_non_empty(&subject, &target, "unknown").to_string();
    let risk = score_risk(&level, &status, &content);

    Some(EventRecord {
        seq,
        source: "demo".to_string(),
        time_text,
        epoch_ns,
        window_bucket_ns: None,
        level,
        rule: String::new(),
        target,
        action,
        status,
        content,
        entity,
        risk,
        stage_idx: 0,
        stage_boundary_prob: 0.0,
    })
}

fn load_wparse_log(path: &Path, seq_start: usize) -> anyhow::Result<(Vec<EventRecord>, usize)> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let re = Regex::new(
        r"^(?P<time>\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d+)\s+\[(?P<level>[A-Za-z]+)\s*\]\s+\[(?P<target>[^\]]+)\]\s*(?P<content>.*)$",
    )?;

    let mut events: Vec<EventRecord> = Vec::new();
    let mut skipped = 0usize;
    let mut seq = seq_start;

    let mut current: Option<(String, String, String, String)> = None;

    for line in reader.lines() {
        let raw = line?;

        if let Some(caps) = re.captures(&raw) {
            if let Some((time_text, level, target, content)) = current.take()
                && let Some(event) = parse_wparse_entry(&time_text, &level, &target, &content, seq)
            {
                events.push(event);
                seq += 1;
            }

            let time_text = caps
                .name("time")
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let level = caps
                .name("level")
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            let target = caps
                .name("target")
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            let content = caps
                .name("content")
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();

            current = Some((time_text, level, target, content));
            continue;
        }

        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some((_time_text, _level, _target, content)) = current.as_mut() {
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(trimmed);
        } else {
            skipped += 1;
        }
    }

    if let Some((time_text, level, target, content)) = current.take()
        && let Some(event) = parse_wparse_entry(&time_text, &level, &target, &content, seq)
    {
        events.push(event);
    }

    Ok((events, skipped))
}

fn parse_wparse_entry(
    time_text: &str,
    level: &str,
    target: &str,
    content: &str,
    seq: usize,
) -> Option<EventRecord> {
    let epoch_ns = parse_epoch_ns(time_text, None)?;
    let level_norm = normalize_level(level);
    let target_norm = clean_text(target);
    let content_norm = clean_text(content);
    let action = guess_action_from_content(&content_norm);
    let status = guess_status_from_content(&content_norm);
    let entity = first_non_empty("", &target_norm, "unknown").to_string();
    let risk = score_risk(&level_norm, &status, &content_norm);

    Some(EventRecord {
        seq,
        source: "wparse".to_string(),
        time_text: time_text.to_string(),
        epoch_ns,
        window_bucket_ns: None,
        level: level_norm,
        rule: String::new(),
        target: target_norm,
        action,
        status,
        content: content_norm,
        entity,
        risk,
        stage_idx: 0,
        stage_boundary_prob: 0.0,
    })
}

fn load_wfusion_alerts(
    path: &Path,
    seq_start: usize,
) -> anyhow::Result<(Vec<EventRecord>, usize, Vec<String>)> {
    if is_arrow_file(path) {
        let (events, skipped) = load_wfusion_alerts_arrow(path, seq_start)?;
        return Ok((events, skipped, vec![path.to_string_lossy().to_string()]));
    }

    if path.is_dir() {
        let arrow_files = resolve_wfusion_arrow_files(path)?;
        if !arrow_files.is_empty() {
            let (events, skipped) = load_wfusion_alerts_arrow_files(&arrow_files, seq_start)?;
            let resolved_paths = arrow_files
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect::<Vec<_>>();
            return Ok((events, skipped, resolved_paths));
        }
    }

    let files = resolve_wfusion_files(path)?;
    if files.is_empty() {
        anyhow::bail!("no wfusion alert file found at '{}'", path.display());
    }

    let mut events = Vec::new();
    let mut skipped = 0usize;
    let mut seq = seq_start;

    for file_path in &files {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let raw = line?;
            let text = raw.trim();
            if text.is_empty() {
                continue;
            }

            match serde_json::from_str::<Value>(text) {
                Ok(value) => {
                    if let Some(event) = parse_wfusion_alert_value(&value, seq) {
                        events.push(event);
                        seq += 1;
                    } else {
                        skipped += 1;
                    }
                }
                Err(_) => skipped += 1,
            }
        }
    }

    let resolved_paths = files
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    Ok((events, skipped, resolved_paths))
}

fn load_wfusion_alerts_arrow_files(
    paths: &[PathBuf],
    seq_start: usize,
) -> anyhow::Result<(Vec<EventRecord>, usize)> {
    let mut events = Vec::new();
    let mut skipped = 0usize;
    let mut seq = seq_start;

    for path in paths {
        let (loaded, file_skipped) = load_wfusion_alerts_arrow(path, seq)?;
        seq += loaded.len();
        skipped += file_skipped;
        events.extend(loaded);
    }

    Ok((events, skipped))
}

fn load_wfusion_alerts_arrow(
    path: &Path,
    seq_start: usize,
) -> anyhow::Result<(Vec<EventRecord>, usize)> {
    let frames = read_arrow_frames(path)?;
    let mut events = Vec::new();
    let mut skipped = 0usize;
    let mut seq = seq_start;

    for frame in frames {
        let _tag = frame.tag;
        let batch = frame.batch;
        for row in 0..batch.num_rows() {
            match parse_wfusion_alert_arrow_row(&batch, row, seq) {
                Some(event) => {
                    events.push(event);
                    seq += 1;
                }
                None => skipped += 1,
            }
        }
    }

    Ok((events, skipped))
}

fn resolve_wfusion_files(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }

    if !path.is_dir() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if p.extension()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case("jsonl"))
            .unwrap_or(false)
        {
            files.push(p);
        }
    }

    if files.is_empty() {
        return Ok(files);
    }

    files.sort();

    if let Some(primary_path) = files.iter().find(|p| {
        p.file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case("wf-alert.jsonl"))
            .unwrap_or(false)
    }) {
        return Ok(vec![primary_path.clone()]);
    }

    if let Some(all_path) = files.iter().find(|p| {
        p.file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case("all.jsonl"))
            .unwrap_or(false)
    }) {
        return Ok(vec![all_path.clone()]);
    }

    let mut preferred = files
        .into_iter()
        .filter(|p| {
            let name = p
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            name != "error.jsonl" && name != "unrouted.jsonl"
        })
        .collect::<Vec<_>>();

    if preferred.is_empty() {
        return Ok(Vec::new());
    }

    preferred.sort();
    Ok(preferred)
}

fn resolve_wfusion_arrow_files(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    if path.is_file() {
        return if is_arrow_file(path) {
            Ok(vec![path.to_path_buf()])
        } else {
            Ok(Vec::new())
        };
    }

    if !path.is_dir() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() || !is_arrow_file(&p) {
            continue;
        }

        let name = p
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        if name.contains("alert") || name.ends_with("_wfu.arrow") {
            files.push(p);
        }
    }

    files.sort();
    Ok(files)
}

fn parse_wfusion_alert_value(value: &Value, seq: usize) -> Option<EventRecord> {
    let time_text = json_string_any(value, &["__wfu_fired_at"])?;
    let epoch_ns = parse_epoch_ns(&time_text, None)?;
    let window_bucket_ns = json_time_ns_any(value, &["window_bucket_time"]);

    let score = json_f64_any(value, &["risk_score", "__wfu_score", "score"]).unwrap_or(0.0);
    let mut risk = if score > 1.0 {
        (score / 100.0) as f32
    } else {
        score as f32
    };
    risk = risk.clamp(0.0, 1.0);

    let level = infer_level_from_risk(risk);
    let rule = json_string_any(value, &["__wfu_rule_name", "rule_name"])
        .map(|s| clean_text(&s))
        .unwrap_or_else(|| "wfusion_rule".to_string());
    let target = json_string_any(value, &["target"])
        .map(|s| clean_text(&s))
        .unwrap_or_default();
    let action = json_string_any(value, &["action"])
        .map(|s| clean_text(&s))
        .unwrap_or_default();
    let status = json_string_any(
        value,
        &[
            "status",
            "__wfu_close_reason",
            "close_reason",
            "__wfu_origin",
            "origin",
        ],
    )
        .map(|s| clean_text(&s))
        .unwrap_or_else(|| "emitted".to_string());
    let entity = json_string_any(value, &["__wfu_entity_id"])
        .map(|s| clean_text(&s))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    let summary = json_string_any(value, &["message"])
        .map(|s| clean_text(&s))
        .unwrap_or_default();
    let content = summary;

    Some(EventRecord {
        seq,
        source: "wfusion".to_string(),
        time_text,
        epoch_ns,
        window_bucket_ns,
        level,
        rule,
        target,
        action,
        status,
        content,
        entity,
        risk,
        stage_idx: 0,
        stage_boundary_prob: 0.0,
    })
}

fn parse_wfusion_alert_arrow_row(
    batch: &RecordBatch,
    row: usize,
    seq: usize,
) -> Option<EventRecord> {
    let time_text = batch_string_any(batch, row, &["__wfu_fired_at"])?;
    let epoch_ns = parse_epoch_ns(&time_text, None)?;
    let window_bucket_ns = batch_timestamp_ns(batch, row, "window_bucket_time").map(i128::from);

    let score = batch_f64_any(batch, row, &["risk_score", "__wfu_score", "score"]).unwrap_or(0.0);
    let mut risk = if score > 1.0 {
        (score / 100.0) as f32
    } else {
        score as f32
    };
    risk = risk.clamp(0.0, 1.0);

    let level = infer_level_from_risk(risk);
    let rule = batch_string_any(batch, row, &["__wfu_rule_name", "rule_name"])
        .map(|s| clean_text(&s))
        .unwrap_or_else(|| "wfusion_rule".to_string());
    let target = batch_string_any(batch, row, &["target"])
        .map(|s| clean_text(&s))
        .unwrap_or_default();
    let action = batch_string_any(batch, row, &["action"])
        .map(|s| clean_text(&s))
        .unwrap_or_default();
    let status = batch_string_any(
        batch,
        row,
        &[
            "status",
            "__wfu_close_reason",
            "close_reason",
            "__wfu_origin",
            "origin",
        ],
    )
        .map(|s| clean_text(&s))
        .unwrap_or_else(|| "emitted".to_string());
    let entity = batch_string_any(batch, row, &["__wfu_entity_id"])
        .map(|s| clean_text(&s))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    let summary = batch_string_any(batch, row, &["message"])
        .map(|s| clean_text(&s))
        .unwrap_or_default();
    let content = summary;

    Some(EventRecord {
        seq,
        source: "wfusion".to_string(),
        time_text,
        epoch_ns,
        window_bucket_ns,
        level,
        rule,
        target,
        action,
        status,
        content,
        entity,
        risk,
        stage_idx: 0,
        stage_boundary_prob: 0.0,
    })
}

fn batch_string(batch: &RecordBatch, row: usize, field: &str) -> Option<String> {
    let idx = batch.schema().index_of(field).ok()?;
    let col = batch.column(idx);
    if col.is_null(row) {
        return None;
    }

    if let Some(arr) = col.as_any().downcast_ref::<StringArray>() {
        return Some(arr.value(row).to_string());
    }
    if let Some(arr) = col.as_any().downcast_ref::<TimestampNanosecondArray>() {
        let value = arr.value(row);
        let dt = Utc.timestamp_nanos(value);
        return Some(dt.naive_utc().format("%Y-%m-%d %H:%M:%S%.9f").to_string());
    }

    None
}

fn batch_string_any(batch: &RecordBatch, row: usize, fields: &[&str]) -> Option<String> {
    fields.iter().find_map(|field| batch_string(batch, row, field))
}

fn batch_f64_any(batch: &RecordBatch, row: usize, fields: &[&str]) -> Option<f64> {
    fields.iter().find_map(|field| batch_f64(batch, row, field))
}

fn json_string_any(value: &Value, fields: &[&str]) -> Option<String> {
    fields
        .iter()
        .find_map(|field| value.get(*field).and_then(Value::as_str))
        .map(ToString::to_string)
}

fn json_f64_any(value: &Value, fields: &[&str]) -> Option<f64> {
    fields.iter().find_map(|field| value.get(*field).and_then(Value::as_f64))
}

fn json_time_ns_any(value: &Value, fields: &[&str]) -> Option<i128> {
    fields.iter().find_map(|field| {
        value.get(*field).and_then(|v| match v {
            Value::String(text) => parse_epoch_ns(text, None),
            Value::Number(num) => num.as_i64().map(i128::from),
            _ => None,
        })
    })
}

fn batch_i64(batch: &RecordBatch, row: usize, field: &str) -> Option<i64> {
    let idx = batch.schema().index_of(field).ok()?;
    let col = batch.column(idx);
    if col.is_null(row) {
        return None;
    }

    if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
        return Some(arr.value(row));
    }
    if let Some(arr) = col.as_any().downcast_ref::<Int32Array>() {
        return Some(arr.value(row) as i64);
    }
    if let Some(arr) = col.as_any().downcast_ref::<UInt64Array>() {
        return i64::try_from(arr.value(row)).ok();
    }
    if let Some(arr) = col.as_any().downcast_ref::<UInt32Array>() {
        return Some(arr.value(row) as i64);
    }

    None
}

fn batch_timestamp_ns(batch: &RecordBatch, row: usize, field: &str) -> Option<i64> {
    let idx = batch.schema().index_of(field).ok()?;
    let col = batch.column(idx);
    if col.is_null(row) {
        return None;
    }

    col.as_any()
        .downcast_ref::<TimestampNanosecondArray>()
        .map(|arr| arr.value(row))
}

fn batch_f64(batch: &RecordBatch, row: usize, field: &str) -> Option<f64> {
    let idx = batch.schema().index_of(field).ok()?;
    let col = batch.column(idx);
    if col.is_null(row) {
        return None;
    }

    if let Some(arr) = col.as_any().downcast_ref::<Float64Array>() {
        return Some(arr.value(row));
    }
    if let Some(arr) = col.as_any().downcast_ref::<Float32Array>() {
        return Some(arr.value(row) as f64);
    }
    if let Some(arr) = col.as_any().downcast_ref::<Int64Array>() {
        return Some(arr.value(row) as f64);
    }

    None
}

fn derive_stages(events: &mut [EventRecord]) -> Vec<StageSegment> {
    if events.is_empty() {
        return Vec::new();
    }

    let gaps = collect_gaps(events);
    let p95_gap = percentile_i128(&gaps, 95).max(1);
    let strong_gap_base = (p95_gap * 3).max(1);

    let boundary_threshold = 0.72_f32;
    let strong_gap_threshold = 0.98_f32;
    let min_segment_events = 12usize;

    let mut stage_idx = 0usize;
    let mut last_boundary_at = 0usize;

    for i in 0..events.len() {
        if i == 0 {
            events[i].stage_idx = stage_idx;
            events[i].stage_boundary_prob = 1.0;
            continue;
        }

        let prev = &events[i - 1];
        let curr = &events[i];

        let action_changed = (!prev.action.is_empty()
            && !curr.action.is_empty()
            && prev.action != curr.action) as i32;
        let entity_changed = (!prev.entity.is_empty()
            && !curr.entity.is_empty()
            && prev.entity != curr.entity) as i32;
        let boundary_action = is_boundary_action(&curr.action) as i32;
        let boundary_status = is_boundary_status(&curr.status, &curr.content) as i32;

        let gap_ns = (curr.epoch_ns - prev.epoch_ns).max(0);
        let gap_score = (gap_ns as f64 / strong_gap_base as f64).clamp(0.0, 1.0) as f32;

        let prob = (0.10
            + 0.35 * action_changed as f32
            + 0.20 * boundary_action as f32
            + 0.15 * boundary_status as f32
            + 0.20 * gap_score
            + 0.10 * entity_changed as f32)
            .clamp(0.0, 1.0);

        let is_candidate = prob >= boundary_threshold || gap_score >= strong_gap_threshold;
        let enough_distance = i.saturating_sub(last_boundary_at) >= min_segment_events;

        if is_candidate && enough_distance {
            stage_idx += 1;
            last_boundary_at = i;
        }

        events[i].stage_idx = stage_idx;
        events[i].stage_boundary_prob = prob;
    }

    build_stage_segments(events)
}

fn assign_stage_indices(events: &mut [EventRecord], stages: &[StageSegment]) {
    if events.is_empty() || stages.is_empty() {
        return;
    }

    let mut stage_idx = 0usize;
    for event in events.iter_mut() {
        while stage_idx + 1 < stages.len() {
            let current = &stages[stage_idx];
            let next = &stages[stage_idx + 1];
            let midpoint = current.end_ns + ((next.start_ns - current.end_ns) / 2);
            if event.epoch_ns >= midpoint {
                stage_idx += 1;
            } else {
                break;
            }
        }
        event.stage_idx = stage_idx;
    }
}

fn build_stage_segments(events: &[EventRecord]) -> Vec<StageSegment> {
    if events.is_empty() {
        return Vec::new();
    }

    let mut segments = Vec::new();
    let mut i = 0usize;

    while i < events.len() {
        let sid = events[i].stage_idx;
        let start = i;
        let mut end = i;
        while end + 1 < events.len() && events[end + 1].stage_idx == sid {
            end += 1;
        }

        let slice = &events[start..=end];
        let start_ns = slice.first().map(|e| e.epoch_ns).unwrap_or(0);
        let end_ns = slice.last().map(|e| e.epoch_ns).unwrap_or(start_ns);
        let duration_ms = ((end_ns - start_ns).max(0) / 1_000_000) as i64;

        let mut action_counts: HashMap<String, usize> = HashMap::new();
        let mut incident_count = 0usize;
        for e in slice {
            if !e.action.is_empty() {
                *action_counts.entry(e.action.clone()).or_insert(0) += 1;
            }
            if e.risk >= 0.70 || e.level == "WARN" || e.level == "ERROR" || e.level == "FATAL" {
                incident_count += 1;
            }
        }

        let top_action = action_counts
            .iter()
            .max_by(|(ak, av), (bk, bv)| av.cmp(bv).then_with(|| ak.cmp(bk)))
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let family = map_action_family(&top_action).to_string();
        let label = format!("{:02}-{}", sid + 1, family);

        let top_action_ratio = action_counts
            .get(&top_action)
            .map(|cnt| *cnt as f32 / slice.len() as f32)
            .unwrap_or(0.0);

        let start_prob = slice.first().map(|e| e.stage_boundary_prob).unwrap_or(0.5);
        let next_start_prob = events
            .iter()
            .find(|e| e.stage_idx == sid + 1)
            .map(|e| e.stage_boundary_prob)
            .unwrap_or(start_prob);

        let confidence =
            (0.50 * top_action_ratio + 0.30 * start_prob + 0.20 * next_start_prob).clamp(0.0, 1.0);

        segments.push(StageSegment {
            idx: sid,
            label,
            family,
            top_action,
            start_ns,
            end_ns,
            start_ts: slice
                .first()
                .map(|e| e.time_text.clone())
                .unwrap_or_default(),
            end_ts: slice
                .last()
                .map(|e| e.time_text.clone())
                .unwrap_or_default(),
            duration_ms,
            event_count: slice.len(),
            incident_count,
            confidence,
        });

        i = end + 1;
    }

    segments
}

fn build_timeline_points(
    filtered_events: &[&EventRecord],
    log_events: &[EventRecord],
    stages: &[StageSegment],
) -> (
    Vec<TimelinePointVm>,
    Vec<String>,
    Vec<Vec<DetailRowVm>>,
    Vec<String>,
    String,
    Vec<LaneLabelVm>,
) {
    if filtered_events.is_empty() {
        return (
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            "No entity lanes in current selection.".to_string(),
            Vec::new(),
        );
    }

    let mut entity_counts: HashMap<String, usize> = HashMap::new();
    for e in filtered_events {
        *entity_counts.entry(e.entity.clone()).or_insert(0) += 1;
    }

    let mut ranked_entities: Vec<(String, usize)> = entity_counts.into_iter().collect();
    ranked_entities.sort_by(|(an, ac), (bn, bc)| bc.cmp(ac).then_with(|| an.cmp(bn)));
    ranked_entities.truncate(MAX_LANES);

    if ranked_entities.is_empty() {
        return (
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            "No entity lanes in current selection.".to_string(),
            Vec::new(),
        );
    }

    let lane_map: HashMap<String, usize> = ranked_entities
        .iter()
        .enumerate()
        .map(|(idx, (entity, _))| (entity.clone(), idx))
        .collect();

    let (min_ns, max_ns) = timeline_axis_bounds_from_refs(filtered_events);
    let span = (max_ns - min_ns).max(1);
    let span_seconds = ((span as f64) / 1_000_000_000.0).ceil().max(1.0) as usize;
    let second_buckets = span_seconds.clamp(DEFAULT_BUCKETS, 7_200);
    let density_buckets = (filtered_events.len() / 6).clamp(24, 1_200);
    let buckets = second_buckets.max(density_buckets);
    let bucket_span = (span / buckets as i128).max(1);

    #[derive(Default, Clone)]
    struct BucketAgg {
        count: usize,
        risk_max: f32,
        sample_idx: usize,
    }

    let mut agg: HashMap<(usize, usize), BucketAgg> = HashMap::new();

    for (idx, event) in filtered_events.iter().enumerate() {
        let Some(&lane) = lane_map.get(&event.entity) else {
            continue;
        };

        let b = ((event.epoch_ns - min_ns) / bucket_span).clamp(0, buckets as i128 - 1) as usize;
        let entry = agg.entry((lane, b)).or_default();
        entry.count += 1;
        if event.risk >= entry.risk_max {
            entry.risk_max = event.risk;
            entry.sample_idx = idx;
        }
    }

    let mut agg_rows: Vec<((usize, usize), BucketAgg)> = agg.into_iter().collect();
    agg_rows.sort_by(|((la, ba), _), ((lb, bb), _)| ba.cmp(bb).then_with(|| la.cmp(lb)));

    let counts: Vec<usize> = agg_rows.iter().map(|(_, a)| a.count).collect();
    let p95_cnt = percentile_usize(&counts, 95).max(1) as f32;

    let mut points = Vec::new();
    let mut detail_summaries = Vec::new();
    let mut detail_rows = Vec::new();
    let mut previews = Vec::new();

    for ((lane, bucket), a) in agg_rows {
        let event = filtered_events[a.sample_idx];
        let lane_denom = if ranked_entities.len() <= 1 {
            1.0
        } else {
            (ranked_entities.len() - 1) as f32
        };

        let x_pct = (bucket as f32 + 0.5) / buckets as f32;
        let y_pct = timeline_lane_y_pct(lane, ranked_entities.len(), lane_denom);

        let size_norm = ((a.count as f32 + 1.0).ln() / (p95_cnt + 1.0).ln()).clamp(0.18, 1.0);

        let point_entity = ranked_entities
            .get(lane)
            .map(|(e, _)| e.clone())
            .unwrap_or_else(|| event.entity.clone());

        points.push(TimelinePointVm {
            x_pct,
            y_pct,
            risk: a.risk_max,
            size_norm,
            entity: point_entity.clone(),
        });

        let stage_label = stages
            .get(event.stage_idx)
            .map(|s| s.label.as_str())
            .unwrap_or("unknown-stage");

        let bucket_start_ns = min_ns + bucket as i128 * bucket_span;
        let bucket_end_ns = if bucket + 1 >= buckets {
            max_ns
        } else {
            min_ns + (bucket as i128 + 1) * bucket_span - 1
        };

        let (detail_summary, detail_row_items) = build_point_log_detail(
            log_events,
            event,
            &point_entity,
            stage_label,
            bucket_start_ns,
            bucket_end_ns,
            a.count,
            a.risk_max,
        );
        detail_summaries.push(detail_summary);
        detail_rows.push(detail_row_items);

        previews.push(format!(
            "entity={} | stage={} | risk={:.2} | cnt={}\n{}",
            event.entity,
            stage_label,
            a.risk_max,
            a.count,
            truncate_text(&event.content.replace('\n', " | "), 110)
        ));
    }

    let lane_legend = ranked_entities
        .iter()
        .enumerate()
        .map(|(idx, (entity, cnt))| {
            format!("{:>2}. {:<18} {}", idx + 1, truncate_text(entity, 18), cnt)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let lane_denom = if ranked_entities.len() <= 1 {
        1.0
    } else {
        (ranked_entities.len() - 1) as f32
    };
    let lane_labels = ranked_entities
        .iter()
        .enumerate()
        .map(|(idx, (entity, _))| LaneLabelVm {
            y_pct: timeline_lane_y_pct(idx, ranked_entities.len(), lane_denom),
            label: truncate_text(entity, 18),
        })
        .collect::<Vec<_>>();

    (
        points,
        detail_summaries,
        detail_rows,
        previews,
        lane_legend,
        lane_labels,
    )
}

fn build_stage_detail(stages: &[StageSegment], selected_stage: Option<usize>) -> String {
    if stages.is_empty() {
        return "No stage data.".to_string();
    }

    match selected_stage {
        Some(idx) if idx < stages.len() => {
            let s = &stages[idx];
            format!(
                "Selected Stage\n{}\n\nfamily={}\ntop_action={}\nevents={}\nincidents={}\nduration={}ms\nconfidence={:.2}\n\n{} -> {}",
                s.label,
                s.family,
                s.top_action,
                s.event_count,
                s.incident_count,
                s.duration_ms,
                s.confidence,
                s.start_ts,
                s.end_ts
            )
        }
        _ => {
            let mut lines = vec!["Stage Overview (click stage to filter)".to_string()];
            for s in stages.iter().take(12) {
                lines.push(format!(
                    "{} | {} | action={} | incident={} | {}ms | conf={:.2}",
                    s.label, s.family, s.top_action, s.incident_count, s.duration_ms, s.confidence
                ));
            }
            lines.join("\n")
        }
    }
}

fn build_point_log_detail(
    log_events: &[EventRecord],
    event: &EventRecord,
    point_entity: &str,
    stage_label: &str,
    bucket_start_ns: i128,
    bucket_end_ns: i128,
    bucket_count: usize,
    risk_max: f32,
) -> (String, Vec<DetailRowVm>) {
    let related_logs = collect_related_logs(
        log_events,
        event,
        point_entity,
        bucket_start_ns,
        bucket_end_ns,
    );

    let summary = format!(
        "stage={} | entity={} | bucket_count={} | matched_logs={} | risk_max={:.2}",
        stage_label,
        point_entity,
        bucket_count,
        related_logs.len(),
        risk_max
    );

    if related_logs.is_empty() {
        return (
            summary,
            vec![DetailRowVm {
                row_no: "1".to_string(),
                time: event.time_text.clone(),
                level: safe_text(&event.level).to_string(),
                risk_score: format!("{:.2}", event.risk),
                rule: safe_text(&event.rule).to_string(),
                target: safe_text(&event.target).to_string(),
                entity: point_entity.to_string(),
                action: safe_text(&event.action).to_string(),
                status: safe_text(&event.status).to_string(),
                content: truncate_text(&event.content.replace('\n', " | "), 220),
            }],
        );
    }

    let rows = related_logs
        .iter()
        .enumerate()
        .map(|(idx, log)| DetailRowVm {
            row_no: (idx + 1).to_string(),
            time: log.time_text.clone(),
            level: safe_text(&log.level).to_string(),
            risk_score: format!("{:.2}", log.risk),
            rule: safe_text(&log.rule).to_string(),
            target: safe_text(&log.target).to_string(),
            entity: safe_text(&log.entity).to_string(),
            action: safe_text(&log.action).to_string(),
            status: safe_text(&log.status).to_string(),
            content: truncate_text(&log.content.replace('\n', " | "), 220),
        })
        .collect::<Vec<_>>();

    (summary, rows)
}

fn collect_related_logs<'a>(
    log_events: &'a [EventRecord],
    event: &EventRecord,
    point_entity: &str,
    bucket_start_ns: i128,
    bucket_end_ns: i128,
) -> Vec<&'a EventRecord> {
    if event.source == "wfusion" {
        let bucket_ns = event
            .window_bucket_ns
            .unwrap_or_else(|| floor_second_bucket_ns(event.epoch_ns));
        let mut exact = log_events
            .iter()
            .filter(|log| log.entity == point_entity && floor_second_bucket_ns(log.epoch_ns) == bucket_ns)
            .collect::<Vec<_>>();
        exact.sort_by(|a, b| a.epoch_ns.cmp(&b.epoch_ns).then_with(|| a.seq.cmp(&b.seq)));
        if !exact.is_empty() {
            exact.truncate(12);
            return exact;
        }
    }

    let mut matched = log_events
        .iter()
        .filter(|log| log.epoch_ns >= bucket_start_ns && log.epoch_ns <= bucket_end_ns)
        .collect::<Vec<_>>();

    if matched.is_empty() {
        let pad = ((bucket_end_ns - bucket_start_ns).max(1) * 2).max(3_000_000_000);
        matched = log_events
            .iter()
            .filter(|log| (log.epoch_ns - event.epoch_ns).abs() <= pad)
            .collect::<Vec<_>>();
    }

    matched.sort_by(|a, b| {
        log_match_score(b, event, point_entity)
            .cmp(&log_match_score(a, event, point_entity))
            .then_with(|| {
                let ad = (a.epoch_ns - event.epoch_ns).abs();
                let bd = (b.epoch_ns - event.epoch_ns).abs();
                ad.cmp(&bd)
            })
            .then_with(|| a.seq.cmp(&b.seq))
    });
    matched.truncate(12);
    matched
}

fn floor_second_bucket_ns(epoch_ns: i128) -> i128 {
    (epoch_ns / SECOND_NS) * SECOND_NS
}

fn log_match_score(log: &EventRecord, event: &EventRecord, point_entity: &str) -> i32 {
    let mut score = 0i32;

    if log.entity == point_entity {
        score += 6;
    }
    if let Some((left, right)) = point_entity.split_once(':') {
        if log.target == left {
            score += 3;
        }
        if log.entity == right {
            score += 3;
        }
    }
    if !event.action.is_empty() && log.action == event.action {
        score += 2;
    }
    if !event.status.is_empty() && log.status == event.status {
        score += 1;
    }
    if !event.target.is_empty() && log.target == event.target {
        score += 1;
    }

    score
}

fn build_time_ticks(filtered_events: &[&EventRecord]) -> Vec<AxisTickVm> {
    if filtered_events.is_empty() {
        return Vec::new();
    }

    let (min_ns, max_ns) = timeline_axis_bounds_from_refs(filtered_events);
    let span_ns = (max_ns - min_ns).max(1);

    let span_seconds = ((span_ns as f64) / 1_000_000_000.0).max(1.0);
    let step_sec = choose_tick_step_seconds(span_seconds / 8.0);

    let min_sec = floor_div(min_ns, SECOND_NS);
    let max_sec = floor_div(max_ns, SECOND_NS);
    let first_tick_sec = round_up_to_step(min_sec, step_sec as i128);

    let mut ticks = Vec::new();
    let mut sec = first_tick_sec;
    while sec <= max_sec {
        let tick_ns = sec * SECOND_NS;
        let pct = ((tick_ns - min_ns) as f64 / span_ns as f64).clamp(0.0, 1.0) as f32;
        ticks.push(AxisTickVm {
            x_pct: pct,
            label: format_second_label(sec),
        });
        sec += step_sec as i128;
    }

    if ticks.len() < 2 {
        ticks.clear();
        ticks.push(AxisTickVm {
            x_pct: 0.0,
            label: format_second_label(min_sec),
        });
        ticks.push(AxisTickVm {
            x_pct: 1.0,
            label: format_second_label(max_sec),
        });
    }

    ticks
}

fn build_timeline_content_width_px(filtered_events: &[&EventRecord]) -> i32 {
    if filtered_events.is_empty() {
        return MIN_TIMELINE_WIDTH_PX as i32;
    }

    let (min_ns, max_ns) = timeline_axis_bounds_from_refs(filtered_events);
    let span_ns = (max_ns - min_ns).max(1);

    let span_seconds = ((span_ns as f64) / SECOND_NS as f64).ceil().max(1.0) as usize;
    let width = span_seconds
        .saturating_mul(TIMELINE_PX_PER_SEC)
        .clamp(MIN_TIMELINE_WIDTH_PX, MAX_TIMELINE_WIDTH_PX);
    width as i32
}

fn timeline_axis_bounds_from_refs(events: &[&EventRecord]) -> (i128, i128) {
    if events.is_empty() {
        return (0, SECOND_NS);
    }

    let min_ns = events.iter().map(|e| e.epoch_ns).min().unwrap_or(0);
    let max_ns = events.iter().map(|e| e.epoch_ns).max().unwrap_or(min_ns + 1);
    align_timeline_axis_bounds(min_ns, max_ns)
}

fn timeline_axis_bounds_from_events(events: &[EventRecord]) -> (i128, i128) {
    let (min_ns, max_ns) = ns_bounds(events);
    align_timeline_axis_bounds(min_ns, max_ns)
}

fn align_timeline_axis_bounds(min_ns: i128, max_ns: i128) -> (i128, i128) {
    let raw_span_ns = (max_ns - min_ns).max(1);
    let raw_span_sec = ((raw_span_ns as f64) / SECOND_NS as f64).ceil().max(1.0) as usize;
    let axis_step_sec = choose_axis_alignment_seconds(raw_span_sec) as i128;
    let axis_step_ns = axis_step_sec * SECOND_NS;

    let natural_start_ns = floor_div(min_ns, axis_step_ns) * axis_step_ns;
    let natural_end_ns = round_up_to_step(max_ns + 1, axis_step_ns);
    let natural_left_pad_ns = (min_ns - natural_start_ns).max(0);
    let natural_right_pad_ns = (natural_end_ns - max_ns).max(0);
    let max_natural_pad_ns = (raw_span_ns / 3).max(5 * SECOND_NS).min(30 * SECOND_NS);

    let (start_ns, mut end_ns) = if natural_left_pad_ns <= max_natural_pad_ns
        && natural_right_pad_ns <= max_natural_pad_ns
    {
        (natural_start_ns, natural_end_ns)
    } else {
        let visual_pad_ns = (raw_span_ns / 6).max(3 * SECOND_NS).min(12 * SECOND_NS);
        let padded_start_ns = floor_div(min_ns - visual_pad_ns, SECOND_NS) * SECOND_NS;
        let padded_end_ns = round_up_to_step(max_ns + visual_pad_ns + 1, SECOND_NS);
        (padded_start_ns, padded_end_ns)
    };

    if end_ns <= start_ns {
        end_ns = start_ns + axis_step_ns;
    }
    (start_ns, end_ns)
}

fn timeline_lane_y_pct(idx: usize, lane_count: usize, lane_denom: f32) -> f32 {
    if lane_count <= 1 {
        return 0.5;
    }

    let inner = (idx as f32 / lane_denom).clamp(0.0, 1.0);
    let pad = TIMELINE_VERTICAL_PADDING_PCT.clamp(0.0, 0.45);
    pad + inner * (1.0 - pad * 2.0)
}

fn choose_axis_alignment_seconds(raw_span_sec: usize) -> usize {
    match raw_span_sec {
        0..=300 => 60,
        301..=1800 => 300,
        1801..=7200 => 900,
        _ => usize::try_from(choose_tick_step_seconds(raw_span_sec as f64 / 8.0))
            .unwrap_or(1800)
            .max(1800),
    }
}

fn build_stage_cards(stages: &[StageSegment], selected_stage: Option<usize>) -> Vec<StageCardVm> {
    let max_cards = 8usize;
    stages
        .iter()
        .take(max_cards)
        .map(|s| StageCardVm {
            idx: s.idx,
            label: s.label.clone(),
            action: truncate_text(&s.top_action, 18),
            summary: format!(
                "incident={} | dur={}ms | conf={:.2}",
                s.incident_count, s.duration_ms, s.confidence
            ),
            selected: selected_stage == Some(s.idx),
        })
        .collect()
}

fn collect_gaps(events: &[EventRecord]) -> Vec<i128> {
    if events.len() < 2 {
        return Vec::new();
    }

    let mut gaps = Vec::with_capacity(events.len() - 1);
    for i in 1..events.len() {
        let gap = (events[i].epoch_ns - events[i - 1].epoch_ns).max(0);
        gaps.push(gap);
    }
    gaps
}

fn compare_event_time(a: &EventRecord, b: &EventRecord) -> Ordering {
    a.epoch_ns.cmp(&b.epoch_ns).then_with(|| a.seq.cmp(&b.seq))
}

fn ns_bounds(events: &[EventRecord]) -> (i128, i128) {
    if events.is_empty() {
        return (0, 1);
    }

    let min_ns = events.iter().map(|e| e.epoch_ns).min().unwrap_or(0);
    let max_ns = events
        .iter()
        .map(|e| e.epoch_ns)
        .max()
        .unwrap_or(min_ns + 1);
    (min_ns, max_ns.max(min_ns + 1))
}

fn parse_epoch_ns(time_str: &str, ns: Option<i64>) -> Option<i128> {
    if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(time_str) {
        return ts.timestamp_nanos_opt().map(i128::from);
    }

    if let Ok(naive) = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S%.f") {
        let ts = Utc.from_utc_datetime(&naive);
        return ts.timestamp_nanos_opt().map(i128::from);
    }

    if let Ok(naive) = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S") {
        let ts = Utc.from_utc_datetime(&naive);
        let base = ts.timestamp_nanos_opt().map(i128::from)?;
        let extra = ns.unwrap_or(0) as i128;
        return Some(base + extra);
    }

    None
}

fn env_flag(name: &str, default: bool) -> bool {
    match env::var(name) {
        Ok(raw) => match raw.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => default,
        },
        Err(_) => default,
    }
}

fn normalize_level(level: &str) -> String {
    level.trim().to_ascii_uppercase()
}

fn infer_level_from_risk(risk: f32) -> String {
    if risk >= 0.85 {
        "ERROR".to_string()
    } else if risk >= 0.55 {
        "WARN".to_string()
    } else {
        "INFO".to_string()
    }
}

fn clean_text(s: &str) -> String {
    s.trim().to_string()
}

fn safe_text(s: &str) -> &str {
    if s.is_empty() { "-" } else { s }
}

fn first_non_empty<'a>(first: &'a str, second: &'a str, fallback: &'a str) -> &'a str {
    if !first.trim().is_empty() {
        first.trim()
    } else if !second.trim().is_empty() {
        second.trim()
    } else {
        fallback
    }
}

fn guess_action_from_content(content: &str) -> String {
    let first = content
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .find(|token| !token.is_empty())
        .unwrap_or("unknown");
    first.to_ascii_lowercase()
}

fn guess_status_from_content(content: &str) -> String {
    let lower = content.to_ascii_lowercase();
    if lower.contains("error") || lower.contains("fail") || lower.contains("exception") {
        return "error".to_string();
    }
    if lower.contains("warn") || lower.contains("miss") || lower.contains("pending") {
        return "warn".to_string();
    }
    if lower.contains("success")
        || lower.contains("suc")
        || lower.contains("done")
        || lower.contains("end")
    {
        return "success".to_string();
    }
    String::new()
}

fn is_boundary_action(action: &str) -> bool {
    let a = action.to_ascii_lowercase();
    [
        "init", "load", "start", "stop", "end", "close", "spawn", "parse", "run", "shutdown",
    ]
    .iter()
    .any(|k| a.contains(k))
}

fn is_boundary_status(status: &str, content: &str) -> bool {
    let s = format!(
        "{} {}",
        status.to_ascii_lowercase(),
        content.to_ascii_lowercase()
    );
    [
        "success",
        "suc",
        "fail",
        "error",
        "exception",
        "timeout",
        "terminal",
        "started",
        "ended",
    ]
    .iter()
    .any(|k| s.contains(k))
}

fn map_action_family(action: &str) -> &'static str {
    let a = action.to_ascii_lowercase();
    if [
        "init", "load", "create", "alloc", "build", "find", "validate", "config", "update",
    ]
    .iter()
    .any(|k| a.contains(k))
    {
        return "prepare";
    }
    if [
        "start", "run", "parse", "process", "work", "spawn", "dispatch", "send", "receive",
    ]
    .iter()
    .any(|k| a.contains(k))
    {
        return "running";
    }
    if ["close", "stop", "end", "exit", "shutdown", "terminal"]
        .iter()
        .any(|k| a.contains(k))
    {
        return "shutdown";
    }
    if ["fail", "error", "miss", "timeout", "exception", "blocking"]
        .iter()
        .any(|k| a.contains(k))
    {
        return "incident";
    }
    if ["monitor", "stat", "speed", "log", "version", "trace"]
        .iter()
        .any(|k| a.contains(k))
    {
        return "observe";
    }
    "other"
}

fn score_risk(level: &str, status: &str, content: &str) -> f32 {
    let mut score = 0.10_f32;

    match level {
        "WARN" => score = score.max(0.55),
        "ERROR" | "FATAL" => score = score.max(0.85),
        _ => {}
    }

    let status_l = status.to_ascii_lowercase();
    if contains_any(
        &status_l,
        &["error", "fail", "exception", "timeout", "terminal"],
    ) {
        score = score.max(0.90);
    } else if contains_any(&status_l, &["warn", "miss", "pending", "blocked"]) {
        score = score.max(0.60);
    } else if contains_any(&status_l, &["success", "suc", "end", "complete", "done"]) {
        score = score.max(0.20);
    }

    let content_l = content.to_ascii_lowercase();
    if contains_any(
        &content_l,
        &["error", "fail", "exception", "miss", "timeout"],
    ) {
        score = score.max(0.70);
    }
    if contains_any(&content_l, &["success", "suc", "completed", "done"]) {
        score = score.min(0.35);
    }

    score.clamp(0.0, 1.0)
}

fn contains_any(text: &str, words: &[&str]) -> bool {
    words.iter().any(|w| text.contains(w))
}

fn percentile_i128(values: &[i128], p: usize) -> i128 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let idx = ((p as f64 / 100.0) * (sorted.len().saturating_sub(1) as f64)).round() as usize;
    sorted[idx]
}

fn percentile_usize(values: &[usize], p: usize) -> usize {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let idx = ((p as f64 / 100.0) * (sorted.len().saturating_sub(1) as f64)).round() as usize;
    sorted[idx]
}

fn format_top_counts(counts: &HashMap<String, usize>, limit: usize, empty: &str) -> String {
    if counts.is_empty() {
        return empty.to_string();
    }

    let mut rows: Vec<(&String, &usize)> = counts.iter().collect();
    rows.sort_by(|(ka, va), (kb, vb)| vb.cmp(va).then_with(|| ka.cmp(kb)));

    rows.into_iter()
        .take(limit)
        .enumerate()
        .map(|(idx, (name, cnt))| {
            format!("{:>2}. {:<18} {}", idx + 1, truncate_text(name, 18), cnt)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_recent_events(events: &[EventRecord], stages: &[StageSegment], limit: usize) -> String {
    if events.is_empty() {
        return "No event data loaded.".to_string();
    }

    let start = events.len().saturating_sub(limit);
    events[start..]
        .iter()
        .map(|event| {
            let stage_label = stages
                .get(event.stage_idx)
                .map(|s| s.label.as_str())
                .unwrap_or("-");
            let text = truncate_text(&event.content.replace('\n', " | "), 78);
            format!(
                "{} | {:<5} | {:<6} | {:<10} | {:<10} | {}",
                event.time_text,
                event.level,
                event.source,
                truncate_text(&event.target, 10),
                stage_label,
                text
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_source_text(report: &LoadReport) -> String {
    let mut lines = vec![
        "Compute Backend".to_string(),
        format!("backend      : {}", report.compute_backend),
        format!("wfusion_on   : {}", report.wfusion_enabled),
        format!("wfusion_used : {}", report.wfusion_used),
        format!("wfusion_path : {}", report.wfusion_alerts_path),
        format!(
            "wfusion rows : {} (skip={})",
            report.wfusion_rows, report.wfusion_skipped
        ),
        "".to_string(),
        "Raw Source Paths".to_string(),
        format!("demo   : {}", report.demo_path),
        format!("wparse : {}", report.wparse_path),
        "".to_string(),
        "Load Notes".to_string(),
        format!("demo skipped lines   : {}", report.demo_skipped),
        format!("wparse skipped lines : {}", report.wparse_skipped),
    ];

    if report.errors.is_empty() {
        lines.push("source error: none".to_string());
    } else {
        lines.push("source error details:".to_string());
        for err in &report.errors {
            lines.push(format!("- {err}"));
        }
    }

    lines.join("\n")
}

fn choose_tick_step_seconds(raw_step: f64) -> i64 {
    const STEPS: [i64; 16] = [
        1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 900, 1_800, 3_600, 7_200, 14_400, 21_600,
    ];
    for step in STEPS {
        if raw_step <= step as f64 {
            return step;
        }
    }
    43_200
}

fn round_up_to_step(value: i128, step: i128) -> i128 {
    if step <= 0 {
        return value;
    }
    if value >= 0 {
        ((value + step - 1) / step) * step
    } else {
        (value / step) * step
    }
}

fn floor_div(value: i128, divisor: i128) -> i128 {
    if divisor == 0 {
        return 0;
    }
    let q = value / divisor;
    let r = value % divisor;
    if r != 0 && ((r > 0) != (divisor > 0)) {
        q - 1
    } else {
        q
    }
}

fn format_second_label(epoch_sec: i128) -> String {
    let Ok(sec_i64) = i64::try_from(epoch_sec) else {
        return "-".to_string();
    };
    let Some(dt) = Utc.timestamp_opt(sec_i64, 0).single() else {
        return "-".to_string();
    };
    dt.format("%H:%M:%S").to_string()
}

fn truncate_text(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }

    let mut out = String::new();
    for (idx, ch) in input.chars().enumerate() {
        if idx >= max_chars.saturating_sub(1) {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}
