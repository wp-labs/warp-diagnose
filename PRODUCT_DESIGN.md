# Warp Diagnose 产品设计文档

版本: v0.2
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
### 5.1 主视图 (Stage + Entity Timeline)
1. X 轴: 时间，从左到右。
2. 背景: stage 时间带。
3. 前景: entity 事件点。
4. 点大小: log1p(bucket_count) 归一化映射。
5. 点颜色: risk。

### 5.2 联动
1. 点击 stage: 过滤至对应时间段。
2. 点击点: 下方展示相关日志与上下文。
3. 清空按钮: 恢复全量视图。

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
1. 主图采用 Stage + Entity 单图，时间从左到右。
2. 点大小使用 log1p(bucket_count) 映射。
3. 点颜色使用风险值 risk 映射。
4. 已支持点击 stage 过滤与点击点下钻明细。
5. 计算链路优先使用 wfusion 输出 `alerts/all.jsonl`。
6. wparse 专用 case 已落地到 `case/wparse/`，可一键生成并消费告警。

## 10. 新任务入口
1. 叙事层增强:
   - Turning Points 卡片
   - Causal Chain 视图
2. 在线参数化:
   - 阈值调节 (PREP/RUN/ANOMALY)
   - 规则版本切换
3. 交互升级:
   - 时间窗缩放与拖拽
   - 多条件联动过滤（stage + entity + risk）
