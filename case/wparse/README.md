# wparse case for wfusion

本目录提供 `wparse` 日志语义计算的最小可运行 case。

## 目录

- `schemas/wparse_semantic.wfs`: 输入/输出窗口定义
- `rules/wparse_semantic.wfl`: 语义规则（prepare/running/anomaly）
- `wfusion.toml`: runtime 配置（file source + sinks）
- `sinks/`: sink 路由（含 `alerts/all.jsonl`）
- `scripts/build_wparse_events.py`: 将 demo.json 或 raw wparse.log 转为 wfusion 输入 NDJSON
  - `event_time` 输出为纳秒时间戳（wfusion file source 可直接消费）

## 1) 生成输入数据

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse
python3 scripts/build_wparse_events.py \
  --input /Users/zuowenjian/devspace/wp-labs/wp-examples/analyse/wp-self/data/out_dat/demo.json \
  --output data/wparse_events.ndjson \
  --mode demo
```

如果输入是原始文本日志：

```bash
python3 scripts/build_wparse_events.py \
  --input /Users/zuowenjian/devspace/wp-labs/wp-examples/analyse/wp-self/data/in_dat/wparse.log \
  --output data/wparse_events.ndjson \
  --mode raw
```

## 2) 执行 wfusion 计算

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse
/Users/zuowenjian/devspace/wp-labs/wp-reactor/target/debug/wfusion run --config wfusion.toml
```

说明:
- 配置中同时启用了 `file` 和 `tcp` source。
- `file` source 读取 `data/wparse_events.ndjson`。
- `tcp` source 监听 `127.0.0.1:9800`，可用于外部 sender 持续送数。
- 建议优先使用 `wp-reactor/target/debug/wfusion`（已验证 file source 生效）。

输出文件：

- `alerts/semantic_alerts.jsonl`
- `alerts/all.jsonl`

样例数据（`demo.json`）下当前输出规模约为 `89` 条告警，适合时间轴分段展示。

一键执行（推荐）：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse
scripts/run_file_case.sh
```

## 3) 接入 warp-diagnose 看板

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
WARP_DIAGNOSE_USE_WFUSION=1 \
WARP_DIAGNOSE_WFUSION_ALERTS=/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/alerts/all.jsonl \
cargo run
```

## 规则说明

- 三条规则均采用 `match<target,subject:1s:fixed>`，按 `target+subject` 做 1 秒窗口聚合。
- `prepare_stage_signal`: 识别初始化/加载/分配/创建/启动聚集（close 触发）。
- `running_stage_signal`: 识别解析/运行/派发聚集（close 触发）。
- `anomaly_burst_signal`: 识别 WARN/ERROR 或错误状态聚集（close 触发）。
