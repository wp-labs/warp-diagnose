# wparse Case

本目录现在只承载 `wparse` 工作根：

1. `conf / models / topology / connectors`
2. 输入日志与解析输出
3. `wp` 日志 Arrow 产物

语义约定：

- `wp` = log，表示 `wparse` 解析后的日志事件
- `wf` = alert，表示 `wfusion` 计算后的告警事件

## 目录

- `conf/wparse.toml`: `wparse` 主配置
- `models/`: 从 `wp-self` 迁入的 WPL / OML / knowledge 配置
- `topology/`: 从 `wp-self` 迁入的 source / sink 路由
- `connectors/`: 当前版本 `wparse` 所需 connector 定义
- `../target_data/raw_log.dat`: 本地默认输入日志
- `data/out_dat/wp-log.arrow`: `wparse` 直接输出的 `wp` 日志 Arrow 文件
- `.run/`: `wparse` 运行期产物

`wfusion` 相关文件已经拆到：

- `../wfusion/wfusion.toml`
- `../wfusion/rules/`
- `../wfusion/schemas/`
- `../wfusion/sinks/`
- `../wfusion/alerts/`
- `../wfusion/logs/`

## 1) 准备输入日志

默认样例日志已经放在：

```bash
../target_data/raw_log.dat
```

如果要替换成目标日志，直接在顶层联调脚本执行时传入 `INPUT` 即可：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
INPUT=/absolute/path/to/target.log case/scripts/run_wp_wf_case.sh
```

脚本会先把目标日志复制为 `case/target_data/raw_log.dat`，然后执行后续解析与计算。

## 2) 单独执行 wparse

要求：
- `wparse` 可执行文件在 `PATH` 中，或通过 `WPARSE_BIN` 指定

推荐命令：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse
wparse batch --work-root .
```

## 3) 执行完整链路

推荐使用顶层联调脚本：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
case/scripts/run_wp_wf_case.sh
```

它会串联：

1. `wparse` 读取 `case/target_data/raw_log.dat`
2. 直接生成 `case/wparse/data/out_dat/wp-log.arrow`
3. OML 已直接将输出字段收敛为 `wfusion` 期望 schema
   其中 `risk_score` 表示危害得分，按 `impl_importance * action_weight * status_weight` 计算
4. 顶层脚本将同一个文件复制到 `case/wfusion/data/in_dat/wp-log.arrow`
5. `wfusion` 直接以 `arrow_framed` 读取该文件
6. 运行 `wfusion`
7. 生成 `case/wfusion/alerts/wf-alert.arrow`

输出文件：

- `data/out_dat/wp-log.arrow`

## 4) 接入 warp-diagnose 看板

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
WARP_DIAGNOSE_USE_WFUSION=1 \
WARP_DIAGNOSE_LOG_WFU=/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/data/out_dat/wp-log.arrow \
WARP_DIAGNOSE_WPARSE_LOG=/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/target_data/raw_log.dat \
WARP_DIAGNOSE_ALERT_WFU_DIR=/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wfusion/alerts \
cargo run
```

也可以直接让脚本拉起桌面端：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
RUN_DIAGNOSE=1 case/scripts/run_wp_wf_case.sh
```

当前 `warp-diagnose` 默认也会优先读取本目录下的：
- `case/wparse/data/out_dat/wp-log.arrow`
- `case/target_data/raw_log.dat`
- `case/wfusion/alerts/wf-alert.arrow`
