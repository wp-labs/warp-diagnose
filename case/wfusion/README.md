# wfusion Case

本目录只承载 `wfusion` 相关内容：

- `wfusion.toml`: 当前提供的是 `batch` 模式 runtime 配置
- `schemas/`, `rules/`, `sinks/`: 规则与输出路由
- `alerts/`: `wf` 告警产物
- `logs/`: runtime 日志

语义约定：

- `wp` = log
- `wf` = alert

当前执行关系：

1. `case/wparse` 直接产出 `wp-log.arrow`
2. `wparse` 侧 OML 已直接产出 `wfusion` 期望字段 schema
3. 顶层脚本仅将同一个 `wp-log.arrow` 复制到 `data/in_dat/wp-log.arrow`
4. `wfusion` 以 `mode = "batch"` 直接消费该 Arrow 文件
5. source 配置使用 `format = "arrow_framed"`，并显式覆盖 `stream = "wparse"`
6. `wfusion` 先写出 `alerts/wf-alert.jsonl`
7. 本工程再用 `wf_alert_json_to_arrow` 转为 `alerts/wf-alert.arrow`
8. `warp-diagnose` 直接读取该 Arrow 产物

推荐联调入口：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
case/scripts/run_wp_wf_case.sh
```
