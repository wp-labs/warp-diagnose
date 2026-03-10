#!/usr/bin/env python3
"""Streamlit dashboard for NDJSON logs (legacy Python view)."""

from __future__ import annotations

import json
import math
import re
from pathlib import Path

import pandas as pd
import plotly.express as px
import plotly.graph_objects as go
import streamlit as st


def resolve_default_log_path() -> Path:
    project_root = Path(__file__).resolve().parents[1]
    candidates = [
        project_root / "case" / "wparse" / "data" / "wparse_events.ndjson",
        project_root / "case" / "wparse" / "alerts" / "all.jsonl",
        Path("/Users/zuowenjian/devspace/wp-labs/wp-examples/analyse/wp-self/data/out_dat/demo.json"),
    ]
    for c in candidates:
        if c.exists():
            return c
    return candidates[0]


DEFAULT_LOG_PATH = resolve_default_log_path()
NULL_TOKENS = {"", "(null)", "(empty)", "null", "none", "nan"}
INCIDENT_HINTS = (
    "fail",
    "error",
    "miss",
    "timeout",
    "exception",
    "panic",
    "blocked",
    "abort",
)
INCIDENT_STATUS = {
    "fail",
    "error",
    "miss",
    "timeout",
    "pending",
    "disabled",
    "blocked",
    "eof",
}
HIGH_RISK_STATUS = {
    "fatal",
    "panic",
    "error",
    "fail",
    "failed",
    "exception",
    "timeout",
    "abort",
}
MEDIUM_RISK_STATUS = {"warn", "warning", "miss", "pending", "blocked", "disabled", "eof"}
LOW_RISK_STATUS = {"suc", "success", "ok", "end", "complete", "completed", "stopped", "normal"}
PREP_ACTIONS = {"init", "load", "create", "alloc", "validate", "update", "find", "build"}
RUN_ACTIONS = {"run", "start", "parse", "receive", "dispatch", "spawn", "work", "match"}
STOP_ACTIONS = {"stop", "end", "drain", "close", "await"}
OBSERVE_ACTIONS = {"monitor", "stat", "speed", "version"}
BOUNDARY_ACTIONS = PREP_ACTIONS | RUN_ACTIONS | STOP_ACTIONS
BOUNDARY_STATUS = {"success", "suc", "fail", "error", "end", "terminal", "complete", "stopped"}
STAGE_FILTER_KEY = "timeline_stage_filter"
STAGE_TRACK_LABEL = "[Stage]"
ENTITY_POINT_KEY = "timeline_entity_point"
PLAIN_LOG_RE = re.compile(
    r"^(?P<time>\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2})\.(?P<ns>\d{1,9})\s+\[(?P<level>[A-Z]+)\s*\]\s+\[(?P<target>[^\]]+)\]\s*(?P<content>.*)$"
)


def normalize_meta_field(value: object) -> str:
    if value is None:
        return "(null)"
    text = str(value).strip()
    return text if text else "(empty)"


def normalize_token(value: object) -> str:
    text = "" if value is None else str(value).strip()
    return "" if text.lower() in NULL_TOKENS else text


def map_action_family(action: str) -> str:
    a = normalize_token(action).lower()
    if not a:
        return "unknown"
    if a in PREP_ACTIONS:
        return "prepare"
    if a in RUN_ACTIONS:
        return "running"
    if a in STOP_ACTIONS:
        return "shutdown"
    if a in OBSERVE_ACTIONS:
        return "observe"
    return "other"


def compute_event_risk(status: str, level: str, content: str) -> float:
    status_l = normalize_token(status).lower()
    level_u = (level or "").upper().strip()
    text_l = (content or "").lower()

    score = 0.10
    if level_u == "WARN":
        score = max(score, 0.55)
    elif level_u in {"ERROR", "FATAL"}:
        score = max(score, 0.85)

    if status_l in HIGH_RISK_STATUS:
        score = max(score, 0.90)
    elif status_l in MEDIUM_RISK_STATUS:
        score = max(score, 0.60)
    elif status_l in LOW_RISK_STATUS:
        score = max(score, 0.20)
    elif status_l:
        score = max(score, 0.35)

    if any(h in text_l for h in INCIDENT_HINTS):
        score = max(score, 0.70)
    if any(ok in text_l for ok in ("success", "suc", "completed", "done")):
        score = min(score, 0.35)

    return min(max(score, 0.0), 1.0)


