# wp-tools Case Root

`case/wp-tools/` 是统一后的单工作根，同时承载：

- `wparse` 的 `conf / models / topology / connectors / data / .run`
- `wfusion` 的 `wfusion.toml / models/rules / models/schemas / sinks / alerts / logs`

这样做的目标是：

- 只维护一个 case 根目录
- 减少 `wparse -> wfusion` 的路径复制与同步成本
- 让运行脚本、默认配置和 diagnose 输入路径都收敛到同一处

## 目录

- `conf/wparse.toml`
  - `wparse` 主配置
- `models/`
  - `wparse` 的 WPL / OML / knowledge
  - `wfusion` 的 `rules` / `schemas`
- `topology/`
  - `wparse` sources / sinks
- `connectors/`
  - `wparse` case connectors
- `data/`
  - `wparse` 输入、中间日志、输出与 rescue
- `.run/`
  - `wparse` 运行期产物
- `wfusion.toml`
  - `wfusion` batch 主入口配置
- `sinks/`
  - `wfusion` sink 与 connector 定义
- `alerts/`
  - `wfusion` 输出告警
- `logs/`
  - `wfusion` runtime 日志

## 统一链路

1. `wparse batch --work-root case/wp-tools`
2. 直接产出 `data/out_dat/wp-log.arrow`
3. `wfusion run --config wfusion.toml`
4. 直接读取 `data/out_dat/wp-log.arrow`
5. 输出 `alerts/wf-alert.arrow` 等告警文件

## 关键路径

- 输入原始日志：
  - `../target_data/raw_log.dat`
- 日志事件输出：
  - `data/out_dat/wp-log.arrow`
  - `data/out_dat/wp-log.json`
- 告警输出：
  - `alerts/wf-alert.arrow`
  - `alerts/wf-alert.json`
  - `alerts/wf-entity.arrow`
  - `alerts/wf-entity.json`

## 推荐执行

完整链路：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
./case/scripts/run_wp_wf_case.sh
```

单独执行：

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
wparse batch --work-root case/wp-tools

cd case/wp-tools
wfusion run --config wfusion.toml
```
