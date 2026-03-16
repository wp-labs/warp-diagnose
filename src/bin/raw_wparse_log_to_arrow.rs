use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, bail};
use arrow::array::{Float64Array, Int64Array, StringArray, TimestampNanosecondArray};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use chrono::{NaiveDateTime, TimeZone, Utc};
use regex::Regex;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        bail!("usage: raw_wparse_log_to_arrow <input.log> <output.arrow>");
    }

    let output = PathBuf::from(args.pop().expect("output"));
    let input = PathBuf::from(args.pop().expect("input"));
    let (rows, skipped) = load_rows(&input)?;
    if rows.is_empty() {
        bail!("no rows parsed from '{}' (skipped={skipped})", input.display());
    }

    let schema = Arc::new(Schema::new(vec![
        Field::new("event_time", DataType::Timestamp(TimeUnit::Nanosecond, None), true),
        Field::new("level", DataType::Utf8, true),
        Field::new("target", DataType::Utf8, true),
        Field::new("subject", DataType::Utf8, true),
        Field::new("action", DataType::Utf8, true),
        Field::new("status", DataType::Utf8, true),
        Field::new("action_class", DataType::Utf8, true),
        Field::new("status_class", DataType::Utf8, true),
        Field::new("impl_importance", DataType::Int64, true),
        Field::new("action_weight", DataType::Int64, true),
        Field::new("status_weight", DataType::Int64, true),
        Field::new("risk_score", DataType::Float64, true),
        Field::new("content", DataType::Utf8, true),
        Field::new("source", DataType::Utf8, true),
    ]));

    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(TimestampNanosecondArray::from(
                rows.iter().map(|r| Some(r.event_time)).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|r| Some(r.level.clone())).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|r| Some(r.target.clone())).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|r| Some(r.subject.clone())).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|r| Some(r.action.clone())).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|r| Some(r.status.clone())).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|r| Some(r.action_class.clone())).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|r| Some(r.status_class.clone())).collect::<Vec<_>>(),
            )),
            Arc::new(Int64Array::from(
                rows.iter().map(|r| Some(r.impl_importance)).collect::<Vec<_>>(),
            )),
            Arc::new(Int64Array::from(
                rows.iter().map(|r| Some(r.action_weight)).collect::<Vec<_>>(),
            )),
            Arc::new(Int64Array::from(
                rows.iter().map(|r| Some(r.status_weight)).collect::<Vec<_>>(),
            )),
            Arc::new(Float64Array::from(
                rows.iter().map(|r| Some(r.risk_score)).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|r| Some(r.content.clone())).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter().map(|r| Some(r.source.clone())).collect::<Vec<_>>(),
            )),
        ],
    )
    .context("build raw log arrow record batch")?;

    let payload = wp_arrow::ipc::encode_ipc("wparse", &batch)
        .map_err(|err| anyhow::anyhow!("encode arrow ipc failed: {err}"))?;

    if let Some(parent) = output.parent()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create '{}'", parent.display()))?;
    }

    let mut out = File::create(&output).with_context(|| format!("create '{}'", output.display()))?;
    out.write_all(&(payload.len() as u32).to_be_bytes())?;
    out.write_all(&payload)?;
    out.flush()?;

    println!(
        "wrote {} rows (skipped {}) -> {}",
        batch.num_rows(),
        skipped,
        output.display()
    );
    Ok(())
}

#[derive(Debug)]
struct Row {
    event_time: i64,
    level: String,
    target: String,
    subject: String,
    action: String,
    status: String,
    action_class: String,
    status_class: String,
    impl_importance: i64,
    action_weight: i64,
    status_weight: i64,
    risk_score: f64,
    content: String,
    source: String,
}

fn load_rows(path: &PathBuf) -> anyhow::Result<(Vec<Row>, usize)> {
    let file = File::open(path).with_context(|| format!("open '{}'", path.display()))?;
    let reader = BufReader::new(file);
    let re = Regex::new(
        r"^(?P<time>\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d+)\s+\[(?P<level>[A-Za-z]+)\s*\]\s+\[(?P<target>[^\]]+)\]\s*(?P<content>.*)$",
    )?;

    let mut rows = Vec::new();
    let mut skipped = 0usize;
    let mut current: Option<(String, String, String, String)> = None;

    for line in reader.lines() {
        let raw = line?;

        if let Some(caps) = re.captures(&raw) {
            if let Some((time_text, level, target, content)) = current.take() {
                if let Some(row) = parse_entry(&time_text, &level, &target, &content) {
                    rows.push(row);
                } else {
                    skipped += 1;
                }
            }

            current = Some((
                caps.name("time").map(|m| m.as_str().to_string()).unwrap_or_default(),
                caps.name("level").map(|m| m.as_str().to_string()).unwrap_or_default(),
                caps.name("target")
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default(),
                caps.name("content")
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default(),
            ));
            continue;
        }

        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some((_time, _level, _target, content)) = current.as_mut() {
            if !content.is_empty() {
                content.push('\n');
            }
            content.push_str(trimmed);
        } else {
            skipped += 1;
        }
    }

    if let Some((time_text, level, target, content)) = current.take() {
        if let Some(row) = parse_entry(&time_text, &level, &target, &content) {
            rows.push(row);
        } else {
            skipped += 1;
        }
    }

    Ok((rows, skipped))
}