def derive_stage_columns(df: pd.DataFrame) -> pd.DataFrame:
    if df.empty:
        return df

    ordered = df.sort_values("event_ts").copy()
    ordered["action_l"] = ordered["meta_action_c"].fillna("").astype(str).str.lower()
    ordered["status_l"] = ordered["meta_status_c"].fillna("").astype(str).str.lower()
    ordered["entity_l"] = ordered["meta_entity"].fillna("").astype(str).str.lower()
    ordered["gap_ms"] = (
        ordered["event_ts"].diff().dt.total_seconds().mul(1000).fillna(0.0).clip(lower=0.0)
    )
    p95_gap = float(ordered["gap_ms"].quantile(0.95))
    p95_gap = max(p95_gap, 1.0)
    gap_score = (ordered["gap_ms"] / (p95_gap * 3.0)).clip(0.0, 1.0)

    prev_action = ordered["action_l"].shift(1).fillna("")
    prev_entity = ordered["entity_l"].shift(1).fillna("")
    action_changed = (
        ordered["action_l"].ne("")
        & prev_action.ne("")
        & ordered["action_l"].ne(prev_action)
    ).astype(float)
    entity_changed = (
        ordered["entity_l"].ne("")
        & prev_entity.ne("")
        & ordered["entity_l"].ne(prev_entity)
    ).astype(float)
    boundary_action = ordered["action_l"].isin(BOUNDARY_ACTIONS).astype(float)
    boundary_status = ordered["status_l"].isin(BOUNDARY_STATUS).astype(float)

    ordered["stage_boundary_prob"] = (
        0.10
        + 0.35 * action_changed
        + 0.20 * boundary_action
        + 0.15 * boundary_status
        + 0.20 * gap_score
        + 0.10 * entity_changed
    ).clip(0.0, 1.0)

    raw_boundary = ordered["stage_boundary_prob"] >= 0.72
    strong_gap_boundary = gap_score >= 0.98
    candidate_boundary = raw_boundary | strong_gap_boundary

    min_segment_events = 12
    start_new = pd.Series(False, index=ordered.index)
    if len(start_new) > 0:
        first_idx = ordered.index[0]
        start_new.loc[first_idx] = True
        last_start_pos = 0
        idx_list = ordered.index.tolist()
        for pos in range(1, len(idx_list)):
            idx = idx_list[pos]
            if bool(candidate_boundary.loc[idx]) and (pos - last_start_pos) >= min_segment_events:
                start_new.loc[idx] = True
                last_start_pos = pos

    ordered["stage_id"] = start_new.cumsum().astype(int)

    stage_ids = ordered["stage_id"].unique().tolist()
    stage_rows = []
    for idx, sid in enumerate(stage_ids, start=1):
        seg = ordered[ordered["stage_id"] == sid]
        action_counts = seg["action_l"][seg["action_l"] != ""].value_counts()
        top_action_ratio = (
            float(action_counts.iloc[0]) / float(action_counts.sum())
            if not action_counts.empty
            else 0.0
        )
        fam_counts = seg["action_l"].map(map_action_family).value_counts()
        fam = fam_counts.index[0] if not fam_counts.empty else "unknown"
        if fam == "unknown":
            incident_ratio = float(seg["is_incident"].mean()) if len(seg) else 0.0
            fam = "incident" if incident_ratio >= 0.5 else "flow"

        start_prob = float(seg["stage_boundary_prob"].iloc[0])
        next_start_prob = (
            float(
                ordered[ordered["stage_id"] == stage_ids[idx]]["stage_boundary_prob"].iloc[0]
            )
            if idx < len(stage_ids)
            else start_prob
        )
        confidence = 0.50 * top_action_ratio + 0.30 * start_prob + 0.20 * next_start_prob
        confidence = max(0.0, min(1.0, confidence))

        stage_rows.append(
            {
                "stage_id": sid,
                "derived_stage": f"{idx:02d}-{fam}",
                "stage_family": fam,
                "stage_confidence": confidence,
            }
        )

    stage_df = pd.DataFrame(stage_rows)
    ordered = ordered.merge(stage_df, on="stage_id", how="left")
    return ordered.sort_index()


def build_meta_columns(df: pd.DataFrame) -> pd.DataFrame:
    out = df.copy()
    out["meta_subject_c"] = out["meta_subject"].map(normalize_token)
    out["meta_action_c"] = out["meta_action"].map(normalize_token)
    out["meta_object_c"] = out["meta_object"].map(normalize_token)
    out["meta_status_c"] = out["meta_status"].map(normalize_token)

    out["meta_entity"] = (
        out["meta_subject_c"]
        .where(out["meta_subject_c"] != "", out["target"].fillna("").astype(str))
        .where(lambda x: x != "", "unknown")
    )

    event_parts = []
    for _, row in out.iterrows():
        parts = []
        if row["meta_subject_c"]:
            parts.append(row["meta_subject_c"])
        if row["meta_action_c"]:
            parts.append(row["meta_action_c"])
        if row["meta_object_c"]:
            parts.append(row["meta_object_c"])
        if row["meta_status_c"]:
            parts.append(f"({row['meta_status_c']})")
        event_parts.append(" ".join(parts) if parts else row.get("target", "unknown"))
    out["event_type"] = event_parts

    level_upper = out["level"].fillna("").astype(str).str.upper()
    status_lower = out["meta_status_c"].fillna("").astype(str).str.lower()
    text_lower = out["content"].fillna("").astype(str).str.lower()
    out["is_incident"] = (
        level_upper.isin(["WARN", "ERROR", "FATAL"])
        | status_lower.isin(INCIDENT_STATUS)
        | text_lower.str.contains("|".join(INCIDENT_HINTS), regex=True, na=False)
    )
    out["status_risk_score"] = out.apply(
        lambda r: compute_event_risk(r.get("meta_status_c", ""), r.get("level", ""), r.get("content", "")),
        axis=1,
    )
    return derive_stage_columns(out)


