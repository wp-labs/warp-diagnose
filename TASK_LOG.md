# Warp Diagnose 任务记录

更新时间: 2026-03-10
记录人: Codex

## 1. 背景
用户确认创建独立项目目录:
/Users/zuowenjian/devspace/wp-labs/warp-diagnose

并要求先沉淀文档和启动工程初始化。

## 2. 已完成
1. 新建文档 PRODUCT_DESIGN.md。
2. 新建文档 TECHNICAL_SOLUTION.md。
3. 新建文档 TASK_LOG.md。
4. 初始化 Rust 二进制工程: warp-diagnose。
5. 接入 Slint 最小可运行骨架:
   - Cargo.toml 依赖
   - build.rs
   - ui/appwindow.slint
   - src/main.rs
6. 执行 cargo check 并通过。
7. 接入真实数据读取:
   - 读取 demo.json (NDJSON)
   - 读取 wparse.log (文本日志)
   - 启动自动加载 + Reload 重新加载
   - 状态区展示行数、WARN/ERROR、实体/目标数、时间范围、错误信息
8. 再次执行 cargo check 并通过。
9. 升级 Slint 基础展示为可读日志看板:
   - KPI 指标卡(total/info/warn/error)
   - Recent Event Stream(真实事件流)
   - Top Targets / Top Entities 排行
   - Source & Status 详情面板
10. 落地 Stage + Entity 时间图（单图）:
   - Stage 带按时间从左到右展示
   - Entity 点按时间投影并用风险着色、对数大小
   - 点击 stage 过滤、点击点查看详情
   - 提供 Clear Stage / Clear Point 交互
11. 重构数据计算层:
   - 风险评分
   - 阶段边界概率与分段
   - 阶段命名与置信度
   - 点位聚合与明细构建
12. 执行 cargo check 通过（无 warning）。
13. 增强时间图可读性：
   - 增加时间刻度与竖向网格线
   - 增加实体泳道标签与横向网格线
   - 增加 Stage 说明卡行并支持点击过滤
14. 完成新一轮 cargo check 验证通过。
15. 增加滚动支持与窗口尺寸控制：
   - 关键日志面板接入 ScrollView 滚动条
   - 顶部增加 Compact/Default/Wide 窗口尺寸控制
   - 圆点去除白色描边，降低锯齿突显
16. 再次执行 cargo check 验证通过。
17. 增加点位 Hover 预览交互：
   - 圆点支持 has-hover 事件回调
   - 新增 Hover Preview 面板实时展示摘要
18. 再次执行 cargo check 验证通过。
19. 完成整体布局重构优化：
   - 主体改为 Timeline 主画布 + 右侧 Inspector + 底部日志区
   - 降低同层信息块密度，提升信息层级清晰度
   - 保留滚动、窗口尺寸控制与 hover/click 联动
20. 再次执行 cargo check 验证通过。
21. 新增 WFusion 执行方案文档：
   - 新建 WFUSION_EXECUTION_PLAN.md
   - 明确模式 A/B/C 执行路径
   - 明确 warp-diagnose 环境变量与回退逻辑
22. 新增 wparse 规则 case:
   - 新建 case/wparse 目录结构（schemas/rules/sinks/scripts/data/alerts）
   - 生成 wparse_semantic.wfs 与 wparse_semantic.wfl（prepare/running/anomaly）
   - 生成 wfusion.toml 与 sink 配置
   - 新增 build_wparse_events.py 用于 demo/raw log 转 NDJSON
   - 新增 case/wparse/README.md 运行说明
23. 验证并修正 file source 路径:
   - 复测 `wfusion` file source ingest（rx_rows=2161）
   - 修复 event_time 为纳秒整数输出
   - 新增 scripts/run_file_case.sh 一键跑通
24. 优化规则叙事密度:
   - 规则改为 `target+subject` 粒度 + `1s fixed` 窗口
   - 采用 close 路径触发，避免事件级爆量
   - 样例输出收敛为 89 条（更适配时间轴故事展示）
