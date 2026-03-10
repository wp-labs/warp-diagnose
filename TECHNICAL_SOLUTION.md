# Warp Diagnose 技术方案

版本: v0.4
日期: 2026-03-10
项目: warp-diagnose

## 1. 技术选型
前端 GUI: Slint (Rust 原生 GUI)
计算引擎: wp-reactor (Rust)
数据交换: JSON/NDJSON (v0.1), 后续可扩展 Arrow/Parquet

选型理由:
1. 统一 Rust 技术栈，降低跨语言维护成本。
2. Slint 适合原生桌面，交互和表现可控。
3. wp-reactor 已有运行时与工程基础，可复用计算能力。

## 2. 系统架构
1. Ingest: 读取日志文件或标准输入。
2. Enrich: 生成 meta_entity/event_type/incident/risk/stage 字段。
3. Aggregate: 时间桶聚合与阶段摘要。
4. Serve: 输出标准化数据对象给 Slint UI。
5. Render: Slint 绘制时间轴、点图、详情面板与故事卡。

## 3. 模块划分
建议仓库: /Users/zuowenjian/devspace/wp-labs/warp-diagnose

模块:
1. crate::domain
   - EventRecord
   - StageSegment
   - PointBucket
   - NarrativeItem
2. crate::compute
   - risk.rs
   - boundary.rs
   - stage.rs
   - turning_points.rs
3. crate::adapter
   - reader_json.rs
   - reader_ndjson.rs
   - reactor_bridge.rs (对接 wp-reactor)
4. crate::ui
   - app.slint
   - timeline_view.rs
   - detail_panel.rs
5. crate::app
   - state.rs
   - commands.rs

## 4. 核心数据契约
### 4.1 事件级
1. event_ts: i64 (ms)
2. level: string
3. target: string
4. content: string
5. meta_subject/action/object/status: string
6. meta_entity: string
7. status_risk_score: f32 [0,1]
8. stage_boundary_prob: f32 [0,1]
9. stage_id: u32
10. derived_stage: string
11. stage_confidence: f32 [0,1]

### 4.2 聚合级 (bucket)
1. bucket_ts: i64
2. bucket_count: u32
3. risk_max: f32
4. incident_cnt: u32
5. entity: string
6. stage_id / derived_stage

## 5. 关键计算逻辑
### 5.1 风险评分
1. base score = 0.10
2. level 修正 (WARN/ERROR)
3. status 词集修正 (high/medium/low)
4. content 关键词二次修正
5. clamp 到 [0,1]

### 5.2 阶段边界概率
1. 输入特征:
   - action_changed
   - entity_changed
   - boundary_action
   - boundary_status
   - gap_score
2. 加权公式输出 stage_boundary_prob。

### 5.3 阶段分段与命名
1. 首条强制起段。
2. threshold + 强 gap 触发边界。
3. min_segment_events 防止碎片化。
4. 主动作映射 family 生成 derived_stage。
5. stage_confidence 由主动作占比和边界概率综合。

## 6. UI 实现策略 (Slint)
1. 顶部 Header:
   - 程序 LOGO
   - 标题与副标题
   - Reload 图标按钮
2. 顶部 Stage Track:
   - 绘制阶段带与可点击阶段节点
   - 触发过滤命令
3. 中部 Unified Timeline:
   - 依据时间轴绘制 entity 点
   - 点大小取 log1p(count) 映射
   - 点颜色取 risk 色带映射
4. 顶部 KPI Summary Row:
   - `ALL EVENTS / risk<0.60 / 0.60-0.84 / risk>=0.85`
   - 仅承担统计展示
5. 单行 Filter Bar:
   - `Level / Risk / Source / Stage`
   - 支持多条件组合过滤
6. Active Filters Row:
   - 展示已生效过滤条件
   - 末尾提供图标化 `Clear All`
7. 下部 Selection Detail:
   - 展示点选 bucket 对应日志明细
   - 显示关联 stage、risk、action、status、content
8. Hover Preview:
   - 跟随点位邻近显示
   - 不占固定布局列

## 6.1 当前交互状态机
1. 过滤状态:
   - `selected_stage: Option<usize>`
   - `selected_level: Option<LevelFilter>`
   - `selected_risk: Option<RiskFilter>`
   - `selected_source: Option<SourceFilter>`
2. 点位状态:
   - `selected_point: Option<usize>`
   - `hover_point: Option<usize>`
3. 视窗状态:
   - `timeline_canvas_x`
   - `timeline_zoom`

组合逻辑:
1. 先按 `selected_stage` 过滤事件集合。
2. 再按 `selected_level` 过滤。
3. 再按 `selected_risk` 过滤。
4. 再按 `selected_source` 过滤。
5. 点选后将 bucket 详情写入 `Selection Detail`。
6. Hover 仅更新短预览文本，不改变主选择。

