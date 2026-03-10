use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use anyhow::{Context, bail};
use arrow::array::{
    Array, Int32Array, Int64Array, StringArray, TimestampNanosecondArray, UInt32Array,
    UInt64Array,
};
use arrow::record_batch::RecordBatch;
use chrono::NaiveDateTime;
use serde_json::json;
use warp_diagnose::arrow_frame::read_arrow_frames;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        bail!("usage: wp_arrow_to_ndjson <input.arrow> <output.ndjson>");
    }

    let output = PathBuf::from(args.pop().expect("output"));
    let input = PathBuf::from(args.pop().expect("input"));

    let frames = read_arrow_frames(&input)?;
    let file =
        File::create(&output).with_context(|| format!("create '{}'", output.display()))?;
    let mut writer = BufWriter::new(file);
    let mut rows = 0usize;

    for frame in frames {
        let batch = frame.batch;
        for row in 0..batch.num_rows() {
            if let Some(event) = arrow_row_to_event(&batch, row) {
                serde_json::to_writer(&mut writer, &event)?;
                writer.write_all(b"\n")?;
                rows += 1;
            }
        }
    }

    writer.flush()?;
    println!("wrote {rows} rows -> {}", output.display());
    Ok(())
}

fn arrow_row_to_event(batch: &RecordBatch, row: usize) -> Option<serde_json::Value> {
    let event_time = batch_timestamp_ns(batch, row, "time").or_else(|| {
        let time_text = batch_string(batch, row, "time")?;
        let ns = batch_i64(batch, row, "ns").unwrap_or(0);
        parse_to_epoch_ns(&time_text, ns)
    })?;
    let level = batch_string(batch, row, "level").unwrap_or_else(|| "INFO".to_string());
    let target = batch_string(batch, row, "target").unwrap_or_else(|| "unknown".to_string());
    let content = batch_string(batch, row, "content").unwrap_or_default();
    let subject = batch_string(batch, row, "meta.subject")
        .or_else(|| batch_string(batch, row, "subject"))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| target.clone());
    let action = batch_string(batch, row, "meta.action")
        .or_else(|| batch_string(batch, row, "action"))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| infer_action(&content));
    let status = batch_string(batch, row, "meta.status")
        .or_else(|| batch_string(batch, row, "status"))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| infer_status(&level, &content));
    let source = batch_string(batch, row, "access_source").unwrap_or_else(|| "wparse.arrow".to_string());

    Some(json!({
        "event_time": event_time,
        "level": level,
        "target": target,
        "subject": subject,
        "action": action,
        "status": status,
        "content": content,
        "source": source,
    }))
}

fn parse_to_epoch_ns(time_text: &str, ns: i64) -> Option<i64> {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(time_text) {
        return Some(dt.timestamp_nanos_opt()?);
    }

    if let Some((left, frac)) = time_text.split_once('.') {
        let dt = NaiveDateTime::parse_from_str(left, "%Y-%m-%d %H:%M:%S").ok()?;
        let frac_digits = frac
            .chars()
            .filter(|ch| ch.is_ascii_digit())
            .collect::<String>()
            .chars()
            .take(9)
            .collect::<String>();
        let extra = frac_digits
            .parse::<i64>()
            .ok()
            .unwrap_or(0)
            * 10_i64.pow((9usize.saturating_sub(frac_digits.len())) as u32);
        return Some(dt.and_utc().timestamp_nanos_opt()? + extra);
    }

    let dt = NaiveDateTime::parse_from_str(time_text, "%Y-%m-%d %H:%M:%S").ok()?;
    Some(dt.and_utc().timestamp_nanos_opt()? + ns.max(0))
}

fn infer_status(level: &str, content: &str) -> String {
    let lower = content.to_ascii_lowercase();
    if level == "ERROR"
        || level == "FATAL"
        || ["error", "fail", "exception", "timeout"]
            .iter()
            .any(|token| lower.contains(token))
    {
        return "error".to_string();
    }
    if level == "WARN" || ["warn", "pending", "miss"].iter().any(|token| lower.contains(token)) {
        return "warn".to_string();
    }
    if ["suc", "success", "done", "started", "completed"]
        .iter()
        .any(|token| lower.contains(token))
    {
        return "success".to_string();
    }
    String::new()
}

fn infer_action(content: &str) -> String {
    content
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
        .find(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .unwrap_or_else(|| "unknown".to_string())
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
    None
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