25. 文档阶段归档:
   - 更新 PRODUCT_DESIGN.md 为 v0.2
   - 更新 TECHNICAL_SOLUTION.md 为 v0.2
   - 明确“当前基线 + 新任务入口”
26. 迁移 Python 看板脚本:
   - 从 `wp-self/view` 迁移 `demo_log_dashboard.py` 到 `warp-diagnose/view`
   - 默认数据路径改为优先 `case/wparse/data/wparse_events.ndjson`
   - 保留旧路径回退兼容

## 3. 当前结论
1. 技术路线确定为 Rust 原生 GUI: Slint。
2. 计算层依托 wp-reactor，可通过 bridge 复用。
3. v0.1 以 JSON/NDJSON 为主，后续可扩展 Arrow/Parquet。
4. 当前工程已可编译，并能读取真实样例数据。

## 4. 当前工程文件
1. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/Cargo.toml
2. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/build.rs
3. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/src/main.rs
4. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/src/data.rs
5. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/ui/appwindow.slint
6. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/PRODUCT_DESIGN.md
7. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/TECHNICAL_SOLUTION.md
8. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/TASK_LOG.md
9. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/WFUSION_EXECUTION_PLAN.md
10. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/README.md
11. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/wfusion.toml
12. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/schemas/wparse_semantic.wfs
13. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/rules/wparse_semantic.wfl
14. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/scripts/build_wparse_events.py
15. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/sinks/defaults.toml
16. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/sinks/sink.d/file_json.toml
17. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/sinks/business.d/semantic.toml
18. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/sinks/business.d/catch_all.toml
19. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/scripts/run_file_case.sh
20. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/view/demo_log_dashboard.py

## 5. 待办列表
1. 优化时间图视觉层次（网格线、坐标刻度、hover 提示）。
2. 增加 Stage 说明卡组件化展示（主动作、异常数、持续时长）。
3. 增加 Turning Points 与 Causal Chain 叙事模块。
4. 支持阈值参数在线调节（boundary_threshold、min_segment_events）。
5. 对接 wp-reactor 输出接口，替换本地文件直读。

## 6. 建议执行顺序
1. 先做 CLI 计算正确性。
2. 再做 Slint 主图渲染。
3. 最后做叙事层与性能优化。

## 7. 变更记录
- 2026-03-09: 首版文档创建。
- 2026-03-09: 初始化 Rust + Slint 工程骨架。
- 2026-03-09: 修复 Slint Button 导入并完成 cargo check 验证。
- 2026-03-09: 接入 demo.json / wparse.log 真实数据读取与状态展示。
- 2026-03-09: 完成首版真实日志数据看板 UI（非占位区块）。
- 2026-03-09: 完成 Stage+Entity 单图、点击过滤与点位下钻交互。
- 2026-03-09: 增加时间刻度/网格与 Stage 卡片，提升可读性与叙事性。
- 2026-03-09: 完成日志面板滚动支持、窗口尺寸控制与点位渲染优化。
- 2026-03-09: 增加点位 Hover 预览能力，增强下钻效率。
- 2026-03-09: 完成整体布局重构，降低视觉混乱并强化主次分区。
- 2026-03-09: 新增 WFusion 执行方案文档，沉淀运行路径与回退机制。
- 2026-03-09: 新增 case/wparse 规则包与数据转换脚本，打通 wparse 语义计算落地路径。
- 2026-03-09: 验证 file source 进数链路并补充一键执行脚本。
- 2026-03-10: 完成 wparse 规则改版（fixed window close 输出），提升故事化展示可读性。
- 2026-03-10: 完成设计与任务文档归档，准备切入新任务阶段。
- 2026-03-10: 完成 `demo_log_dashboard.py` 迁移到 warp-diagnose 并修正默认数据入口。
