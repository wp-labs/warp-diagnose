# Warp Diagnose + WFusion 执行方案

版本: v0.2  
日期: 2026-03-10  
项目: warp-diagnose

## 1. 目标
将日志计算统一交给 `wp-reactor/wfusion`，`warp-diagnose` 只负责可视化展示。

## 2. 执行模式
1. 模式 A（推荐）: 使用 `wfusion` 的 `file source` 直接读取 `case/wparse/data/wparse_events.ndjson` 并输出告警文件。
2. 模式 B（调试）: 在同一配置下保留 `tcp source`，可外部送数做联调。
3. 模式 C（回退）: 若 `wfusion` 无输出，`warp-diagnose` 自动回退到本地 `demo.json + wparse.log`。

## 3. 路径约定
1. `wp-reactor`: `/Users/zuowenjian/devspace/wp-labs/wp-reactor`
2. `warp-diagnose`: `/Users/zuowenjian/devspace/wp-labs/warp-diagnose`
3. wparse 规则 case 目录:  
   `/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse`
4. 默认 wfusion 告警文件（file source 模式产物）:  
   `/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/alerts/all.jsonl`

## 4. 模式 A（推荐）
使用一键脚本（推荐）：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse
scripts/run_file_case.sh
```

等价手工命令：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse
python3 scripts/build_wparse_events.py \
  --input /Users/zuowenjian/devspace/wp-labs/wp-examples/analyse/wp-self/data/out_dat/demo.json \
  --output data/wparse_events.ndjson \
  --mode demo
/Users/zuowenjian/devspace/wp-labs/wp-reactor/target/debug/wfusion run --config wfusion.toml
```

产物（默认）：
1. `alerts/semantic_alerts.jsonl`
2. `alerts/all.jsonl`

## 5. 模式 B（调试）
步骤 1: 启动 runtime（终端 A，沿用 `case/wparse/wfusion.toml`）

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse
/Users/zuowenjian/devspace/wp-labs/wp-reactor/target/debug/wfusion run --config wfusion.toml
```

步骤 2: 外部 sender 向 `127.0.0.1:9800` 持续送数（可选）

步骤 3: 确认 sink 输出（`alerts/all.jsonl` 持续增长）。

## 6. warp-diagnose 启动方式
在 `warp-diagnose` 目录执行：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
WARP_DIAGNOSE_USE_WFUSION=1 \
WARP_DIAGNOSE_WFUSION_ALERTS=/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/alerts/all.jsonl \
cargo run
```

### 6.1 环境变量
1. `WARP_DIAGNOSE_USE_WFUSION`
   - `1/true/yes/on`: 启用 wfusion 输入（默认启用）
   - `0/false/no/off`: 禁用 wfusion，直接使用本地日志
2. `WARP_DIAGNOSE_WFUSION_ALERTS`
   - 可传单个 `*.jsonl` 文件
   - 也可传目录；目录模式下优先读取 `all.jsonl`
3. `WARP_DIAGNOSE_DEMO_JSON`
   - 本地回退数据源（NDJSON）
4. `WARP_DIAGNOSE_WPARSE_LOG`
   - 本地回退数据源（文本日志）

## 7. 当前实现边界
1. 当前 `warp-diagnose` 已支持“读取 wfusion 输出文件”，还未实现“应用内直接拉起 wfusion 命令”。
2. 若 `wfusion` 文件读取失败或数据为 0 条，会自动回退本地源，并在状态栏显示错误与回退说明。

## 8. 验收清单
1. 页面状态区显示 `backend=wfusion` 且 `wfusion_used=true`。
2. `wfusion_rows > 0`，时间轴可见实体点与 stage 分段。
3. 点击 stage 可过滤时间段，点击点可看到下方证据详情。
4. 当 `wfusion` 路径错误时，状态区出现失败信息并触发 `local-fallback`。

## 9. wparse case（新增）
1. 规则文件: `case/wparse/rules/wparse_semantic.wfl`
2. Schema 文件: `case/wparse/schemas/wparse_semantic.wfs`
3. 运行配置: `case/wparse/wfusion.toml`
4. 数据转换脚本: `case/wparse/scripts/build_wparse_events.py`
5. 运行入口文档: `case/wparse/README.md`

## 10. 变更说明（v0.2）
1. 推荐路径从 `wp-reactor` e2e 测试产物切换为 `case/wparse` 的 `file source` 执行模式。
2. 默认告警文件路径更新为 `case/wparse/alerts/all.jsonl`。
3. 保留 `tcp source` 作为联调能力，不再作为主执行路径。