@st.cache_data(show_spinner=False)
def load_ndjson(path: str, file_sig: tuple[int, int] | None = None) -> tuple[pd.DataFrame, int, str]:
    # `file_sig` participates in Streamlit cache key so file updates invalidate cache.
    _ = file_sig
    rows = []
    skipped_lines = 0

    with open(path, "r", encoding="utf-8") as f:
        raw_lines = f.read().splitlines()

    first_non_empty = next((ln.strip() for ln in raw_lines if ln.strip()), "")
    is_ndjson = first_non_empty.startswith("{")
    mode = "ndjson" if is_ndjson else "plain_log"

    if is_ndjson:
        for line in raw_lines:
            line = line.strip()
            if not line:
                continue
            try:
                item = json.loads(line)
            except json.JSONDecodeError:
                skipped_lines += 1
                continue

            meta = item.get("meta") if isinstance(item.get("meta"), dict) else {}
            rows.append(
                {
                    "wp_event_id": item.get("wp_event_id"),
                    "time": item.get("time"),
                    "ns": item.get("ns"),
                    "level": item.get("level"),
                    "target": item.get("target"),
                    "content": item.get("content"),
                    "biz": item.get("biz"),
                    "env": item.get("env"),
                    "source": item.get("wp_src_key"),
                    "meta_action": normalize_meta_field(meta.get("action")),
                    "meta_object": normalize_meta_field(meta.get("object")),
                    "meta_status": normalize_meta_field(meta.get("status")),
                    "meta_subject": normalize_meta_field(meta.get("subject")),
                }
            )
    else:
        for line in raw_lines:
            line = line.rstrip()
            if not line:
                continue
            m = PLAIN_LOG_RE.match(line)
            if not m:
                skipped_lines += 1
                continue
            ns_text = m.group("ns").strip()
            ns_text = (ns_text + "0" * 9)[:9]
            rows.append(
                {
                    "wp_event_id": None,
                    "time": m.group("time"),
                    "ns": ns_text,
                    "level": m.group("level"),
                    "target": m.group("target").strip(),
                    "content": m.group("content").strip(),
                    "biz": None,
                    "env": None,
                    "source": None,
                    "meta_action": "(null)",
                    "meta_object": "(null)",
                    "meta_status": "(null)",
                    "meta_subject": "(null)",
                }
            )

    df = pd.DataFrame(rows)
    if df.empty:
        return df, skipped_lines, mode

    df["base_ts"] = pd.to_datetime(df["time"], errors="coerce")
    ns_num = pd.to_numeric(df["ns"], errors="coerce")
    ns_num = ns_num.where(ns_num.between(0, 999_999_999), 0).fillna(0).astype("int64")
    df["event_ts"] = df["base_ts"] + pd.to_timedelta(ns_num, unit="ns")
    df["event_ts"] = df["event_ts"].where(df["event_ts"].notna(), df["base_ts"])
    return df, skipped_lines, mode


def apply_filters(df: pd.DataFrame) -> pd.DataFrame:
    st.sidebar.header("Filters")

    levels = sorted(df["level"].dropna().astype(str).unique().tolist())
    selected_levels = st.sidebar.multiselect("Level", levels, default=levels)

    targets = sorted(df["target"].dropna().astype(str).unique().tolist())
    selected_targets = st.sidebar.multiselect("Target", targets, default=targets)

    top_actions = (
        df["meta_action"].value_counts().head(12).index.tolist()
        if "meta_action" in df.columns
        else []
    )
    selected_actions = st.sidebar.multiselect("Meta action (Top 12)", top_actions, default=top_actions)

    top_subjects = (
        df["meta_subject"].value_counts().head(12).index.tolist()
        if "meta_subject" in df.columns
        else []
    )
    selected_subjects = st.sidebar.multiselect(
        "Meta subject (Top 12)", top_subjects, default=top_subjects
    )

    top_status = (
        df["meta_status"].value_counts().head(12).index.tolist()
        if "meta_status" in df.columns
        else []
    )
    selected_status = st.sidebar.multiselect("Meta status (Top 12)", top_status, default=top_status)

    top_derived_stages = (
        df["derived_stage"].value_counts().head(20).index.tolist()
        if "derived_stage" in df.columns
        else []
    )
    selected_derived_stages = st.sidebar.multiselect(
        "Derived stage (Top 20)", top_derived_stages, default=top_derived_stages
    )

    keyword = st.sidebar.text_input("Keyword in content", "")

    filtered = df.copy()
    if selected_levels:
        filtered = filtered[filtered["level"].astype(str).isin(selected_levels)]
    if selected_targets:
        filtered = filtered[filtered["target"].astype(str).isin(selected_targets)]
    if selected_actions:
        filtered = filtered[filtered["meta_action"].isin(selected_actions)]
    if selected_subjects:
        filtered = filtered[filtered["meta_subject"].isin(selected_subjects)]
    if selected_status:
        filtered = filtered[filtered["meta_status"].isin(selected_status)]
    if selected_derived_stages:
        filtered = filtered[filtered["derived_stage"].isin(selected_derived_stages)]
    if keyword.strip():
        filtered = filtered[
            filtered["content"].astype(str).str.contains(keyword.strip(), case=False, na=False)
        ]

    if filtered["event_ts"].notna().any():
        min_ts = filtered["event_ts"].min().to_pydatetime()
        max_ts = filtered["event_ts"].max().to_pydatetime()
        ts_range = st.sidebar.slider("Time range", min_value=min_ts, max_value=max_ts, value=(min_ts, max_ts))
        filtered = filtered[(filtered["event_ts"] >= ts_range[0]) & (filtered["event_ts"] <= ts_range[1])]

    return filtered


