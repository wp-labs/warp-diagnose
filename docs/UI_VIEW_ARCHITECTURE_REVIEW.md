# Warp Diagnose 窗口视图关系 Review

版本: v0.1
日期: 2026-03-12
项目: warp-diagnose

## 1. 目的
这份文档只回答一个问题:

当前窗口视图关系是否合理，如果不合理，应该如何调整为更稳定、更易维护的界面方案。

重点不在视觉细节，而在:
1. 页面职责是否清晰。
2. 状态归属是否正确。
3. 交互联动是否边界明确。
4. 布局和滚动是否符合桌面应用习惯。

## 2. 当前结构概览
当前窗口可以概括为:

1. 顶部 Header
   - LOGO
   - 标题
   - Reload
2. KPI 区
   - 全量事件
   - 风险区间统计
3. 全局 Filter 区
   - Level
   - Risk
   - Source
   - Active Filters
4. Page 区
   - Overview
   - Log Data
   - Alert Data
5. Overview 主体
   - TimeLine
   - Selection Detail
   - Canonical Subjects / Top Targets / Source & Status
6. Log Data 主体
   - 结构化日志表
7. Alert Data 主体
   - 结构化告警表

这个方向本身没有问题，但当前实现上，窗口层、页面层、组件层耦合过重。

## 3. 当前问题
### 3.1 AppWindow 承担了过多职责
当前 `AppWindow` 同时承担:
1. 应用壳布局。
2. 全局过滤状态。
3. Overview 页状态。
4. Log Data 页状态。
5. Alert Data 页状态。
6. TimeLine 交互状态。
7. 表格分页和滚动状态。

这会导致两个直接问题:
1. 任意交互都容易影响不相关页面。
2. 新增一个页面或新过滤维度时，顶层状态继续膨胀。

### 3.2 当前刷新模型是“全局刷新 + 局部补丁”
现在的行为接近:

1. 过滤变化后重算整个主视图。
2. 再根据当前页补刷 log/alert 表。
3. resize 时只补刷当前表页。
4. 点选时又单独改 detail。

这个模型的问题是:
1. 刷新入口太多。
2. 同一份状态在多个函数里被重复改写。
3. 很难稳定推导某个用户动作会影响哪些区域。

### 3.3 Overview 页内部职责还不够单纯
Overview 当前同时承担:
1. 时间定位。
2. 风险感知。
3. 点选证据查看。
4. hover 预览。
5. timeline 缩放和平移。
6. 右侧上下文信息浏览。

这些功能可以共存，但必须拆清子区域职责:
1. TimeLine 只负责时序观察与选择。
2. Selection Detail 只负责展示已选中点相关原始日志。
3. Context Panel 只负责辅助说明，不参与主交互闭环。

### 3.4 表格组件抽象层次不对
当前 `DetailTablePanel` 同时负责:
1. 表格外框。
2. 页头说明。
3. 分页条。
4. 表头。
5. 表体滚动。
6. Log 列定义。
7. Alert 列定义。
8. 语义颜色差异。

这意味着它不是“通用表格框架”，而是“多个业务表格拼在一起”。

后果:
1. 开关越来越多。
2. 表头/表体对齐更容易出问题。
3. 新增列或调整颜色时，影响面不清楚。

### 3.5 分页大小不应由窗口高度近似推导
当前分页本质上还是基于窗口总高度估算。

这不是稳定方案，因为:
1. Header 高度会变。
2. Summary 行高度会变。
3. 不同页的工具条高度不一定一致。
4. 将来如果加局部过滤栏，分页又会偏。

更合理的依据应是:
表格 body viewport 的真实可视高度。

## 4. 正确的视图关系
建议把窗口拆成三层:

1. 应用壳层 `AppShell`
2. 页面层 `OverviewPage / LogPage / AlertPage`
3. 组件层 `TimelinePanel / DataGridFrame / ContextPanel`

其中:

### 4.1 应用壳层
职责只包括:
1. 顶部品牌区。
2. KPI 区。
3. 全局过滤区。
4. 页签区。
5. 当前页面承载区。

应用壳层不应直接保存:
1. point 选中状态。
2. timeline pan/zoom。
3. 表格滚动位置。
4. 表格分页位置。

### 4.2 页面层
每个页面只对自己的局部状态负责。

#### OverviewPage
负责:
1. TimeLine 渲染。
2. 点位 hover。
3. 点位选择。
4. 下方证据详情。
5. 右侧上下文说明。

不负责:
1. Log 表格分页。
2. Alert 表格分页。
3. 其他页面滚动状态。

#### LogPage
负责:
1. 原始日志表。
2. 表格分页。
3. 横向滚动。
4. 表体裁剪展示。
5. 后续排序能力。

#### AlertPage
负责:
1. 告警结果表。
2. 表格分页。
3. 横向滚动。
4. 表体裁剪展示。
5. 后续排序能力。

### 4.3 组件层
建议拆成下面几类:

