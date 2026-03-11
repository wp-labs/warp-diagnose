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
27. 完成交互与布局重构归档:
   - `INFO / WARN / ERROR` 改为数据过滤选择器
   - 移除固定 `Inspector` 与 `Recent Event Stream` 主路径
   - 点击时间线点位后在下方 `Selection Detail` 展示证据
   - `Hover Preview` 改为点位附近浮层
   - 时间线支持平移、缩放与图标化导航
   - 整体主题切换为浅色版本
28. 完成过滤与工具栏二次重构:
   - 顶部 KPI 改为风险区间统计，不再使用 `INFO / WARN / ERROR`
   - 过滤体系扩展为 `Level / Risk / Source / Stage` 四维
   - `Filter` 优先收敛为单行
   - `Active` 行增加彩色过滤标签与图标化 `Clear All`
   - 时间线功能按钮减化为语义图标
   - 顶部增加程序 LOGO，窗口最大化回归系统标准控件
29. 完成交互调整文档归档细化:
   - 在产品文档中补充交互调整归档段落
   - 在技术文档中补充交互实现约束与窗口行为说明
   - 清理旧版 `Home/Now` 等已过时描述
30. 打通本地 `wparse -> wfusion -> warp-diagnose` 链路:
   - 将 `wp-self` 的 `conf / models / topology` 复制到 `case/wparse`
   - 补齐当前版本 `wparse` 所需 `connectors`
   - `run_file_case.sh` 升级为端到端脚本，支持直接从目标日志跑到本工程产物目录
   - `warp-diagnose` 默认路径切换为优先读取本工程 `case/wparse` 产物
31. 收敛 Arrow 持久化链路:
   - 明确 `wp = log`、`wf = alert`
   - `warp-diagnose` 增加 Arrow 读取能力，默认优先消费 `wp-log.arrow` 与 `wf-alert.arrow`
   - 新增 `wp_arrow_to_ndjson` 本地转换工具
   - 新增 `wf_alert_json_to_arrow`，将 `wfusion` alert JSONL 归一化为 Arrow
   - 联调脚本改为 `wp-log.arrow -> wparse_events.ndjson -> wf-alert.jsonl -> wf-alert.arrow`
   - 文档补充当前 `wparse` 与 `wfusion alert sink` 的 Arrow 运行时限制
32. 拆分 `wparse / wfusion` case 目录:
   - `case/wparse` 只保留 `wparse` 工作根与 `wp` 日志产物
   - `case/wfusion` 承接 `wfusion` 规则、sink、日志与 `wf` 告警产物
   - 联调脚本上提到 `case/scripts/run_wp_wf_case.sh`
   - `warp-diagnose` 默认路径切换为 `case/wparse + case/wfusion` 双目录结构
33. 切换为 `wparse` 直接 Arrow 输出:
   - `case/wparse/topology/sinks/business.d/demo.toml` 改为 `arrow_file_sink`
   - `wparse` 直接写 `case/wparse/data/out_dat/wp-log.arrow`
   - 删除 `demo.json -> wp-log.arrow` 这段本地转换依赖
34. 增加双分页日志阅读模式:
   - 默认第一页保持现有 `Overview` 时间线布局
   - 新增第二页 `Log Data`，按当前过滤条件展示结构化输入日志表
   - `log_events` 同步补齐 stage 映射，确保 `Stage / Level / Risk / Source` 过滤在日志页也生效
35. 更新 `wfusion` Arrow 输入链路:
   - 根据 `wp-reactor/docs/user-guide/runtime-config.md`，确认 `file source` 已支持 `arrow_framed`
   - `case/wfusion/wfusion.toml` 改为读取 `data/in_dat/wp-log.arrow`
   - 在 `wparse` 侧通过 OML + sink 字段清单直接收敛到 `wfusion` 期望 schema
   - 顶层脚本移除 `wp_arrow_to_ndjson` 与 `wp_arrow_to_wf_arrow` 中间转换步骤