def add_scaled_marker_size(df: pd.DataFrame, count_col: str = "count") -> pd.DataFrame:
    out = df.copy()
    if out.empty:
        out["marker_size"] = []
        return out

    counts = pd.to_numeric(out[count_col], errors="coerce").fillna(0.0).clip(lower=0.0)
    log_vals = counts.map(lambda x: math.log1p(float(x)))
    p95 = float(log_vals.quantile(0.95))
    p95 = max(p95, 1e-9)
    scaled = log_vals.clip(upper=p95) / p95
    out["marker_size"] = 6.0 + scaled * 14.0
    return out


def compute_stage_summary(df: pd.DataFrame) -> pd.DataFrame:
    if df.empty or "stage_id" not in df.columns:
        return pd.DataFrame()

    summary = (
        df.groupby(["stage_id", "derived_stage"], as_index=False)
        .agg(
            start_ts=("event_ts", "min"),
            end_ts=("event_ts", "max"),
            events=("derived_stage", "size"),
            entities=("meta_entity", "nunique"),
            incidents=("is_incident", "sum"),
            stage_confidence=("stage_confidence", "mean"),
            avg_boundary_prob=("stage_boundary_prob", "mean"),
            top_action=(
                "meta_action_c",
                lambda s: s[s != ""].value_counts().index[0] if len(s[s != ""]) else "",
            ),
        )
        .sort_values("stage_id")
    )
    summary["duration_ms"] = (
        (summary["end_ts"] - summary["start_ts"]).dt.total_seconds().mul(1000).round(1)
    )
    summary["stage_confidence_pct"] = (summary["stage_confidence"] * 100).round(1)
    summary["avg_boundary_pct"] = (summary["avg_boundary_prob"] * 100).round(1)
    return summary


def compute_turning_points(df: pd.DataFrame) -> dict:
    if df.empty or not df["event_ts"].notna().any():
        return {}

    bucket = df.dropna(subset=["event_ts"]).copy()
    bucket["bucket_ts"] = bucket["event_ts"].dt.floor("200ms")
    agg = (
        bucket.groupby("bucket_ts", as_index=False)
        .agg(
            incident_cnt=("is_incident", "sum"),
            risk_max=("status_risk_score", "max"),
            stage=("derived_stage", lambda s: s.mode().iloc[0] if len(s.mode()) else ""),
        )
        .sort_values("bucket_ts")
    )
    if agg.empty:
        return {}

    out: dict[str, dict] = {}
    with_inc = agg[agg["incident_cnt"] > 0]
    if not with_inc.empty:
        first = with_inc.iloc[0]
        out["first_incident"] = {
            "ts": first["bucket_ts"],
            "incident_cnt": int(first["incident_cnt"]),
            "stage": str(first["stage"]),
        }

        peak = with_inc.sort_values(["incident_cnt", "risk_max"], ascending=[False, False]).iloc[0]
        out["peak_incident"] = {
            "ts": peak["bucket_ts"],
            "incident_cnt": int(peak["incident_cnt"]),
            "risk_max": float(peak["risk_max"]),
            "stage": str(peak["stage"]),
        }

        after_peak = agg[agg["bucket_ts"] > peak["bucket_ts"]].copy()
        recovery = after_peak[(after_peak["incident_cnt"] == 0) & (after_peak["risk_max"] < 0.35)]
        if not recovery.empty:
            r = recovery.iloc[0]
            out["recovery"] = {
                "ts": r["bucket_ts"],
                "stage": str(r["stage"]),
            }
    return out


def render_main_narrative(df: pd.DataFrame) -> None:
    st.subheader("Main Narrative")
    if df.empty or not df["event_ts"].notna().any():
        st.info("No narrative data.")
        return

    stage_summary = compute_stage_summary(df)
    turning = compute_turning_points(df)

    start_ts = df["event_ts"].min()
    end_ts = df["event_ts"].max()
    lines = [
        (
            f"{start_ts} 到 {end_ts} 共发生 {len(df):,} 条事件，"
            f"涉及 {df['meta_entity'].nunique()} 个实体，识别到 {df['derived_stage'].nunique()} 个阶段。"
        )
    ]

    for _, row in stage_summary.head(5).iterrows():
        top_action = row["top_action"] if row["top_action"] else "-"
        lines.append(
            f"{row['derived_stage']} 阶段持续 {row['duration_ms']:.0f}ms，主动作 `{top_action}`，异常 {int(row['incidents'])}。"
        )

    first = turning.get("first_incident")
    if first:
        lines.append(
            f"首个异常出现在 {first['ts']}（{first['stage']}，每桶异常 {first['incident_cnt']}）。"
        )
    peak = turning.get("peak_incident")
    if peak:
        lines.append(
            f"异常峰值在 {peak['ts']}（{peak['stage']}，每桶异常 {peak['incident_cnt']}，风险峰值 {peak['risk_max']:.2f}）。"
        )
    rec = turning.get("recovery")
    if rec:
        lines.append(f"恢复点在 {rec['ts']}（{rec['stage']}），异常降至 0 且风险回落。")

    for idx, line in enumerate(lines[:8], start=1):
        st.markdown(f"{idx}. {line}")