1. `TimelinePanel`
   - 时间轴
   - 点位图层
   - tick
   - hover 浮层
   - pan / zoom 工具
2. `SelectionDetailPanel`
   - 已选点的原始日志详情
3. `ContextPanel`
   - Canonical Subjects
   - Top Targets
   - Source & Status
4. `DataGridFrame`
   - 页头说明
   - 分页条
   - 表头
   - 表体
   - 单层横向滚动
   - 表体裁剪
5. `LogTableView`
   - 只定义 Log 的列
6. `AlertTableView`
   - 只定义 Alert 的列

## 5. 正确的状态归属
### 5.1 全局状态
建议只保留:
1. `active_page`
2. `global_filters`
   - level
   - risk
   - source
3. `data_store`

### 5.2 Overview 局部状态
建议独立维护:
1. `timeline_zoom`
2. `timeline_pan_x`
3. `selected_point_id`
4. `hover_point_id`

### 5.3 LogPage 局部状态
建议独立维护:
1. `page_idx`
2. `page_size`
3. `viewport_x`

### 5.4 AlertPage 局部状态
建议独立维护:
1. `page_idx`
2. `page_size`
3. `viewport_x`

## 6. 正确的数据流
### 6.1 全局过滤变化
只影响:
1. Overview 派生视图
2. LogPage 派生结果
3. AlertPage 派生结果

但不应直接重置:
1. Overview 的 zoom/pan
2. 其他页面的滚动位置

例外:
如果过滤后当前分页越界，应把该页页码钳制到最后一页。

### 6.2 页签切换
只切换当前显示页面。

不应做的事:
1. 不应重新初始化所有页面状态。
2. 不应清空当前选中的 point。
3. 不应清空表格滚动位置。

### 6.3 时间线点选
只影响:
1. Overview 的 `selected_point_id`
2. Selection Detail 内容

不应影响:
1. 全局 filter
2. LogPage 分页
3. AlertPage 分页

### 6.4 时间线 hover
只影响:
1. Overview 的 `hover_point_id`
2. hover preview 内容和位置

不应影响:
1. Selection Detail 主内容
2. 表格页

### 6.5 表格翻页
只影响对应页:
1. `LogPage.page_idx`
2. `AlertPage.page_idx`

不应影响:
1. Overview
2. Timeline
3. 另一张表

### 6.6 窗口 resize
应该影响:
1. 布局伸缩
2. 表格真实 viewport 高度
3. 当前页 page_size

不应影响:
1. point 选择
2. hover
3. 过滤条件

## 7. 正确的页面职责
### 7.1 Overview
定位:
分析入口页。

主要回答:
1. 什么时间发生了什么。
2. 哪些 canonical subject 风险集中。
3. 该时间点能下钻到哪些原始日志。

核心动作:
1. 浏览
2. 缩放
3. 平移
4. 点选
5. hover 预览

### 7.2 Log Data
定位:
证据浏览页。

主要回答:
1. 过滤条件下有哪些原始日志。
2. 这些日志的时间、canonical subject、status、content 分布如何。

核心动作:
1. 分页
2. 滚动
3. 后续排序

### 7.3 Alert Data
定位:
风险结果浏览页。

主要回答:
1. 规则输出了哪些告警。
2. 每条告警的风险、规则、实体和目标是什么。
3. 这批告警和原始日志如何对应。

核心动作:
1. 分页
2. 滚动
3. 后续排序
4. 后续点击联动到日志证据

## 8. 正确的布局方案
### 8.1 上层固定区
从上到下固定为:
1. Header
2. KPI Row
3. Filter Row
4. Page Tabs

这些区域高度应稳定，避免页面切换时主内容跳动。

### 8.2 下层可伸缩区
下层是唯一主内容区。

#### Overview 布局
建议:
1. 上半区: `TimelinePanel`
2. 下半区左: `SelectionDetailPanel`
3. 下半区右: `ContextPanel`

比例建议:
1. Timeline 约占 60% 到 65%
2. 下层详情区约占 35% 到 40%

#### Log / Alert 布局
建议:
1. 整页只有一个主表格区
2. 表头固定
3. 表体裁剪
4. 分页放在标题下方

Log 与 Alert 布局结构应一致，只是列定义不同。

## 9. 正确的表格方案
### 9.1 表头与表体必须共用横向 viewport
这是桌面表格的硬约束，不是可选优化。

否则会出现:
1. 最大化后偏离
2. 横向滚动后错位
3. 首行对齐异常

### 9.2 表格分页应按真实 body viewport 计算
正确做法:
1. 读到表格 body 可视高度
2. `page_size = floor(visible_height / row_height)`
3. 至少设置一个下限，例如 20 行

### 9.3 行号语义
行号应明确是哪一种:

1. 页内行号
2. 结果集绝对行号

建议:
Log / Alert 页使用“结果集绝对行号”，更利于过滤后定位。
Overview 下方 detail 使用“局部明细行号”，从 1 开始即可。

