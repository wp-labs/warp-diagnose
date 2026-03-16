use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, bail};
use arrow::array::{Float64Array, Int64Array, StringArray, TimestampNanosecondArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use chrono::NaiveDateTime;
use serde_json::Value;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        bail!("usage: wp_log_json_to_arrow <input.json> <output.arrow>");
    }

    let output = PathBuf::from(args.pop().expect("output"));
    let input = PathBuf::from(args.pop().expect("input"));

    let file = File::open(&input).with_context(|| format!("open '{}'", input.display()))?;
    let reader = BufReader::new(file);

    let mut event_time = Vec::new();
    let mut level = Vec::new();
    let mut target = Vec::new();
    let mut subject = Vec::new();
    let mut action = Vec::new();
    let mut status = Vec::new();
    let mut action_class = Vec::new();
    let mut status_class = Vec::new();
    let mut impl_importance = Vec::new();
    let mut action_weight = Vec::new();
    let mut status_weight = Vec::new();
    let mut risk_score = Vec::new();
    let mut content = Vec::new();
    let mut source = Vec::new();

    for line in reader.lines() {
        let text = line?;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }

        let value: Value = serde_json::from_str(trimmed)
            .with_context(|| format!("parse json row from '{}'", input.display()))?;

        event_time.push(first_time_ns(&value, &["event_time", "time"]));
        level.push(first_string(&value, &["level"]));
        target.push(first_string(&value, &["target"]));
        subject.push(first_string(&value, &["subject", "subject_raw"]).or_else(|| first_string(&value, &["target"])));
        action.push(first_string(&value, &["action"]));
        status.push(first_string(&value, &["status"]));
        action_class.push(first_string(&value, &["action_class"]));
        status_class.push(first_string(&value, &["status_class"]));
        impl_importance.push(first_i64(&value, &["impl_importance"]));
        action_weight.push(first_i64(&value, &["action_weight"]));
        status_weight.push(first_i64(&value, &["status_weight"]));
        risk_score.push(first_f64(&value, &["risk_score"]));
        content.push(first_string(&value, &["content"]));
        source.push(first_string(&value, &["source", "access_source"]));
    }

    let schema = Arc::new(Schema::new(vec![
        Field::new("event_time", DataType::Timestamp(arrow::datatypes::TimeUnit::Nanosecond, None), true),
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
            Arc::new(TimestampNanosecondArray::from(event_time)),
            Arc::new(StringArray::from(level)),
            Arc::new(StringArray::from(target)),
            Arc::new(StringArray::from(subject)),
            Arc::new(StringArray::from(action)),
            Arc::new(StringArray::from(status)),
            Arc::new(StringArray::from(action_class)),
            Arc::new(StringArray::from(status_class)),
            Arc::new(Int64Array::from(impl_importance)),
            Arc::new(Int64Array::from(action_weight)),
            Arc::new(Int64Array::from(status_weight)),
            Arc::new(Float64Array::from(risk_score)),
            Arc::new(StringArray::from(content)),
            Arc::new(StringArray::from(source)),
        ],
    )
    .context("build wparse arrow record batch")?;

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

    println!("wrote {} rows -> {}", batch.num_rows(), output.display());
    Ok(())
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|v| match v {
            Value::String(s) => Some(s.to_string()),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        })
    })
}

fn first_i64(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|v| match v {
            Value::Number(n) => n.as_i64(),
            Value::String(s) => s.parse::<i64>().ok(),
            _ => None,
        })
    })
}

fn first_f64(value: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|v| match v {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.parse::<f64>().ok(),
            _ => None,
        })
    })
}

fn first_time_ns(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|v| match v {
            Value::String(s) => parse_to_epoch_ns(s),
            Value::Number(n) => n.as_i64(),
            _ => None,
        })
    })
}

fn parse_to_epoch_ns(time_text: &str) -> Option<i64> {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(time_text) {
        return dt.timestamp_nanos_opt();
    }

    if let Some((left, frac)) = time_text.split_once('.') {
        let dt = NaiveDateTime::parse_from_str(left, "%Y-%m-%d %H:%M:%S").ok()?;
        let frac_digits = frac
            .chars()
            .filter(|ch| ch.is_ascii_digit())
            .take(9)
            .collect::<String>();
        let extra = if frac_digits.is_empty() {
            0
        } else {
            frac_digits.parse::<i64>().ok().unwrap_or(0)
                * 10_i64.pow((9usize.saturating_sub(frac_digits.len())) as u32)
        };
        return Some(dt.and_utc().timestamp_nanos_opt()? + extra);
    }

    let dt = NaiveDateTime::parse_from_str(time_text, "%Y-%m-%d %H:%M:%S").ok()?;
    dt.and_utc().timestamp_nanos_opt()
}