def pick_anchor_incident(df: pd.DataFrame) -> tuple[pd.Series | None, str]:
    if df.empty:
        return None, ""

    point = st.session_state.get(ENTITY_POINT_KEY)
    if isinstance(point, dict):
        bucket_start = pd.to_datetime(point.get("bucket_start"), errors="coerce")
        entity = str(point.get("entity", "")).strip()
        if pd.notna(bucket_start) and entity:
            bucket_end = bucket_start + pd.Timedelta(milliseconds=200)
            cand = df[
                (df["meta_entity"] == entity)
                & (df["event_ts"] >= bucket_start)
                & (df["event_ts"] < bucket_end)
                & (df["is_incident"])
            ].copy()
            if not cand.empty:
                row = cand.sort_values(["status_risk_score", "event_ts"], ascending=[False, True]).iloc[0]
                return row, "selected_point"

    incidents = df[df["is_incident"]].copy()
    if incidents.empty:
        return None, ""
    row = incidents.sort_values(["status_risk_score", "event_ts"], ascending=[False, True]).iloc[0]
    return row, "top_incident"


def render_causal_chain(df: pd.DataFrame) -> None:
    st.subheader("Causal Chain")
    anchor, source = pick_anchor_incident(df)
    if anchor is None:
        st.info("No incident found for causal chain.")
        return

    anchor_ts = anchor["event_ts"]
    before = df[(df["event_ts"] < anchor_ts) & (df["event_ts"] >= anchor_ts - pd.Timedelta(seconds=1))].copy()
    incident = df[
        (df["event_ts"] >= anchor_ts - pd.Timedelta(milliseconds=100))
        & (df["event_ts"] <= anchor_ts + pd.Timedelta(milliseconds=100))
    ].copy()
    after = df[(df["event_ts"] > anchor_ts) & (df["event_ts"] <= anchor_ts + pd.Timedelta(seconds=1))].copy()

    st.caption(
        f"anchor={anchor_ts} | entity={anchor['meta_entity']} | source={source} | risk={float(anchor['status_risk_score']):.2f}"
    )
    col1, col2, col3 = st.columns(3)
    with col1:
        st.markdown("**Before (-1s ~ 0s)**")
        st.dataframe(
            before[["event_ts", "meta_entity", "meta_action", "meta_status", "content"]]
            .sort_values("event_ts", ascending=False)
            .head(25),
            use_container_width=True,
            height=220,
        )
    with col2:
        st.markdown("**Incident (±100ms)**")
        st.dataframe(
            incident[["event_ts", "meta_entity", "meta_action", "meta_status", "status_risk_score", "content"]]
            .sort_values("event_ts", ascending=False)
            .head(25),
            use_container_width=True,
            height=220,
        )
    with col3:
        st.markdown("**After (0s ~ +1s)**")
        st.dataframe(
            after[["event_ts", "meta_entity", "meta_action", "meta_status", "content"]]
            .sort_values("event_ts")
            .head(25),
            use_container_width=True,
            height=220,
        )


def get_active_stage_filter() -> dict | None:
    value = st.session_state.get(STAGE_FILTER_KEY)
    return value if isinstance(value, dict) else None


def apply_active_stage_filter(df: pd.DataFrame) -> pd.DataFrame:
    active = get_active_stage_filter()
    if not active or df.empty:
        return df

    start_ts = pd.to_datetime(active.get("start_ts"), errors="coerce")
    end_ts = pd.to_datetime(active.get("end_ts"), errors="coerce")
    if pd.isna(start_ts) or pd.isna(end_ts):
        return df
    return df[(df["event_ts"] >= start_ts) & (df["event_ts"] <= end_ts)]


def render_active_stage_filter_banner() -> None:
    active = get_active_stage_filter()
    if not active:
        return
    col_info, col_btn = st.columns([5, 1])
    label = active.get("label", "unknown")
    start_ts = active.get("start_ts", "")
    end_ts = active.get("end_ts", "")
    col_info.info(f"Stage-linked filter: `{label}`  ({start_ts} ~ {end_ts})")
    if col_btn.button("Clear Stage Filter"):
        st.session_state.pop(STAGE_FILTER_KEY, None)
        st.rerun()


def render_active_entity_point_banner() -> None:
    point = st.session_state.get(ENTITY_POINT_KEY)
    if not isinstance(point, dict):
        return
    col_info, col_btn = st.columns([5, 1])
    entity = point.get("entity", "unknown")
    bucket_start = point.get("bucket_start", "")
    col_info.info(f"Point-linked detail: entity=`{entity}` bucket_start=`{bucket_start}`")
    if col_btn.button("Clear Point Detail"):
        st.session_state.pop(ENTITY_POINT_KEY, None)
        st.rerun()


def update_stage_filter_from_selection(event: object) -> None:
    points = []
    if event is None:
        return
    try:
        points = event.selection.get("points", [])
    except Exception:  # noqa: BLE001
        try:
            points = event["selection"]["points"]
        except Exception:  # noqa: BLE001
            points = []
    if not points:
        return

    for point in points:
        custom = point.get("customdata") if isinstance(point, dict) else None
        if not custom or len(custom) < 3:
            continue
        if custom[0] != "stage_selector":
            if custom[0] == "entity_point":
                entity = str(custom[1])
                bucket_start = str(custom[2])
                risk_max = float(custom[3]) if len(custom) > 3 else 0.0
                count = int(custom[4]) if len(custom) > 4 else 0
                new_point = {
                    "entity": entity,
                    "bucket_start": bucket_start,
                    "risk_max": risk_max,
                    "count": count,
                }
                if st.session_state.get(ENTITY_POINT_KEY) != new_point:
                    st.session_state[ENTITY_POINT_KEY] = new_point
                return
            continue

        label = str(custom[1])
        start_ts = str(custom[2])
        end_ts = str(custom[3])
        stage_id = int(custom[4]) if len(custom) > 4 else 0
        new_value = {
            "label": label,
            "start_ts": start_ts,
            "end_ts": end_ts,
            "stage_id": stage_id,
        }
        if st.session_state.get(STAGE_FILTER_KEY) != new_value:
            st.session_state[STAGE_FILTER_KEY] = new_value
            st.rerun()
        return


