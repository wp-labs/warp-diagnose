# Warp Diagnose 配置说明

运行时配置文件默认路径:

- `/Users/zuowenjian/devspace/wp-labs/warp-diagnose/config/warp-diagnose.toml`

也可以通过环境变量覆盖配置文件路径:

- `WARP_DIAGNOSE_CONFIG`

## 1. 配置结构

当前支持四类配置:

1. `data`
2. `timeline`
3. `table`
4. `window`

## 2. data

用于控制默认数据输入来源。

字段:

- `primary_log_path`
  - 主日志输入，支持 Arrow / JSON / NDJSON
- `wparse_log_path`
  - 原始日志路径，用于 detail / fallback
- `wfusion_alerts_path`
  - wfusion 告警输入，可以是 Arrow 文件，也可以是 alerts 目录
- `wfusion_enabled`
  - 是否优先读取 wfusion 结果

当前兼容的环境变量覆盖:

- `WARP_DIAGNOSE_LOG_WFU`
- `WARP_DIAGNOSE_DEMO_JSON`
- `WARP_DIAGNOSE_WPARSE_LOG`
- `WARP_DIAGNOSE_ALERT_WFU_DIR`
- `WARP_DIAGNOSE_WFUSION_ALERTS`
- `WARP_DIAGNOSE_USE_WFUSION`

## 3. timeline

用于控制时间线显示和聚合粒度。

字段:

- `unit_ms`
  - 时间线基础时间单位，当前默认 `100`
- `max_lanes`
  - 最多显示多少条 canonical subject 泳道
- `min_width_px`
  - 时间线最小内容宽度
- `max_width_px`
  - 时间线最大内容宽度
- `px_per_unit`
  - 每个时间单位对应的像素宽度
- `vertical_padding_pct`
  - 泳道上下留白比例

当前兼容的环境变量覆盖:

- `WARP_DIAGNOSE_TIMELINE_UNIT_MS`

## 4. table

用于控制分页表格的页面容量估算。

字段:

- `window_chrome_px`
  - 估算分页时扣除的非表体高度
- `row_height_px`
  - 表格单行高度
- `min_page_size`
  - 每页最少显示行数

## 5. window

用于控制应用默认启动窗口尺寸。

字段:

- `width`
- `height`

## 6. 当前仍未配置化的内容

下面这些仍然属于代码层面的结构常量，而不是运行时配置:

1. 时间单位换算常量
   - `SECOND_NS`
   - `MILLISECOND_NS`
2. tick 步长候选集合
3. UI 组件内部的大量视觉尺寸
   - 圆点大小
   - lane rail 宽度
   - hover 卡尺寸
   - 各类圆角和间距
4. case 脚本中的联调路径和 wfusion 二进制发现逻辑

这些后续如果要进一步产品化，可以继续分层:

1. `render` 配置
2. `case` / `dev` 配置
3. `theme` 配置
