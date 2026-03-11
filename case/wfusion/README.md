# wfusion Case

本目录只承载 `wfusion` 相关内容：

- `wfusion.toml`: 当前提供的是 `batch` 模式 runtime 配置
- `schemas/`, `rules/`, `sinks/`: 规则与输出路由
- `alerts/`: `wf` 告警产物
- `logs/`: runtime 日志
- `UPSTREAM_DEFECT_CLOSE_AGGREGATION.md`: 已修正的上游 close 聚合求值问题记录

语义约定：

- `wp` = log
- `wf` = alert

当前执行关系：

1. `case/wparse` 直接产出 `wp-log.arrow`
2. `wparse` 侧 OML 已直接产出 `wfusion` 期望字段 schema
3. 顶层脚本仅将同一个 `wp-log.arrow` 复制到 `data/in_dat/wp-log.arrow`
4. `wfusion` 以 `mode = "batch"` 直接消费该 Arrow 文件
5. source 配置使用 `format = "arrow_framed"`，并显式覆盖 `stream = "wparse"`
6. `wfusion` 当前稳定写出：
   - `alerts/wf-alert.arrow`
   - `alerts/wf-alert.json`
7. `alerts/wf-semantic.arrow` 当前仍会被创建为空文件：
   - `semantic_events` 是中间 window
   - 中间 target 会写入 window，不会直接走最终 sink
   - 因此它现在不是可消费的业务产物
8. `warp-diagnose` 的 `Alert Data` 页面现在可以：
   - 直接读取单个 `alerts/wf-alert.arrow`
   - 或读取整个 `alerts/` 目录并自动合并多份 `alert*.arrow`

## 当前规则语义

`rules/wparse_semantic.wfl` 现在拆成三层 window：

- `wparse_events`
  - 输入日志事件
- `semantic_events`
  - 逐条语义化后的中间事件
- `risk_alerts`
  - 面向 `warp-diagnose` 的最终风险告警

当前规则分两段：

1. `event_semantic_project`
   - `wparse_events -> semantic_events`
   - 负责把单条日志逐条标准化为可复用语义事件
   - 使用 `on each` 做逐条评分与语义投影
   - `risk_score` / `risk_level` 在这一层内由 `status` 计算得到，不来自 `wparse`
2. `window_risk_high / window_risk_medium / window_risk_low`
   - 输出到 `risk_alerts`
   - 负责按 `subject + 1m fixed window` 给出高/中/低风险判断
   - 三条规则互斥：
     - 先命中 high
     - 否则命中 medium
     - 否则落到 low

运行时约束说明：

- `semantic_events -> risk_alerts` 链式消费已经工作。
- 批量回放日志存在时间乱序，因此 `wfusion.toml` 里将：
  - `allowed_lateness = "365d"`
  - 用于避免 batch case 中间 window 被误判为 late data
- 历史上，上游 runtime 在 `fixed + close` 路径上对复杂聚合求值曾有稳定性问题。
- 该问题已修正；历史说明见 `UPSTREAM_DEFECT_CLOSE_AGGREGATION.md`。

`event_semantic_project` 的状态映射规则：

| 输入状态 | 风险分值 |
|---|---|
| `error` / `failed` / `failure` / `timeout` / `fatal` / `panic` / `abort` | `90` |
| `miss` / `disabled` / `retry` / `partial` / `degraded` | `80` |
| `warn` / `warning` | `70` |
| `success` / `suc` / `ok` / `end` / `enabled` / `done` / `pass` | `20` |
| 其它或空状态 | `40` |

`wfusion` 当前窗口判级规则：

- 命中高风险条件：
  - `status in {error, failed, failure, timeout, fatal, panic, abort}`
  - 输出 `Rule = window_risk_high`，`RiskScore = 90`
- 未命中高风险，但命中中风险条件：
  - `status in {warn, warning, disabled, miss, retry, partial, degraded}`
  - 输出 `Rule = window_risk_medium`，`RiskScore = 70`
- 其余窗口输出：
  - `Rule = window_risk_low`，`RiskScore = 20`

补充说明：

- `Entity = subject`
- `status` 字段在 alert 中表示窗口最终判级：`high` / `medium` / `low`
- 诊断端 `RiskLevel` 仍然由 `RiskScore` 推导

## Alert Data 字段映射

`Alert Data` 页面不是将 `wf-alert.arrow` 原始列名直接展示到表格，而是先读取为内部统一结构，再映射到 UI 表格列。

当前主表列与 `wf-alert.arrow` 字段关系如下：

| UI 列 | 优先读取字段 | 回退字段 / 规则 |
|---|---|---|
| `FiredAt` | `__wfu_fired_at` | 不回退，缺失则该 alert 行不进入 `Alert Data` |
| `RiskLevel` | 不直接读取 | 根据 `__wfu_score` / `score` 推导风险等级，再映射为 `INFO/WARN/ERROR` |
| `RiskScore` | `__wfu_score` | `score` |
| `Rule` | `__wfu_rule_name` | `rule_name` |
| `Target` | `target` | 不回退，缺失则显示为空 |
| `Entity` | `__wfu_entity_id` | 不回退，缺失或为空则显示为 `unknown` |
| `Action` | `action` | 不回退，缺失则显示为空 |

补充说明：

- `__wfu_*` 是 `wfusion` 保留字段。
- `target`、`subject`、`action`、`status`、`message`、`event_count` 来自规则 `yield (...)` 展开字段。
- 当前 `status` 已不是输入日志原始状态，而是窗口汇总后的风险分组：`high` / `medium` / `low`。
- `Rule`、`FiredAt`、`RiskScore` 属于固定引擎字段；`Target`、`Action` 属于业务 `yield` 字段。
- `Target` 与 `Action` 当前取自该 `subject` 窗口内的首条事件，仅作为辅助上下文。
- `Reason` 与 `Message` 当前不在主表展示：
  - `Reason` 对应 `__wfu_close_reason` / `close_reason` / `__wfu_origin` / `origin`
  - `Message` 对应规则产出的窗口汇总摘要
- `wf-alert.json` 和 `wf-alert.arrow` 语义一致；`warp-diagnose` 当前优先读取 `wf-alert.arrow`。
- `Alert Data` 页面展示的是归一化后的视图，不是 `wf-alert.arrow` schema 的逐列镜像。

推荐联调入口：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
case/scripts/run_wp_wf_case.sh
```

推荐诊断端变量：

```bash
WARP_DIAGNOSE_LOG_WFU=/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/data/out_dat/wp-log.arrow
WARP_DIAGNOSE_ALERT_WFU_DIR=/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wfusion/alerts
```