def render_point_related_data(df: pd.DataFrame) -> None:
    point = st.session_state.get(ENTITY_POINT_KEY)
    if not isinstance(point, dict):
        return
    if df.empty:
        return

    bucket_start = pd.to_datetime(point.get("bucket_start"), errors="coerce")
    entity = str(point.get("entity", "")).strip()
    if pd.isna(bucket_start) or not entity:
        return
    bucket_end = bucket_start + pd.Timedelta(milliseconds=200)

    rel = df[
        (df["meta_entity"] == entity)
        & (df["event_ts"] >= bucket_start)
        & (df["event_ts"] < bucket_end)
    ].copy()

    st.markdown("**Related Data (Selected Point)**")
    st.caption(
        f"entity={entity} | bucket={bucket_start} ~ {bucket_end} | points={point.get('count', 0)} | risk_max={point.get('risk_max', 0):.2f}"
    )
    if rel.empty:
        st.info("No rows found for the selected point under current filters.")
        return

    st.dataframe(
        rel[
            [
                "event_ts",
                "level",
                "meta_entity",
                "derived_stage",
                "meta_action",
                "meta_status",
                "status_risk_score",
                "content",
                "source",
                "wp_event_id",
            ]
        ].sort_values("event_ts", ascending=False),
        use_container_width=True,
        height=260,
    )


def plot_stage_entity_timeline(df: pd.DataFrame) -> None:
    st.subheader("Stage + Entity Timeline")
    if df.empty or not df["event_ts"].notna().any():
        st.info("No timeline data.")
        return

    stage_segments = compute_stage_summary(df)
    top_entities = df["meta_entity"].value_counts().head(15).index.tolist()

    entity_line = (
        df[df["meta_entity"].isin(top_entities)]
        .dropna(subset=["event_ts"])
        .groupby([pd.Grouper(key="event_ts", freq="200ms"), "meta_entity"], as_index=False)
        .agg(
            count=("meta_entity", "size"),
            risk_avg=("status_risk_score", "mean"),
            risk_max=("status_risk_score", "max"),
            level_top=(
                "level",
                lambda s: s.mode().iloc[0] if len(s.mode()) else (s.iloc[0] if len(s) else ""),
            ),
        )
    )
    entity_line = add_scaled_marker_size(entity_line)

    fig = go.Figure()
    turning = compute_turning_points(df)

    band_colors = (
        px.colors.qualitative.Set3
        + px.colors.qualitative.Pastel
        + px.colors.qualitative.Light24
    )
    for idx, row in stage_segments.iterrows():
        color = band_colors[idx % len(band_colors)]
        fig.add_vrect(
            x0=row["start_ts"],
            x1=row["end_ts"],
            fillcolor=color,
            opacity=0.16,
            line_width=0,
            layer="below",
        )
        span_ms = (row["end_ts"] - row["start_ts"]).total_seconds() * 1000.0
        if span_ms >= 120:
            mid = row["start_ts"] + (row["end_ts"] - row["start_ts"]) / 2
            fig.add_annotation(
                x=mid,
                y=1.08,
                yref="paper",
                text=f"{row['derived_stage']} ({row['stage_confidence'] * 100:.0f}%)",
                showarrow=False,
                font=dict(size=10),
            )

    if not stage_segments.empty:
        selector = stage_segments.copy()
        selector["mid_ts"] = selector["start_ts"] + (selector["end_ts"] - selector["start_ts"]) / 2
        selector["start_ts_s"] = selector["start_ts"].dt.strftime("%Y-%m-%d %H:%M:%S.%f")
        selector["end_ts_s"] = selector["end_ts"].dt.strftime("%Y-%m-%d %H:%M:%S.%f")
        selector["track"] = STAGE_TRACK_LABEL
        selector_customdata = (
            selector.apply(
                lambda r: [
                    "stage_selector",
                    r["derived_stage"],
                    r["start_ts_s"],
                    r["end_ts_s"],
                    int(r["stage_id"]),
                ],
                axis=1,
            )
            .tolist()
        )
        fig.add_trace(
            go.Scatter(
                x=selector["mid_ts"],
                y=selector["track"],
                mode="markers",
                marker=dict(size=14, color="#111111", symbol="diamond-open"),
                customdata=selector_customdata,
                hovertemplate=(
                    "click to filter<br>stage=%{customdata[1]}<br>"
                    "start=%{customdata[2]}<br>end=%{customdata[3]}<extra></extra>"
                ),
                name="stage-selector",
                showlegend=False,
            )
        )

    if not entity_line.empty:
        entity_line = entity_line.copy()
        entity_line["event_ts_s"] = entity_line["event_ts"].dt.strftime("%Y-%m-%d %H:%M:%S.%f")
        entity_customdata = (
            entity_line.apply(
                lambda r: [
                    "entity_point",
                    r["meta_entity"],
                    r["event_ts_s"],
                    float(r["risk_max"]),
                    int(r["count"]),
                ],
                axis=1,
            )
            .tolist()
        )
        fig.add_trace(
            go.Scatter(
                x=entity_line["event_ts"],
                y=entity_line["meta_entity"],
                mode="markers",
                marker=dict(
                    size=entity_line["marker_size"],
                    color=entity_line["risk_max"],
                    cmin=0.0,
                    cmax=1.0,
                    colorscale=[
                        [0.00, "#2ca02c"],
                        [0.50, "#ffbf00"],
                        [1.00, "#d62728"],
                    ],
                    colorbar=dict(title="risk"),
                    line=dict(color="#334155", width=0.4),
                    opacity=0.78,
                ),
                customdata=entity_customdata,
                hovertemplate=(
                    "time=%{x}<br>entity=%{y}<br>"
                    "count=%{customdata[4]}<br>"
                    "risk_max=%{customdata[3]:.2f}<extra></extra>"
                ),
                name="entity",
                showlegend=False,
            )
        )

    tp_style = {
        "first_incident": ("#d62728", "首个异常"),
        "peak_incident": ("#8b0000", "异常峰值"),
        "recovery": ("#2ca02c", "恢复点"),
    }
    for key, meta in turning.items():
        if key not in tp_style:
            continue
        color, label = tp_style[key]
        ts = meta.get("ts")
        if ts is None:
            continue
        fig.add_vline(x=ts, line_color=color, line_width=1.5, line_dash="dash")
        fig.add_annotation(
            x=ts,
            y=1.14,
            yref="paper",
            text=label,
            showarrow=False,
            font=dict(size=10, color=color),
        )

    fig.update_layout(
        height=620,
        title="Unified Timeline: Stage Bands (left→right) + Entity Events",
        margin=dict(l=20, r=20, t=86, b=20),
    )
    fig.update_xaxes(title_text="event_ts")
    fig.update_yaxes(
        title_text="meta_entity",
        categoryorder="array",
        categoryarray=[STAGE_TRACK_LABEL, *top_entities[::-1]],
        autorange="reversed",
    )
    st.caption("背景色块代表 stage 时间段；点大小=事件数（log），点颜色=风险值（绿低红高）；虚线表示转折点（首个异常/峰值/恢复）；点击 `[Stage]` 轨道上的菱形可联动过滤到该阶段。")
    event = st.plotly_chart(
        fig,
        use_container_width=True,
        key="stage_entity_timeline",
        on_select="rerun",
        selection_mode=("points",),
    )
    update_stage_filter_from_selection(event)
    render_active_entity_point_banner()
    render_point_related_data(df)