### 9.4 已验证的稳定表格模型
2026-03-12 的修正已经证明，桌面表格在当前项目里应遵守下面的结构:

1. 整张表只允许一层横向 `ScrollView`。
2. 表头和表体必须放在同一个横向内容坐标系中。
3. 纵向不使用 `ScrollView` 承担“翻页后展示下一批行”的职责。
4. 纵向只做两件事:
   - 通过 `page_size` 决定当前页包含哪些行
   - 通过 `clip` 裁剪超出表体高度的内容
5. 页签切换、过滤切换、数据重载后，只需要重置横向视口，不应再维护独立纵向 viewport 状态。

这样做的直接收益:
1. 表头不会丢失。
2. 首行不会被隐藏或从中间行开始。
3. 最大化后表头和表体仍能保持对齐。
4. 横向滚动后列不会偏移。

### 9.5 明确禁止的错误模式
下面这些模式在本项目里已经被验证会导致错位或首行异常，后续不要再使用:

1. 把表头和表体分别放在两个独立横向 viewport 中，再尝试手动同步。
2. 把表头和表体一起塞进外层 `ScrollView`，同时又在内部保留第二层滚动容器。
3. 使用纵向 `ScrollView` 承担分页后的表体显示。
4. 依赖窗口总高度近似估算表格可见行数。
5. 通过零散补丁反复重置 `viewport-y` 试图修正首行错位。

这些模式的问题不是“偶尔会出 bug”，而是结构上就不稳定:
1. viewport 状态容易残留。
2. 头体容易脱离同一坐标系。
3. resize、page switch、data reset 时很难保证结果一致。

### 9.6 当前确认可复用的页面方案
截至 2026-03-12，下面这套方案已经在当前工程中被验证为正确基线:

1. `Overview`
   - 顶部全宽 `TimeLine`
   - 下方左侧 `Selection Detail`
   - 下方右侧 `ContextPanel`
2. `Log Data`
   - 单主表格页
   - 标题
   - 页码条
   - 固定表头
   - 裁剪表体
3. `Alert Data`
   - 与 `Log Data` 使用同构布局
   - 只变列定义与语义着色

对这三页的统一要求:
1. 全局过滤只影响数据结果，不直接改写局部滚动模型。
2. 点选只影响 `Overview` 的 `Selection Detail`。
3. `Log Data` 与 `Alert Data` 的表格结构完全一致，只允许业务列定义不同。
4. 时间线不是表格，不允许为迎合表格实现而把时间线做成栅格表观。

### 9.7 泳道与实体的最终语义
截至 2026-03-12，术语统一为:

1. `Entity = canonical subject`
2. `RawSubject = original subject`
3. `Object = related object`
4. `Swimlane = canonical subject activity lane`

对应的界面约束:
1. 主图泳道只按 `canonical subject` 分组。
2. 原始 `subject` 不直接参与主泳道。
3. `object` 不直接参与主泳道。
4. `raw_subject / object / target / action / status` 进入 hover 和 detail。

## 10. 推荐的最终界面方案
### 10.1 应保留
1. 顶部浅色 Header + LOGO
2. 风险区间 KPI 看板
3. 单行全局 Filter
4. `Overview / Log Data / Alert Data` 三页
5. Overview 的 TimeLine + Selection Detail
6. Hover 邻近浮层

### 10.2 应收敛
1. 右侧 `ContextPanel` 只做辅助说明，不承担主交互
2. `Selection Detail` 只展示当前点选证据，不混入 hover 内容
3. 表格页不再承接 overview 的点选状态

### 10.3 应去耦
1. 时间线状态与表格状态分离
2. 全局过滤与局部滚动分离
3. 通用表格框架与业务列定义分离

## 11. 落地顺序
建议按下面顺序实施:

1. 第一步: 文档和结构重命名
   - 明确 `AppShell / OverviewPage / LogPage / AlertPage / DataGridFrame`
2. 第二步: 状态拆分
   - 把 Overview / Log / Alert 的局部状态从顶层状态中拆开
3. 第三步: 表格组件拆分
   - `DetailTablePanel` 拆为 `DataGridFrame + LogTableView + AlertTableView`
4. 第四步: 刷新链路拆分
   - `apply_view()` 拆为多个页面级 apply 函数
5. 第五步: 分页改为真实 viewport 驱动
6. 第六步: 再做排序、列宽、联动增强

## 12. 结论
当前界面方向是对的:
1. `Overview` 负责发现问题。
2. `Log Data` 负责看原始证据。
3. `Alert Data` 负责看计算结果。

但当前实现层次还不够清晰。

更正确的方案是:
1. 应用壳只管全局区域。
2. 页面只管本页状态。
3. 组件只管单一职责。
4. Overview 和表格页彻底解耦。
5. 表格滚动与分页回到标准桌面表格模型。

这样后续再增加:
1. 新 filter 维度
2. 排序
3. 列定制
4. 点击 alert 联动 log
5. 第二种 overview 布局

都不会再次把窗口层逻辑拖乱。
