# Case Layout

`case/` 现在按职责拆成两段：

- `case/target_data`: 原始目标日志输入
- `case/wparse`: 只放 `wparse` 工作根、输入日志和 `wp` 日志产物
- `case/wfusion`: 只放 `wfusion` 规则、运行配置、`wf` 告警产物
- `case/scripts/run_wp_wf_case.sh`: 顶层联调入口，串联两段执行

端到端链路：

`raw log -> wparse -> wp-log.arrow -> wfusion seed -> wfusion -> wf-alert.arrow`

推荐入口：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
case/scripts/run_wp_wf_case.sh
```
