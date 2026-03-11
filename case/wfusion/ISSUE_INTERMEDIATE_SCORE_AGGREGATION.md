# Issue: downstream `match + close` cannot aggregate intermediate `__wfu_score` / float fields correctly

Date: 2026-03-11

## Summary

In current `wfusion`, an upstream `on each -> score(...)` rule can emit intermediate records successfully, but a downstream `match ... and close` rule does not aggregate the intermediate score/value columns correctly.

The main symptom is:

- `avg(x.__wfu_score)` evaluates to empty and falls back to `0.0`
- `avg(x.risk_score)` also evaluates to empty and falls back to `0.0`

This happens even though:

- the intermediate rule runs successfully
- downstream compilation accepts `x.__wfu_score`
- runtime docs explicitly say intermediate `__wfu_score` should be available downstream

## Expected

Given:

```wfl
rule event_semantic_project {
  events {
    e : wparse_events && subject != ""
  }

  on each e -> score(
    if lower(e.status) in ("error", "failed", "failure", "timeout", "fatal", "panic", "abort")
    then 90.0 else if lower(e.status) in ("miss", "disabled", "retry", "partial", "degraded")
    then 80.0 else if lower(e.status) in ("warn", "warning")
    then 70.0 else if lower(e.status) in ("success", "suc", "ok", "end", "enabled", "done", "pass")
    then 20.0 else 40.0
  )

  entity(service, e.subject)

  yield semantic_events (
    event_time = e.event_time,
    level = e.level,
    target = e.target,
    subject = e.subject,
    action = e.action,
    status = e.status,
    risk_score = @score,
    content = e.content,
    source = e.source
  )
}
```

and downstream:

```wfl
rule window_risk {
  events {
    x : semantic_events
  }

  match<subject:1s:fixed> {
    on event {
      x | count >= 1;
    }
    and close {
      x | count >= 1;
    }
  } -> score(coalesce(avg(x.__wfu_score), 0.0))

  entity(service, x.subject)

  yield risk_alerts (
    subject = x.subject,
    risk_score = coalesce(avg(x.risk_score), 0.0),
    event_count = coalesce(count(x), 0)
  )
}
```

The downstream window should produce:

- non-zero `__wfu_score` when upstream `__wfu_score` is non-zero
- non-zero `risk_score` when upstream `risk_score` is non-zero
- values matching the average over grouped intermediate rows

## Actual

The case runs successfully, but downstream aggregated scores are all `0.0`.

Observed output example from:

- `case/wfusion/alerts/wf-alert.json`

```json
{"__wfu_rule_name":"window_risk","__wfu_score":0.0,"subject":"SinkGroup","status":"low","risk_score":0.0,"event_count":3}
```

This is incorrect because upstream `event_semantic_project` assigns non-zero per-event scores (`90 / 80 / 70 / 20 / 40`).

## Reproduction

Repository:

- `/Users/zuowenjian/devspace/wp-labs/warp-diagnose`

Run:

```bash
cd /Users/zuowenjian/devspace/wp-labs/warp-diagnose
./case/scripts/run_wp_wf_case.sh
```

Current relevant files:

- `case/wfusion/rules/wparse_semantic.wfl`
- `case/wfusion/schemas/wparse_semantic.wfs`
- `case/scripts/run_wp_wf_case.sh`

Current `wfusion` binary used by the script:

- `../wp-reactor/target/debug/wfusion`

## Why this looks like a runtime bug

### 1. Docs explicitly say downstream can use `x.__wfu_score`

Refs:

- `/Users/zuowenjian/devspace/wp-labs/wp-reactor/docs/user-guide/runtime-config.md`
- `/Users/zuowenjian/devspace/wp-labs/wp-reactor/docs/user-guide/on-each.md`

Relevant documented behavior:

- intermediate records should expose `__wfu_score`
- downstream rules can use `avg(x.__wfu_score)`
- no need to redeclare `__wfu_score` in `.wfs`

### 2. Compiler/checker already models this as valid

Ref:

- `/Users/zuowenjian/devspace/wp-labs/wp-reactor/crates/wf-lang/src/checker/intermediate.rs`

`effective_schemas_for_rules(...)` automatically injects:

- `__wfu_score`
- `__wfu_rule_name`
- `__wfu_entity_type`
- `__wfu_entity_id`

into intermediate targets that are consumed downstream.

### 3. Runtime bootstrap uses the effective schemas

Ref:

- `/Users/zuowenjian/devspace/wp-labs/wp-reactor/crates/wf-runtime/src/lifecycle/bootstrap.rs`

The runtime does not build windows only from raw `.wfs`; it uses `effective_schemas`.

So this does not look like a config/schema omission in the case.

### 4. The issue is not just `__wfu_score`

The downstream rule also aggregates:

```wfl
risk_score = coalesce(avg(x.risk_score), 0.0)
```

and this also becomes `0.0`.

So the bug may be broader:

- downstream `match + close` aggregation over intermediate float fields may be losing values
- or intermediate rows may be stored without usable float values for downstream aggregation

## Current evidence

Case output after running `./case/scripts/run_wp_wf_case.sh`:

- `case/wfusion/alerts/wf-alert.arrow`
- `case/wfusion/alerts/wf-alert.json`
- `case/wfusion/alerts/wf-entity.arrow`
- `case/wfusion/alerts/wf-entity.json`

The run succeeds, but score-like aggregated fields collapse to `0.0`.

## Suspected areas

Likely places to inspect:

- intermediate window row construction
  - `/Users/zuowenjian/devspace/wp-labs/wp-reactor/crates/wf-runtime/src/engine_task/rule_task.rs`
- downstream window row -> event conversion
  - `/Users/zuowenjian/devspace/wp-labs/wp-reactor/crates/wf-core/src/rule/event_bridge.rs`
- match/close aggregation evaluation over intermediate float fields

## Suggested upstream checks

1. Add an end-to-end runtime test:

   - rule A: `on each -> score(...)`, yield intermediate record with business float field
   - rule B: `match + close`, compute:
     - `avg(x.__wfu_score)`
     - `avg(x.some_float_field)`
     - `count(x)`
   - assert downstream output is non-zero and numerically correct

2. Verify that intermediate window snapshots really contain:

   - `__wfu_score`
   - business float fields such as `risk_score`

3. Verify close aggregation reads those float columns correctly from intermediate rows.

## Minimal impact statement

This blocks a common rule pattern documented by upstream:

- `on each` for per-event scoring
- downstream `match + close` for subject/window aggregation

Without this, users can compile documented patterns like `avg(x.__wfu_score)`, but runtime output does not reflect actual upstream scores.
