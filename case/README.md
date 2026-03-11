# Case Guide

`case/` 是本工程内置的端到端联调样例目录，用来在本仓库内完成：

`raw log -> wparse -> log_wfu.arrow/wp-log.arrow -> wfusion -> alert*_wfu.arrow(or wf-alert.arrow/json) -> warp-diagnose`

语义约定：

- `wp` = log，表示 `wparse` 解析后的日志事件
- `wf` = alert，表示 `wfusion` 计算后的告警事件

## 目录结构

- `target_data/`
  - 原始输入日志
  - 默认文件：`target_data/raw_log.dat`
- `wparse/`
  - `wparse` 工作根
  - 负责把原始日志解析为 `wp` 事件
- `wfusion/`
  - `wfusion` 工作根
  - 负责基于 `wp` 事件计算 `wf` 告警
- `scripts/run_wp_wf_case.sh`
  - 顶层联调入口
  - 串联 `wparse -> wfusion -> warp-diagnose`

## 当前链路

1. `wparse` 读取 `case/target_data/raw_log.dat`
2. `wparse` 输出 `case/wparse/data/out_dat/wp-log.arrow`
3. 顶层脚本复制该文件到 `case/wfusion/data/in_dat/wp-log.arrow`
4. `wfusion` 以 `format = "arrow_framed"` 直接消费该 Arrow 文件
5. `wfusion` 稳定输出：
   - `case/wfusion/alerts/wf-alert.arrow`
   - `case/wfusion/alerts/wf-alert.json`
6. `case/wfusion/alerts/wf-semantic.arrow` 当前会保留为空文件：
   - `semantic_events` 是中间 window
   - 中间 target 不会直接进入最终 sink
7. `warp-diagnose` 读取：
   - `wp-log.arrow` 作为日志数据
   - `wf-alert.arrow` 作为告警数据

## 当前风险语义

`wfusion` 当前采用 `subject` 级风险汇总模型：

1. `wparse` 只负责把原始日志解析成结构化 `wp` 事件
2. `wfusion` 的 `event_semantic_project` 使用 `on each` 按 `status` 计算单条事件的 `risk_score`
3. 同时在 `semantic_events` 中产出单条事件的 `risk_level`
4. `wfusion` 在固定 `1m` 时间窗内，按 `subject` 归并这些日志
5. 再输出该 `subject` 的整体 `high / medium / low` 风险告警

补充：

- batch case 使用放宽后的 late-data 配置：
  - `allowed_lateness = "365d"`
- 这是为了兼容离线日志的时间乱序，避免中间 `semantic_events` 被丢弃

这意味着：

- `LogData` 展示的是逐条计算后的日志事件
- `Alert Data` 展示的是多份批量告警文件归并后的结果
- 一个 `subject` 在一个窗口内通常只对应一条汇总告警

## Diagnose 输入模型

`warp-diagnose` 现在按下面的模型读数据：

- 1 份逐条事件文件
  - 优先：`log_wfu.arrow`
  - 兼容：`wp-log.arrow`
- 多份批量告警文件
  - 目录模式：读取目录内所有 `*alert*.arrow` 或 `*_wfu.arrow`
  - 兼容单文件：`wf-alert.arrow`

推荐环境变量：

- `WARP_DIAGNOSE_LOG_WFU`
  - 指向逐条事件 Arrow 文件
- `WARP_DIAGNOSE_ALERT_WFU_DIR`
  - 指向批量告警 Arrow 所在目录

兼容变量仍保留：

- `WARP_DIAGNOSE_DEMO_JSON`
- `WARP_DIAGNOSE_WFUSION_ALERTS`

## 推荐入口

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
./case/scripts/run_wp_wf_case.sh
```

执行完成后，通常会得到：

- `case/wparse/data/out_dat/wp-log.arrow`
- `case/wparse/data/out_dat/wp-log.json`
- `case/wfusion/alerts/wf-alert.arrow`
- `case/wfusion/alerts/wf-alert.json`
- `case/wfusion/alerts/wf-semantic.arrow`
  - 文件存在但当前为空，用于保留中间语义流位置

## 替换输入日志

如果要换成自己的目标日志，可以在运行脚本时传入 `INPUT`：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
INPUT=/absolute/path/to/target.log ./case/scripts/run_wp_wf_case.sh
```

脚本会先把输入复制为：

```bash
case/target_data/raw_log.dat
```

再执行后续链路。

## 直接启动看板

链路执行完成后，可以直接启动桌面端：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
WARP_DIAGNOSE_USE_WFUSION=1 \
WARP_DIAGNOSE_LOG_WFU=/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/data/out_dat/wp-log.arrow \
WARP_DIAGNOSE_WPARSE_LOG=/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/target_data/raw_log.dat \
WARP_DIAGNOSE_ALERT_WFU_DIR=/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wfusion/alerts \
cargo run
```

也可以让联调脚本在末尾直接拉起：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
RUN_DIAGNOSE=1 ./case/scripts/run_wp_wf_case.sh
```

## 子目录说明

- [wparse/README.md](/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/README.md)
  - `wparse` 输入、OML、拓扑和 `wp` 产物说明
- [wfusion/README.md](/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wfusion/README.md)
  - `wfusion` 规则、sink、`wf` 产物，以及 `Alert Data` 字段映射说明
