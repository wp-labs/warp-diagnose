# Case Guide

`case/` 是本工程内置的端到端联调目录，用来在本仓库内完成：

`raw log -> wparse -> wp-log.arrow -> wfusion -> wf-alert.arrow/json -> warp-diagnose`

语义约定：

- `wp` = log，表示 `wparse` 解析后的日志事件
- `wf` = alert，表示 `wfusion` 计算后的告警事件

## 目录

- `target_data/`
  - 原始输入日志
  - 默认文件：`target_data/raw_log.dat`
- `wp-tools/`
  - 统一工作根
  - 同时承载 `wparse` 与 `wfusion` 的配置、模型、规则和产物
- `scripts/run_wp_wf_case.sh`
  - 顶层联调入口
  - 串联 `wparse -> wfusion -> warp-diagnose`

## 当前链路

1. `wparse` 读取 `case/target_data/raw_log.dat`
2. `wparse` 直接输出 `case/wp-tools/data/out_dat/wp-log.arrow`
3. `wfusion` 直接读取同一个 `case/wp-tools/data/out_dat/wp-log.arrow`
4. `wfusion` 输出：
   - `case/wp-tools/alerts/wf-alert.arrow`
   - `case/wp-tools/alerts/wf-alert.json`
   - `case/wp-tools/alerts/wf-entity.arrow`
   - `case/wp-tools/alerts/wf-entity.json`
5. `warp-diagnose` 读取：
   - `case/wp-tools/data/out_dat/wp-log.arrow`
   - `case/wp-tools/alerts/`

## 推荐入口

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
./case/scripts/run_wp_wf_case.sh
```

执行完成后，通常会得到：

- `case/wp-tools/data/out_dat/wp-log.arrow`
- `case/wp-tools/data/out_dat/wp-log.json`
- `case/wp-tools/alerts/wf-alert.arrow`
- `case/wp-tools/alerts/wf-alert.json`

## 替换输入日志

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
INPUT=/absolute/path/to/target.log ./case/scripts/run_wp_wf_case.sh
```

脚本会先把输入复制为：

```bash
case/target_data/raw_log.dat
```

## 直接启动看板

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
WARP_DIAGNOSE_USE_WFUSION=1 \
WARP_DIAGNOSE_LOG_WFU=/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wp-tools/data/out_dat/wp-log.arrow \
WARP_DIAGNOSE_WPARSE_LOG=/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/target_data/raw_log.dat \
WARP_DIAGNOSE_ALERT_WFU_DIR=/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wp-tools/alerts \
cargo run
```

也可以让联调脚本在末尾直接拉起：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
RUN_DIAGNOSE=1 ./case/scripts/run_wp_wf_case.sh
```

## 说明

- 当前 case 已不再维护 `case/wparse` 与 `case/wfusion` 双工作根。
- `wfusion` 的主配置入口已收敛为 `case/wp-tools/wfusion.toml`，不再推荐使用 `case/wp-tools/conf/wfusion.toml`。
- 统一后的工作根说明见：
  [wp-tools/README.md](/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wp-tools/README.md)
