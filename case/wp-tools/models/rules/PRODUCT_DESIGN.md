# Warp Diagnose 产品设计文档

版本: v0.4
日期: 2026-03-10
项目: warp-diagnose

## 1. 产品目标
面向日志分析场景，提供一套“看一眼就知道发生了什么”的可视化诊断界面。

核心目标:
1. 让用户在统一时间轴上同时理解 stage 和 entity。
2. 支持从宏观流程到单点证据的快速下钻。
3. 对异常风险提供可解释的量化表达。

## 2. 用户问题
当前日志以行文本为主，用户难以快速回答:
1. 发生了什么。
2. 在什么时间段发生。
3. 哪些实体参与、哪些异常最关键。

## 3. 范围定义
### 3.1 In Scope
1. 单图融合: stage 时间段 + entity 事件点。
2. 风险表达: 点颜色映射风险值。
3. 数量表达: 点大小映射 log1p(bucket_count)。
4. 交互联动: 点击 stage 过滤、点击点展示下方证据。
5. 故事层: 主线叙事、关键转折点、因果链。

### 3.2 Out of Scope (v0.1)
1. 复杂跨数据源关联查询。
2. 多租户权限体系。
3. 在线规则编辑器。

## 4. 术语与数据语义
1. entity: 事件主体，优先来自 meta.subject。
2. stage: 时间流程段，由时间序列和行为变化推断得到。
3. risk: 状态风险值，范围 [0,1]。
4. stage_boundary_prob: 单事件作为阶段边界的概率。
5. stage_confidence: 阶段命名可信度。

## 5. 核心体验
### 5.1 主视图 (TimeLine)
1. X 轴: 时间，从左到右。
2. 背景: stage 时间带。
3. 前景: entity 事件点。
4. 点大小: log1p(bucket_count) 归一化映射。
5. 点颜色: risk。

### 5.2 联动
1. 点击 stage: 过滤至对应时间段。
2. 点击点: 下方展示对应 bucket 的日志证据与上下文。
3. 时间线工具栏使用语义图标，支持缩放、左右平移与统一重置。
4. Hover 点: 在点位附近显示短预览，不占固定布局区域。
5. 顶部过滤栏支持多维过滤，并在 Active 行显示当前生效条件。

### 5.3 说明卡
每个 stage 输出:
1. 主动作。
2. 异常数量。
3. 持续时长。
4. 平均风险。

## 6. 计算逻辑要求
1. 支持风险评分 status_risk_score。
2. 支持阶段边界概率 stage_boundary_prob。
3. 支持阶段分段 stage_id 与阶段命名 derived_stage。
4. 支持阶段可信度 stage_confidence。
5. 支持转折点识别: first_incident / peak_incident / recovery。
6. 支持因果链窗口: Before / Incident / After。

## 7. 验收标准
1. 一张图同时展示 stage 与 entity，时间方向明确。
2. 点击 stage 可联动过滤全页。
3. 点击点可展示相关数据明细。
4. 风险颜色与 status_risk_score 一致。
5. 点大小与 log1p(bucket_count) 一致。
6. 页面可展示主线叙事与关键转折点。

## 8. 版本里程碑
1. M1: 数据契约冻结与离线样例。
2. M2: 主视图与联动完成。
3. M3: 故事层与性能优化。
4. M4: 稳定化与文档完善。

## 9. 当前基线 (2026-03-10)
### 9.1 页面结构
1. 页面采用两层结构:
   - 顶部: LOGO + 标题 + Reload 图标按钮
   - 中上: 风险区间 KPI 看板
   - 中部: 单行 `Filter Bar`
   - 过滤栏下: `Overview / Log Data` 双分页切换
   - 上层主区: 全宽 `TimeLine`
   - 下层左侧: `Selection Detail`
   - 下层右侧: `Entity Lanes / Top Targets / Source & Status`
2. 右侧固定 `Inspector` 已移除，不再占据主视线。
3. `Recent Event Stream` 已从主路径移除，不再作为核心阅读区域。

### 9.2 主交互
1. 顶部 KPI 卡片只展示统计，不承担过滤职责。
2. 当前过滤栏支持 4 个维度:
   - `Level`
   - `Risk`
   - `Source`
   - `Stage`
