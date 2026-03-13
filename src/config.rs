use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Deserialize;

const ENV_CONFIG_PATH: &str = "WARP_DIAGNOSE_CONFIG";
const ENV_LOG_WFU: &str = "WARP_DIAGNOSE_LOG_WFU";
const ENV_DEMO_JSON: &str = "WARP_DIAGNOSE_DEMO_JSON";
const ENV_WPARSE_LOG: &str = "WARP_DIAGNOSE_WPARSE_LOG";
const ENV_USE_WFUSION: &str = "WARP_DIAGNOSE_USE_WFUSION";
const ENV_ALERT_WFU_DIR: &str = "WARP_DIAGNOSE_ALERT_WFU_DIR";
const ENV_WFUSION_ALERTS: &str = "WARP_DIAGNOSE_WFUSION_ALERTS";
const ENV_TIMELINE_UNIT_MS: &str = "WARP_DIAGNOSE_TIMELINE_UNIT_MS";

const LOCAL_CASE_LOG_WFU_ARROW: &str = "case/wp-tools/data/out_dat/log_wfu.arrow";
const LOCAL_CASE_WP_LOG_ARROW: &str = "case/wp-tools/data/out_dat/wp-log.arrow";
const LOCAL_CASE_WF_ALERT_ARROW: &str = "case/wp-tools/alerts/wf-alert.arrow";
const LOCAL_CASE_WF_ALERT_DIR: &str = "case/wp-tools/alerts";
const LOCAL_CASE_DEMO_JSON: &str = "case/wp-tools/data/out_dat/demo.json";
const LOCAL_CASE_DEMO_JSON_LEGACY: &str = "case/wp-tools/data/demo.json";
const LOCAL_CASE_WPARSE_LOG: &str = "case/target_data/raw_log.dat";
const LOCAL_CASE_WFUSION_ALERTS: &str = "case/wp-tools/alerts/wf-alert.jsonl";
const LOCAL_CASE_WFUSION_ALERTS_LEGACY: &str = "case/wp-tools/alerts/all.jsonl";
const DEFAULT_CONFIG_PATH: &str = "config/warp-diagnose.toml";

static RUNTIME_CONFIG: OnceLock<AppConfig> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub data: DataConfig,
    pub timeline: TimelineConfig,
    pub table: TableConfig,
    pub window: WindowConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DataConfig {
    pub primary_log_path: String,
    pub wparse_log_path: String,
    pub wfusion_alerts_path: String,
    pub wfusion_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TimelineConfig {
    pub unit_ms: usize,
    pub max_lanes: usize,
    pub min_width_px: usize,
    pub max_width_px: usize,
    pub px_per_unit: usize,
    pub vertical_padding_pct: f32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TableConfig {
    pub window_chrome_px: usize,
    pub row_height_px: usize,
    pub min_page_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    pub width: f32,
    pub height: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            data: DataConfig::default(),
            timeline: TimelineConfig::default(),
            table: TableConfig::default(),
            window: WindowConfig::default(),
        }
    }
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            primary_log_path: default_primary_log_path(),
            wparse_log_path: manifest_path(LOCAL_CASE_WPARSE_LOG),
            wfusion_alerts_path: default_wfusion_alerts_path(),
            wfusion_enabled: true,
        }
    }
}

impl Default for TimelineConfig {
    fn default() -> Self {
        Self {
            unit_ms: 100,
            max_lanes: 9,
            min_width_px: 3600,
            max_width_px: 24_000,
            px_per_unit: 14,
            vertical_padding_pct: 0.08,
        }
    }
}

impl Default for TableConfig {
    fn default() -> Self {
        Self {
            window_chrome_px: 308,
            row_height_px: 34,
            min_page_size: 1,
        }
    }
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 1680.0,
            height: 980.0,
        }
    }
}

pub fn runtime_config() -> &'static AppConfig {
    RUNTIME_CONFIG.get_or_init(load_runtime_config)
}

fn load_runtime_config() -> AppConfig {
    let mut config = load_config_file().unwrap_or_else(|err| {
        eprintln!("[warp-diagnose] config load failed: {err}");
        AppConfig::default()
    });

    apply_env_overrides(&mut config);
    normalize_config(&mut config);
    config
}

fn load_config_file() -> anyhow::Result<AppConfig> {
    let path = resolve_config_path();
    if !path.exists() {
        return Ok(AppConfig::default());
    }

    let raw = fs::read_to_string(&path)?;
    let mut config: AppConfig = toml::from_str(&raw)?;
    if config.data.primary_log_path.is_empty() {
        config.data.primary_log_path = default_primary_log_path();
    }
    if config.data.wparse_log_path.is_empty() {
        config.data.wparse_log_path = manifest_path(LOCAL_CASE_WPARSE_LOG);
    }
    if config.data.wfusion_alerts_path.is_empty() {
        config.data.wfusion_alerts_path = default_wfusion_alerts_path();
    }
    Ok(config)
}

fn resolve_config_path() -> PathBuf {
    env::var(ENV_CONFIG_PATH)
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_CONFIG_PATH))
}

fn apply_env_overrides(config: &mut AppConfig) {
    if let Ok(path) = env::var(ENV_LOG_WFU).or_else(|_| env::var(ENV_DEMO_JSON)) {
        config.data.primary_log_path = path;
    }
    if let Ok(path) = env::var(ENV_WPARSE_LOG) {
        config.data.wparse_log_path = path;
    }
    if let Ok(path) = env::var(ENV_ALERT_WFU_DIR).or_else(|_| env::var(ENV_WFUSION_ALERTS)) {
        config.data.wfusion_alerts_path = path;
    }
    if let Ok(value) = env::var(ENV_USE_WFUSION) {
        config.data.wfusion_enabled = parse_bool(&value, config.data.wfusion_enabled);
    }
    if let Ok(value) = env::var(ENV_TIMELINE_UNIT_MS)
        && let Ok(unit_ms) = value.trim().parse::<usize>()
    {
        config.timeline.unit_ms = unit_ms;
    }
}

fn normalize_config(config: &mut AppConfig) {
    config.timeline.unit_ms = config.timeline.unit_ms.clamp(10, 60_000);
    config.timeline.max_lanes = config.timeline.max_lanes.max(1);
    config.timeline.min_width_px = config.timeline.min_width_px.max(400);
    config.timeline.max_width_px = config
        .timeline
        .max_width_px
        .max(config.timeline.min_width_px);
    config.timeline.px_per_unit = config.timeline.px_per_unit.max(1);
    config.timeline.vertical_padding_pct = config.timeline.vertical_padding_pct.clamp(0.0, 0.45);
    config.table.window_chrome_px = config.table.window_chrome_px.max(1);
    config.table.row_height_px = config.table.row_height_px.max(1);
    config.table.min_page_size = config.table.min_page_size.max(1);
    config.window.width = config.window.width.max(1500.0);
    config.window.height = config.window.height.max(920.0);
}

fn parse_bool(raw: &str, default: bool) -> bool {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => default,
    }
}

fn manifest_path(relative: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(relative)
        .to_string_lossy()
        .to_string()
}

fn default_primary_log_path() -> String {
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
