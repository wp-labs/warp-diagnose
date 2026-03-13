# Warp Diagnose + WFusion 执行方案

版本: v0.2  
日期: 2026-03-10  
项目: warp-diagnose

## 1. 目标
将日志计算统一交给 `wp-reactor/wfusion`，`warp-diagnose` 只负责可视化展示。

## 2. 执行模式
1. 模式 A（推荐）: `batch` 模式，使用 `file source` 直接读取 `case/wfusion/data/in_dat/wparse_events.ndjson` 并输出告警文件。
2. 模式 B（调试）: `daemon` 模式，需要单独准备带 TCP source 的配置文件做联调。
3. 模式 C（回退）: 若 `wfusion` 无输出，`warp-diagnose` 自动回退到本地 `demo.json + wparse.log`。

## 3. 路径约定
1. `wp-reactor`: `/Users/zuowenjian/devspace/wp-labs/wp-reactor`
2. `warp-diagnose`: `/Users/zuowenjian/devspace/wp-labs/warp-diagnose`
3. wparse 工作根目录:  
   `/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse`
4. wfusion 工作根目录:  
   `/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wfusion`
5. 默认 wfusion 告警文件（file source 模式产物）:  
   `/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wfusion/alerts/wf-alert.arrow`

## 4. 模式 A（推荐 / batch）
使用一键脚本（推荐）：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
case/scripts/run_wp_wf_case.sh
```

等价手工命令：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
cargo run --quiet --bin wp_arrow_to_ndjson -- \
  case/wparse/data/out_dat/wp-log.arrow \
  case/wfusion/data/in_dat/wparse_events.ndjson
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wfusion
/Users/zuowenjian/devspace/wp-labs/wp-reactor/target/debug/wfusion run --config wfusion.toml
```

产物（默认）：
1. `alerts/wf-semantic.jsonl`
2. `alerts/wf-alert.jsonl`
3. `alerts/wf-semantic.arrow`
4. `alerts/wf-alert.arrow`

## 5. 模式 B（调试 / daemon）
1. 当前仓库内提供的 `case/wfusion/wfusion.toml` 是 `batch` 模式，不包含 TCP source。
2. 若要做 daemon 联调，需要单独准备一个 `mode = "daemon"` 的配置文件。
3. 该配置必须至少启用一个 TCP source，且不能直接复用当前 batch 配置。

## 6. warp-diagnose 启动方式
在 `warp-diagnose` 目录执行：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
WARP_DIAGNOSE_USE_WFUSION=1 \
WARP_DIAGNOSE_WFUSION_ALERTS=/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wfusion/alerts/wf-alert.arrow \
cargo run
```

### 6.1 环境变量
1. `WARP_DIAGNOSE_USE_WFUSION`
   - `1/true/yes/on`: 启用 wfusion 输入（默认启用）
   - `0/false/no/off`: 禁用 wfusion，直接使用本地日志
2. `WARP_DIAGNOSE_WFUSION_ALERTS`
   - 可传单个 `*.arrow` 或 `*.jsonl` 文件
   - 也可传目录；目录模式下优先读取 `wf-alert.jsonl`
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

## 9. case 目录拆分
1. `wparse` 工作根: `case/wparse`
2. `wfusion` 规则文件: `case/wfusion/rules/wparse_semantic.wfl`
3. `wfusion` Schema 文件: `case/wfusion/schemas/wparse_semantic.wfs`
4. `wfusion` 运行配置: `case/wfusion/wfusion.toml`（当前为 `batch` 模式）
5. `wparse` Arrow 日志输出: `case/wparse/data/out_dat/wp-log.arrow`
6. `wfusion` 输入中间文件: `case/wfusion/data/in_dat/wparse_events.ndjson`
7. 联调入口文档: `case/README.md`

## 10. 变更说明（v0.2）
1. 推荐路径从 `wp-reactor` e2e 测试产物切换为 `case/wfusion` 的 `file source` 执行模式。
2. 默认告警文件路径更新为 `case/wfusion/alerts/wf-alert.arrow`。
3. `daemon` 模式改为单独配置，不再和当前 `batch` 主配置混放。