36. 验证 `wfusion` 直接 Arrow 输出限制:
   - 尝试将 `case/wfusion` sink 从 `file_json` 切到 `arrow_file`
   - 运行时确认当前 `wfusion` alert sink 链路输出的是 JSON 字符串，不是 record
   - `arrow_file` 现阶段不可直接接收 alert 输出，临时保留 `wf_alert_json_to_arrow` 桥接
   - 联调脚本继续采用 `wf-alert.jsonl -> wf-alert.arrow` 转换

## 3. 当前结论
1. 技术路线确定为 Rust 原生 GUI: Slint。
2. 计算层依托 wp-reactor，可通过 bridge 复用。
3. 当前本地 case 已统一为 Arrow 持久化优先，保留 JSON/NDJSON 仅作为 `wparse` 兼容中间态。
4. 当前工程已可编译，并能读取真实样例数据。
5. 当前主交互已经从“日志流阅读”转向“多维筛选 + 时间线下钻 + 点选证据展示”。
6. 当前已补充第二阅读路径，支持从时间线总览切换到条件化日志表。

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
10. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/README.md
11. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/scripts/run_wp_wf_case.sh
12. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wparse/README.md
13. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wfusion/README.md
14. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wfusion/wfusion.toml
15. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wfusion/schemas/wparse_semantic.wfs
16. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wfusion/rules/wparse_semantic.wfl
17. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wfusion/sinks/sink.d/file_json.toml
18. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wfusion/sinks/business.d/semantic.toml
19. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/case/wfusion/sinks/business.d/catch_all.toml
20. /Users/zuowenjian/devspace/wp-labs/warp-diagnose/view/demo_log_dashboard.py

## 5. 待办列表
1. 增加结构化 `Selection Detail` 卡片，替代纯文本明细。
2. 支持鼠标滚轮缩放与更细粒度导航。
3. 继续评估过滤条在窄窗口下的折行/收纳策略。
4. 增加 Turning Points 与 Causal Chain 叙事模块。
5. 支持阈值参数在线调节（boundary_threshold、min_segment_events）。
6. 对接 wp-reactor 输出接口，替换本地文件直读。
7. 继续优化 `Log Data` 页的查询条件表达与大数据量分页策略。

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
- 2026-03-10: 完成交互重构归档，明确当前基线为浅色主题、level 过滤、点选详情与浮动 hover 预览。
- 2026-03-10: 完成过滤体系与头部工具栏归档，明确当前基线为风险 KPI、四维过滤、图标化工具栏与 LOGO 头部。
- 2026-03-10: 细化交互调整文档，补充布局重排、时间线工具栏、过滤器分组配色与系统窗口行为记录。
- 2026-03-10: 将 `wp-self` 解析配置迁入 `case/wparse`，并打通本工程内 `wparse -> wfusion -> warp-diagnose` 执行链路。
- 2026-03-10: 完成 Arrow 链路收敛，明确 `wp-log.arrow` 与 `wf-alert.arrow` 为诊断侧默认输入。
- 2026-03-10: 完成 case 目录拆分，`wparse` 与 `wfusion` 改为双目录结构，并新增顶层联调脚本。
- 2026-03-10: 切换到 `wparse` 直接输出 `wp-log.arrow`，收掉 `demo.json -> Arrow` 的兼容转换。
- 2026-03-10: 新增 `Overview / Log Data` 双分页，支持按当前条件查看结构化输入日志。
- 2026-03-10: 确认 `wfusion` 已支持 `arrow_framed` file source，并切换到直接消费 `wp-log.arrow`。
- 2026-03-10: 将 Arrow schema 对齐逻辑前移到 `wparse` OML，删除中间 `wp_arrow_to_wf_arrow` 桥接工具。
- 2026-03-10: 验证 `wfusion` 直接 Arrow 输出受限，暂时保留 `wf_alert_json_to_arrow` 作为桥接。