def plot_story_view(df: pd.DataFrame) -> None:
    st.subheader("Meta Story")
    if df.empty:
        st.info("No data in current filter.")
        return

    story = (
        df.groupby(["meta_entity", "event_type"], as_index=False)
        .agg(
            count=("event_type", "size"),
            first_ts=("event_ts", "min"),
            last_ts=("event_ts", "max"),
            warn_cnt=("level", lambda s: int((s == "WARN").sum())),
            sample=("content", "first"),
        )
        .sort_values(["first_ts", "count"], ascending=[True, False])
    )
    st.dataframe(
        story[
            ["first_ts", "last_ts", "meta_entity", "event_type", "count", "warn_cnt", "sample"]
        ].head(80),
        use_container_width=True,
        height=320,
    )


def plot_derived_stage_view(df: pd.DataFrame) -> None:
    st.subheader("Derived Stage (Probabilistic)")
    if df.empty or "derived_stage" not in df.columns:
        st.info("No stage data.")
        return

    st.caption(
        "stage_boundary_prob 表示单条事件是阶段边界的概率；stage_confidence 表示该阶段命名可信度。"
    )

    stage_summary = compute_stage_summary(df)

    st.markdown("**Stage Cards**")
    card_rows = stage_summary.head(6).to_dict("records")
    if card_rows:
        cols = st.columns(min(3, len(card_rows)))
        for i, row in enumerate(card_rows):
            col = cols[i % len(cols)]
            duration_txt = f"{row['duration_ms']:.0f} ms" if pd.notna(row["duration_ms"]) else "-"
            top_action = row["top_action"] if row["top_action"] else "(none)"
            with col:
                st.markdown(
                    f"**{row['derived_stage']}**  \n"
                    f"主动作: `{top_action}`  \n"
                    f"异常数: `{int(row['incidents'])}`  \n"
                    f"持续时长: `{duration_txt}`"
                )

    st.dataframe(
        stage_summary[
            [
                "stage_id",
                "derived_stage",
                "start_ts",
                "end_ts",
                "events",
                "entities",
                "incidents",
                "top_action",
                "duration_ms",
                "stage_confidence_pct",
                "avg_boundary_pct",
            ]
        ],
        use_container_width=True,
        height=260,
    )


def plot_incident_view(df: pd.DataFrame) -> None:
    st.subheader("Incident Feed")
    incidents = df[df["is_incident"]].copy()
    if incidents.empty:
        st.success("No incidents detected under current filter.")
        return

    summary = (
        incidents.groupby(["meta_entity", "event_type"], as_index=False)
        .agg(
            count=("event_type", "size"),
            first_ts=("event_ts", "min"),
            last_ts=("event_ts", "max"),
            levels=("level", lambda s: ",".join(sorted(set(s.astype(str))))),
            sample=("content", "first"),
        )
        .sort_values(["count", "last_ts"], ascending=[False, False])
    )

    fig_incident = px.bar(
        summary.head(20),
        x="count",
        y="event_type",
        color="meta_entity",
        orientation="h",
        title="Top Incident Types",
    )
    st.plotly_chart(fig_incident, use_container_width=True)

    st.dataframe(
        summary[
            ["first_ts", "last_ts", "meta_entity", "event_type", "levels", "count", "sample"]
        ].head(80),
        use_container_width=True,
        height=320,
    )