3. `Stage` 继续通过时间线选择，但会同步出现在过滤体系和 Active 行里。
4. `Active` 行展示当前已生效过滤条件，并在末尾提供图标化 `Clear All`。
5. 时间线支持:
   - 主图区拖拽平移
   - 图标化缩放工具
   - 图标化左右平移
   - 图标化统一重置
6. 点击时间线圆点: 在下方 `Selection Detail` 展示该 bucket 对应证据。
7. Hover 时间线圆点: 在鼠标附近弹出短预览卡，不使用固定 Hover 面板。
8. 窗口保留系统标准控件，支持系统全屏和窗口拖拽改大小。
9. 默认进入 `Overview` 页；切换到 `Log Data` 页后，按当前 `Level / Risk / Source / Stage` 条件展示输入日志表。

### 9.3 视觉风格
1. 当前主题已切换为浅色版本。
2. 主背景为浅灰蓝，信息面板为白底。
3. 风险语义保持不变:
   - 绿色: 低风险
   - 橙色: 中风险
   - 红色: 高风险
4. 过滤维度采用分组底色:
   - Level: 蓝色系
   - Risk: 暖橙系
   - Source: 绿色系
   - Stage: 灰蓝系
5. Hover 预览采用浅暖色浮层，与白色主面板区分层级。

### 9.4 数据与实现现状
1. 主图采用 Stage + Entity 单图，时间从左到右。
2. 点大小使用 log1p(bucket_count) 映射。
3. 点颜色使用风险值 risk 映射。
4. 顶部看板已改为风险区间统计:
   - `risk < 0.60`
   - `0.60 - 0.84`
   - `risk >= 0.85`
5. 已支持 stage / level / risk / source 四维过滤、点选下钻与 hover 预览。
6. 头部已加入程序 LOGO，窗口行为回归系统标准窗口控件。
7. 计算链路优先使用 wfusion 输出 `case/wfusion/alerts/wf-alert.arrow`。
8. case 目录已按职责拆分为 `case/wparse/` 与 `case/wfusion/`，由顶层脚本串联执行。

### 9.5 交互调整归档 (2026-03-10)
1. 信息架构重排:
   - 固定 `Inspector` 移除
   - `Recent Event Stream` 移出主阅读路径
   - `Selection Detail` 升级为时间线点选后的主证据区
2. 时间线交互收敛:
   - 主图名称统一为 `TimeLine`
   - 工具按钮由文字按钮改为语义图标
   - 当前保留操作为缩小、放大、适配、左移、右移、重置
   - 拖拽平移手势绑定到固定 viewport 层，降低点位拖动抖动
3. Hover 策略调整:
   - 固定 Hover 面板移除
   - 预览卡改为点位邻近浮层，不再占据固定版面
4. 过滤体系升级:
   - 从单一 `INFO/WARN/ERROR` 演进为 `Level / Risk / Source / Stage` 四维过滤
   - `Filter` 优先压缩到单行，降低横向浪费
   - `Active` 区展示所有当前条件，并提供统一清空入口
   - 不同过滤维度使用不同底色，强化分组识别
5. 顶部区域重构:
   - 增加程序 LOGO，并导出独立 SVG
   - 顶部仅保留品牌信息与 Reload 操作，移除冗余控件
   - KPI 看板改为风险区间统计，而不是 level 过滤入口
6. 视觉体系切换:
   - 整体主题由深色切换为浅色
   - 白底信息卡 + 浅灰蓝背景作为当前基线
   - 风险颜色语义保持一致，避免切换主题后失去风险直觉
7. 双页阅读路径:
   - `Overview` 保留时间线分析与点选证据
   - `Log Data` 聚焦条件命中的结构化日志表

## 10. 新任务入口
1. 叙事层增强:
   - Turning Points 卡片
   - Causal Chain 视图
2. 在线参数化:
   - 阈值调节 (PREP/RUN/ANOMALY)
   - 规则版本切换
3. 交互升级:
   - 鼠标滚轮缩放
   - 多条件联动过滤（stage + entity + risk）
   - 结构化详情卡片替代纯文本证据块