fn parse_entry(time_text: &str, level: &str, target: &str, content: &str) -> Option<Row> {
    let event_time = parse_epoch_ns(time_text)?;
    let level = normalize_level(level);
    let target = clean_text(target);
    let content = clean_text(content);
    let subject = if target.is_empty() {
        "unknown".to_string()
    } else {
        target.clone()
    };
    let action = guess_action_from_content(&content);
    let status = guess_status_from_content(&content);
    let action_class = classify_action(&action).to_string();
    let status_class = classify_status(&status).to_string();
    let impl_importance = classify_importance(&subject);
    let action_weight = classify_action_weight(&action_class);
    let status_weight = classify_status_weight(&status_class);
    let risk_score = f64::from(score_risk(&level, &status, &content) * 100.0);

    Some(Row {
        event_time,
        level,
        target,
        subject,
        action,
        status,
        action_class,
        status_class,
        impl_importance,
        action_weight,
        status_weight,
        risk_score,
        content,
        source: "wparse".to_string(),
    })
}

fn parse_epoch_ns(time_str: &str) -> Option<i64> {
    if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(time_str) {
        return ts.timestamp_nanos_opt();
    }

    if let Ok(naive) = NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S%.f") {
        let ts = Utc.from_utc_datetime(&naive);
        return ts.timestamp_nanos_opt();
    }

    None
}

fn normalize_level(level: &str) -> String {
    level.trim().to_ascii_uppercase()
}

fn clean_text(s: &str) -> String {
    s.trim().to_string()
}

fn guess_action_from_content(content: &str) -> String {
    content
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .find(|token| !token.is_empty())
        .unwrap_or("unknown")
        .to_ascii_lowercase()
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

fn classify_action(action: &str) -> &'static str {
    let a = action.to_ascii_lowercase();
    if [
        "delete", "drop", "abort", "panic", "parse", "load", "save", "alloc", "create", "open",
        "connect",
    ]
    .iter()
    .any(|k| a.contains(k))
    {
        return "critical";
    }
    if ["start", "spawn", "run", "init", "update", "write"]
        .iter()
        .any(|k| a.contains(k))
    {
        return "lifecycle";
    }
    "other"
}

fn classify_status(status: &str) -> &'static str {
    let s = status.to_ascii_lowercase();
    if ["error", "fail", "failed", "failure", "timeout", "fatal", "panic", "abort"]
        .iter()
        .any(|k| s.contains(k))
    {
        return "failed";
    }
    if ["miss", "disabled", "retry", "partial", "degraded"]
        .iter()
        .any(|k| s.contains(k))
    {
        return "degraded";
    }
    if ["warn", "warning", "pending"]
        .iter()
        .any(|k| s.contains(k))
    {
        return "warning";
    }
    if ["start", "started", "running"]
        .iter()
        .any(|k| s.contains(k))
    {
        return "running";
    }
    if ["success", "suc", "ok", "enabled", "done", "pass"]
        .iter()
        .any(|k| s.contains(k))
    {
        return "success";
    }
    if ["end", "ended"].iter().any(|k| s.contains(k)) {
        return "ended";
    }
    "other"
}

fn classify_importance(subject: &str) -> i64 {
    match subject {
        "ctrl" => 100,
        "wpl" | "sink" | "oml" => 80,
        "source" | "picker" => 50,
        "monitor" => 30,
        _ => 75,
    }
}

fn classify_action_weight(action_class: &str) -> i64 {
    match action_class {
        "critical" => 100,
        "lifecycle" => 90,
        _ => 70,
    }
}

fn classify_status_weight(status_class: &str) -> i64 {
    match status_class {
        "failed" => 95,
        "degraded" => 78,
        "warning" => 58,
        "running" => 22,
        "success" => 8,
        "ended" => 32,
        _ => 24,
    }
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
