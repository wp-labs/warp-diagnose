# Upstream Defect Record: `match ... and close` 聚合字段求值问题

状态：已修正

记录日期：2026-03-11

## 结论

此前 `wfusion` 在 `match ... and close` 路径上，聚合表达式存在两类问题：

1. `-> score(expr)` 使用 `count(...)`、`avg(...)` 等聚合表达式时，close 阶段可能报：
   - `score expression evaluated to None`
2. `yield (...)` 中依赖聚合函数的字段，可能被静默丢弃，最终输出里只剩直接字段

该问题现已在上游修正。

## 历史现象

受影响的规则形态：

```wfl
match<subject:1s:fixed> {
  on event {
    x | count >= 1;
  }
  and close {
    x | count >= 1;
  }
} -> score(avg(x.risk_score))
```

以及：

```wfl
yield entity_stats (
  subject = x.subject,
  avg_risk_score = avg(x.risk_score),
  event_count = count(x),
  high_event_count = count(hi),
  elevated_event_count = count(elevated)
)
```

历史上会出现：

- `subject` 这类直接字段正常输出
- `event_count`、`avg_risk_score`、`status = if count(...) ...` 这类聚合字段缺失
- 或 `score(...)` 在 close 阶段直接失败

## 历史定位

问题当时定位在 close 路径的 `yield` 求值逻辑：

- `/Users/zuowenjian/devspace/wp-labs/wp-reactor/crates/wf-core/src/rule/executor/close_exec.rs`

当时 `yield` 字段通过 `filter_map(...)` 构造，表达式求值失败会被静默跳过，因此外部表现为“字段消失但不报错”。

## 当前文档用途

本文件保留为历史缺陷记录，用于说明：

- 为什么本 case 一度采用过 `score(50.0)` 之类的兼容写法
- 为什么此前会观察到 `wf-alert.json` / `wf-entity.json` 中聚合字段丢失

如果当前上游版本已经包含修复，应优先按正确业务语义编写规则，不再需要因为这个历史问题而主动规避 close 聚合表达式。
