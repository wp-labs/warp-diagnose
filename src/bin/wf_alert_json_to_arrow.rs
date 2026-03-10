use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, bail};
use arrow::array::{Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use serde_json::Value;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        bail!("usage: wf_alert_json_to_arrow <input.jsonl> <output.arrow>");
    }

    let output = PathBuf::from(args.pop().expect("output"));
    let input = PathBuf::from(args.pop().expect("input"));

    let file = File::open(&input).with_context(|| format!("open '{}'", input.display()))?;
    let reader = BufReader::new(file);

    let mut wfx_id = Vec::new();
    let mut rule_name = Vec::new();
    let mut score = Vec::new();
    let mut entity_type = Vec::new();
    let mut entity_id = Vec::new();
    let mut origin = Vec::new();
    let mut close_reason = Vec::new();
    let mut fired_at = Vec::new();
    let mut emit_time = Vec::new();
    let mut summary = Vec::new();

    for line in reader.lines() {
        let text = line?;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }

        let value: Value = serde_json::from_str(trimmed)
            .with_context(|| format!("parse json row from '{}'", input.display()))?;

        wfx_id.push(first_string(&value, &["wfx_id", "id"]));
        rule_name.push(first_string(&value, &["rule_name", "rule"]));
        score.push(first_f64(&value, &["score", "risk_score"]));
        entity_type.push(first_string(&value, &["entity_type", "entity_kind"]));
        entity_id.push(first_string(&value, &["entity_id", "entity", "subject"]));
        origin.push(first_string(&value, &["origin"]));
        close_reason.push(first_string(&value, &["close_reason", "status"]));
        fired_at.push(first_string(&value, &["fired_at", "emit_time", "time"]));
        emit_time.push(first_string(&value, &["emit_time", "fired_at", "time"]));
        summary.push(first_string(&value, &["summary", "message"]));
    }

    let schema = Arc::new(Schema::new(vec![
        Field::new("wfx_id", DataType::Utf8, true),
        Field::new("rule_name", DataType::Utf8, true),
        Field::new("score", DataType::Float64, true),
        Field::new("entity_type", DataType::Utf8, true),
        Field::new("entity_id", DataType::Utf8, true),
        Field::new("origin", DataType::Utf8, true),
        Field::new("close_reason", DataType::Utf8, true),
        Field::new("fired_at", DataType::Utf8, true),
        Field::new("emit_time", DataType::Utf8, true),
        Field::new("summary", DataType::Utf8, true),
    ]));

    let batch = RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(wfx_id)),
            Arc::new(StringArray::from(rule_name)),
            Arc::new(Float64Array::from(score)),
            Arc::new(StringArray::from(entity_type)),
            Arc::new(StringArray::from(entity_id)),
            Arc::new(StringArray::from(origin)),
            Arc::new(StringArray::from(close_reason)),
            Arc::new(StringArray::from(fired_at)),
            Arc::new(StringArray::from(emit_time)),
            Arc::new(StringArray::from(summary)),
        ],
    )
    .context("build alert arrow record batch")?;

    let payload = wp_arrow::ipc::encode_ipc("wf_alert", &batch)
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

fn first_f64(value: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|v| match v {
            Value::Number(n) => n.as_f64(),
            Value::String(s) => s.parse::<f64>().ok(),
            _ => None,
        })
    })
}