## 6.2 当前布局基线
1. 上层:
   - Header + LOGO
   - KPI Summary Row
   - 单行 Filter Bar
   - Active Filters Row
   - 全宽 Timeline 面板
   - 内含图标化缩放、平移与重置
2. 下层左侧:
   - `Selection Detail` 主展示区
3. 下层右侧:
   - `Entity Lanes`
   - `Top Targets`
   - `Source & Status`
4. 已移除:
   - 固定 `Inspector`
   - 固定 `Hover Preview`
   - `Recent Event Stream` 主阅读区

## 6.3 交互实现约束
1. 时间线导航:
   - 仅保留语义图标按钮
   - 当前包括缩小、放大、适配、左移、右移、重置
   - `重置` 同时恢复时间线视窗并清空过滤条件
2. 拖拽平移:
   - 手势绑定在时间线固定 viewport 层
   - 点位自身只负责 hover/click，不承担整图区拖拽
   - 通过分离职责降低拖拽抖动与误触
3. Hover 预览:
   - 采用点位邻近浮层
   - 不再维护固定侧栏或固定底部 hover 区
   - hover 只更新短预览，不改动当前选中详情
4. 过滤条:
   - `Level / Risk / Source` 使用 pill 选择器
   - `Stage` 通过时间线选择后映射为过滤标签
   - `Active` 行展示所有生效维度，并在尾部提供统一清空图标
   - 各维度使用独立色系，降低多维过滤时的识别成本
5. 窗口行为:
   - Slint Window 保持 `no-frame: false`
   - 使用系统标准窗口控件处理最大化/全屏
   - 初始窗口尺寸由 Rust 侧设置，避免在 UI 描述层锁死窗口行为

## 7. 性能策略
1. 预聚合: 200ms bucket
2. 增量加载: 仅加载当前时间窗
3. 分层渲染: stage 背景与点图分离
4. 缓存: 输入文件 mtime/size 变化才重算

## 8. 里程碑与交付
1. P0 (1 周): 数据模型 + 计算模块 + CLI 输出
2. P1 (1 周): Slint 主界面 + stage/point 联动
3. P2 (1 周): 叙事层 + turning points + 因果链
4. P3 (1 周): 优化 + 回归测试 + 发布文档

## 9. 风险与应对
1. 日志 meta 质量不稳定
   - 对策: 回退规则 + unknown 占位
2. 阶段切分并非业务真值
   - 对策: 暴露概率和置信度
3. 数据量过大导致 UI 卡顿
   - 对策: 预聚合 + 视窗裁剪 + 分级渲染

## 10. 当前可执行链路 (2026-03-10)
1. 规则与数据准备:
   - 目录: `/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse`
   - 脚本: `scripts/run_file_case.sh`
2. 运行 wfusion 计算:
   - 推荐二进制: `/Users/zuowenjian/devspace/wp-labs/wp-reactor/target/debug/wfusion`
   - 输出: `case/wparse/alerts/all.jsonl`
3. 启动看板消费:
   - `WARP_DIAGNOSE_USE_WFUSION=1`
   - `WARP_DIAGNOSE_WFUSION_ALERTS=/Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/alerts/all.jsonl`
4. 当前样例规模:
   - file source ingest: `2161` rows
   - alert 输出: `89` rows（适合时间轴故事展示）

## 10.1 当前 UI 基线 (2026-03-10)
1. 已支持时间线拖拽平移，拖拽手势绑定在固定 viewport 层，避免抖动。
2. 时间线工具栏已收敛为图标按钮，并支持缩放、左右平移、统一重置。
3. 已支持 `level / risk / source / stage` 四维过滤联动。
4. 顶部 KPI 已切换为风险区间统计，而非 level 统计。
5. 已切换为浅色主题，维持风险色语义不变，并为不同过滤维度分配不同底色。
6. Hover 预览已改为点位邻近浮层，固定 Hover 面板不再保留。
7. 窗口行为已回归系统标准控件，支持系统级全屏与调整窗口大小。

## 11. 新任务技术起点
1. 在 `case/wparse/rules/wparse_semantic.wfl` 上增量调参，优先保证输出稳定性与可解释性。
2. 在 `src/data.rs` 增加规则标签聚合（按 rule_name/score/entity_type 分层）。
3. 在 Slint 主图增加 Narrative 侧栏，复用现有 point detail 与 stage card 数据。
4. 后续若需要严格 lint 通过，可按 WFL v2.1 补 `limits { ... }`（当前运行不受阻）。
