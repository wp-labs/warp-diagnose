#!/usr/bin/env python3
"""
Build wfusion input NDJSON for case/wfusion.

Supports two input styles:
1) demo NDJSON from wp-self (recommended)
2) raw text wparse.log
"""

from __future__ import annotations

import argparse
import json
import re
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, Iterable, Iterator, Optional

CASE_DIR = Path(__file__).resolve().parent.parent
DEFAULT_IN = "../wparse/data/out_dat/demo.json"
LEGACY_DEFAULT_IN = "../wparse/data/demo.json"
DEFAULT_OUT = "data/in_dat/wparse_events.ndjson"

HEADER_RE = re.compile(
    r"^(?P<time>\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.\d+)\s+\[(?P<level>[A-Za-z]+)\s*\]\s+\[(?P<target>[^\]]+)\]\s*(?P<content>.*)$"
)


def parse_to_epoch_ns(time_text: str, ns: int = 0) -> Optional[int]:
    time_text = time_text.strip()

    # RFC3339 like: 2026-01-01T00:00:00Z
    if "T" in time_text and (time_text.endswith("Z") or "+" in time_text):
        iso = time_text.replace("Z", "+00:00")
        try:
            dt = datetime.fromisoformat(iso)
            if dt.tzinfo is None:
                dt = dt.replace(tzinfo=timezone.utc)
            return int(dt.timestamp() * 1_000_000_000)
        except ValueError:
            pass

    try:
        dt = datetime.strptime(time_text, "%Y-%m-%d %H:%M:%S")
        base = int(dt.replace(tzinfo=timezone.utc).timestamp() * 1_000_000_000)
        extra = int(ns) if ns >= 0 else 0
        return base + extra
    except ValueError:
        pass

    # 2026-01-17 18:38:51.468263000
    if " " in time_text and "." in time_text:
        left, right = time_text.split(".", 1)
        try:
            dt = datetime.strptime(left, "%Y-%m-%d %H:%M:%S")
        except ValueError:
            return None
        frac = "".join(ch for ch in right if ch.isdigit())[:9].ljust(9, "0")
        base = int(dt.replace(tzinfo=timezone.utc).timestamp() * 1_000_000_000)
        return base + int(frac)

    return None


def normalize_status(raw: str) -> str:
    s = (raw or "").strip().lower()
    if not s:
        return ""
    if s in {"suc", "ok", "success", "done"}:
        return "success"
    if s in {"fail", "failed", "err", "error", "exception"}:
        return "error"
    return s


def infer_status(level: str, content: str) -> str:
    lc = content.lower()
    if level in {"ERROR", "FATAL"} or any(k in lc for k in ["error", "fail", "exception", "timeout"]):
        return "error"
    if level == "WARN" or any(k in lc for k in ["warn", "pending", "miss"]):
        return "warn"
    if any(k in lc for k in ["suc", "success", "done", "started", "completed"]):
        return "success"
    return ""


def infer_action(content: str) -> str:
    for token in re.split(r"[^A-Za-z0-9_\-]+", content):
        if token:
            return token.lower()
    return "unknown"


def iter_demo_events(path: Path) -> Iterator[Dict[str, str]]:
    for raw in path.read_text(encoding="utf-8").splitlines():
        text = raw.strip()
        if not text:
            continue
        try:
            obj = json.loads(text)
        except json.JSONDecodeError:
            continue

        meta = obj.get("meta") or {}
        event_time = parse_to_epoch_ns(str(obj.get("time", "")), int(obj.get("ns", 0) or 0))
        if not event_time:
            continue

        level = str(obj.get("level", "INFO")).strip().upper()
        target = str(obj.get("target", "unknown")).strip() or "unknown"
        subject = str(meta.get("subject", "")).strip() or target
        action = str(meta.get("action", "")).strip().lower() or infer_action(str(obj.get("content", "")))
        status = normalize_status(str(meta.get("status", "")))
        if not status:
            status = infer_status(level, str(obj.get("content", "")))

        yield {
            "event_time": event_time,
            "level": level,
            "target": target,
            "subject": subject,
            "action": action,
            "status": status,
            "content": str(obj.get("content", "")).strip(),
            "source": str(obj.get("access_source", "demo.json")).strip() or "demo.json",
        }


def iter_raw_log_events(path: Path) -> Iterator[Dict[str, str]]:
    current: Optional[Dict[str, str]] = None

    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.rstrip("\n")
        m = HEADER_RE.match(line)
        if m:
            if current:
                yield current

            level = m.group("level").strip().upper()
            target = m.group("target").strip() or "unknown"
            content = m.group("content").strip()
            event_time = parse_to_epoch_ns(m.group("time"))
            if not event_time:
                current = None
                continue

            action = infer_action(content)
            status = infer_status(level, content)

            current = {
                "event_time": event_time,
                "level": level,
                "target": target,
                "subject": target,
                "action": action,
                "status": status,
                "content": content,
                "source": str(path),
            }
            continue

        if current and line.strip():
            current["content"] += " | " + line.strip()

    if current:
        yield current


def detect_mode(input_path: Path) -> str:
    if input_path.suffix.lower() in {".json", ".ndjson", ".jsonl"}:
        return "demo"
    return "raw"


def write_ndjson(rows: Iterable[Dict[str, str]], output: Path) -> int:
    output.parent.mkdir(parents=True, exist_ok=True)
    count = 0
    with output.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row, ensure_ascii=False) + "\n")
            count += 1
    return count


def resolve_case_path(path_text: str) -> Path:
    path = Path(path_text)
    if path.is_absolute():
        return path
    return CASE_DIR / path


def main() -> None:
    parser = argparse.ArgumentParser(description="Build wfusion input NDJSON for wparse case")
    parser.add_argument("--input", default=DEFAULT_IN, help="input file path")
    parser.add_argument("--output", default=DEFAULT_OUT, help="output ndjson path")
    parser.add_argument("--mode", choices=["auto", "demo", "raw"], default="auto", help="input parse mode")
    args = parser.parse_args()

    input_path = resolve_case_path(args.input)
    output_path = resolve_case_path(args.output)

    if not input_path.exists() and args.input == DEFAULT_IN:
        legacy_input = resolve_case_path(LEGACY_DEFAULT_IN)
        if legacy_input.exists():
            input_path = legacy_input

    if not input_path.exists():
        raise SystemExit(f"input not found: {input_path}")

    mode = args.mode
    if mode == "auto":
        mode = detect_mode(input_path)

    if mode == "demo":
        rows = iter_demo_events(input_path)
    else:
        rows = iter_raw_log_events(input_path)

    count = write_ndjson(rows, output_path)
    print(f"wrote {count} rows -> {output_path}")


if __name__ == "__main__":
    main()