def plot_dashboard(df: pd.DataFrame) -> None:
    render_active_stage_filter_banner()
    view_df = apply_active_stage_filter(df)

    if view_df.empty:
        st.warning("No rows in active stage-linked time range.")
        return

    col1, col2, col3, col4, col5, col6, col7 = st.columns(7)
    warn_cnt = int((view_df["level"] == "WARN").sum())
    info_cnt = int((view_df["level"] == "INFO").sum())
    incident_cnt = int(view_df["is_incident"].sum()) if "is_incident" in view_df.columns else 0
    stage_cnt = int(view_df["derived_stage"].nunique()) if "derived_stage" in view_df.columns else 0
    avg_risk = float(view_df["status_risk_score"].mean()) if "status_risk_score" in view_df.columns else 0.0
    col1.metric("Rows", f"{len(view_df):,}")
    col2.metric("WARN", f"{warn_cnt:,}")
    col3.metric("INFO", f"{info_cnt:,}")
    col4.metric("Unique Subjects", f"{view_df['meta_subject'].nunique():,}")
    col5.metric("Incidents", f"{incident_cnt:,}")
    col6.metric("Derived Stages", f"{stage_cnt:,}")
    col7.metric("Avg Risk", f"{avg_risk:.2f}")

    plot_stage_entity_timeline(view_df)
    render_main_narrative(view_df)
    render_causal_chain(view_df)
    plot_derived_stage_view(view_df)
    plot_story_view(view_df)
    plot_incident_view(view_df)

    trend = (
        view_df.dropna(subset=["event_ts"])
        .groupby([pd.Grouper(key="event_ts", freq="200ms"), "level"], as_index=False)
        .size()
        .rename(columns={"size": "count"})
    )
    fig_trend = px.area(
        trend,
        x="event_ts",
        y="count",
        color="level",
        title="Event Trend (200ms buckets)",
    )
    st.plotly_chart(fig_trend, use_container_width=True)

    col_left, col_right = st.columns(2)
    level_dist = (
        view_df.groupby("level", as_index=False).size().rename(columns={"size": "count"}).sort_values("count", ascending=False)
    )
    fig_level = px.bar(level_dist, x="level", y="count", title="Level Distribution")
    col_left.plotly_chart(fig_level, use_container_width=True)

    target_dist = (
        view_df.groupby("target", as_index=False).size().rename(columns={"size": "count"}).sort_values("count", ascending=False)
    )
    fig_target = px.bar(target_dist, x="target", y="count", title="Target Distribution")
    col_right.plotly_chart(fig_target, use_container_width=True)

    col_left2, col_right2 = st.columns(2)
    action_top = (
        view_df.groupby("meta_action", as_index=False).size().rename(columns={"size": "count"}).sort_values("count", ascending=False).head(15)
    )
    fig_action = px.bar(action_top, x="count", y="meta_action", orientation="h", title="Top Meta Actions")
    col_left2.plotly_chart(fig_action, use_container_width=True)

    subject_top = (
        view_df.groupby("meta_subject", as_index=False).size().rename(columns={"size": "count"}).sort_values("count", ascending=False).head(15)
    )
    fig_subject = px.bar(subject_top, x="count", y="meta_subject", orientation="h", title="Top Meta Subjects")
    col_right2.plotly_chart(fig_subject, use_container_width=True)

    st.subheader("Event Detail")
    st.dataframe(
        view_df[
            [
                "event_ts",
                "level",
                "target",
                "meta_entity",
                "event_type",
                "derived_stage",
                "stage_confidence",
                "stage_boundary_prob",
                "status_risk_score",
                "meta_action",
                "meta_subject",
                "meta_status",
                "content",
                "source",
                "wp_event_id",
            ]
        ].sort_values("event_ts", ascending=False),
        use_container_width=True,
        height=480,
    )


def main() -> None:
    st.set_page_config(page_title="Demo Log Dashboard", layout="wide")
    st.title("Demo JSON Log Dashboard")

    default_path = str(DEFAULT_LOG_PATH)
    log_path = st.sidebar.text_input("NDJSON file path", default_path)
    if st.sidebar.button("Reload File"):
        st.cache_data.clear()
        st.rerun()

    path = Path(log_path)

    if not path.exists():
        st.error(f"File not found: {path}")
        return

    stat = path.stat()
    file_sig = (stat.st_mtime_ns, stat.st_size)
    st.sidebar.caption(f"mtime_ns={file_sig[0]}  size={file_sig[1]} bytes")

    try:
        df, bad_lines, mode = load_ndjson(str(path), file_sig)
    except Exception as exc:  # noqa: BLE001
        st.exception(exc)
        return

    st.sidebar.caption(f"format={mode}")
    if bad_lines:
        st.warning(f"Skipped {bad_lines} non-event line(s).")

    if df.empty:
        st.warning("No valid rows parsed from file.")
        return

    enriched = build_meta_columns(df)
    filtered = apply_filters(enriched)
    plot_dashboard(filtered)


if __name__ == "__main__":
    main()
