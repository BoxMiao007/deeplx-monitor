# 拆分“今天”和“24h”时间范围

## Goal

澄清并实现时间范围筛选语义：`今天` 应表示本地自然日从 00:00 到当前时间；现有滚动 24 小时的计算方式需要保留，并作为额外的 `24h` 选项提供。

## What I already know

* 用户显式触发了 `trellis-brainstorm`。
* 项目为 single-repo 模式，已配置 backend 规范层。
* 当前工作区存在未提交改动：`.trellis/.template-hashes.json`、`AGENTS.md`、`CLAUDE.md`、`.agents/`、`.codex/`。
* `frontend/src/pages/Overview.tsx` 的“范围”下拉当前将 `今天` 配置为 `value: 1`。
* `frontend/src/stores/useStatsStore.ts` 会把该值作为 `days=1` 传给 `/api/stats`。
* `src/api.rs` 中 `/api/stats` 的 `days=1` 会调用 `get_period_stats_filtered(1, ...)`。
* `src/db.rs` 中 `get_period_stats_filtered(1, ...)` 使用 `datetime('now', '-1 days', 'localtime')`，实际语义是滚动 24 小时，不是自然日 00:00 起算。
* `frontend/src/pages/Overview.tsx` 的图表请求 `/api/chart?days=1` 会使用 `hourly` 数据，后端 `/api/chart` 的 `hourly` 固定来自最近 24 小时。
* `frontend/src/pages/Analysis.tsx` 的错误趋势已有独立 `24h` 选项；热力图有一个复用 `overview.today` 文案的 `今天` 选项，需统一语义。
* `frontend/src/components/charts/Heatmap.tsx` 当前直接请求 `/api/analytics/timeline?days=...`，因此 Analysis 热力图的 `今天` 实际影响 timeline 数据和 tooltip 时间窗口。

## Assumptions (temporary)

* `今天` 与 `24h` 需要在接口参数中可区分，不能继续只依赖 `days=1` 表达两种语义。

## Open Questions

* 无

## Requirements (evolving)

* Overview 页面“范围”下拉新增 `24h` 选项，保留现有滚动 24 小时统计方式。
* Overview 页面“范围”下拉中的 `今天` 改为本地自然日，从当天 00:00 起算到当前时间。
* Analysis 页面中使用 `今天` 文案的热力图筛选也采用同样的自然日语义，包括 timeline 数据请求和 tooltip 时间窗口。
* `7 天`、`30 天`、`90 天`、`全部` 的语义保持不变。
* 前后端参数需要明确区分自然日 `今天` 与滚动 `24h`，避免 `days=1` 同时代表两种含义。
* 中英文文案需要同步更新。

## Acceptance Criteria (evolving)

* [ ] Overview 页面“范围”控件同时提供 `今天` 和 `24h`。
* [ ] 选择 `今天` 时，总量统计和趋势图只包含本地当天 00:00 之后的数据。
* [ ] 选择 `24h` 时，总量统计和趋势图保持当前滚动 24 小时行为。
* [ ] Analysis 页面中所有显示为 `今天` 的筛选与 Overview 保持同一自然日语义。
* [ ] 选择 `7 天`、`30 天`、`90 天`、`全部` 时行为不回退。
* [ ] 中英文界面文案一致，且不再把滚动 24 小时显示为 `今天`。
* [x] 用户确认完整需求后再进入实现准备阶段。

## Definition of Done (team quality bar)

* Tests added/updated (unit/integration where appropriate)
* Lint / typecheck / CI green
* Docs/notes updated if behavior changes
* Rollout/rollback considered if risky

## Out of Scope (explicit)

* 不改变日志保留策略。
* 不改变端点健康检查、缓存统计、语言统计等非范围筛选相关逻辑。
* 在 PRD 收敛并获得确认前，不运行 `task.py start`。

## Technical Approach

优先把时间范围语义从“单纯 days 数字”升级为显式范围值，避免 `days=1` 同时承载自然日和滚动 24 小时两种含义。前端根据用户选择传递明确范围，后端按范围分支查询对应时间窗口；Analysis 热力图使用的 timeline 接口也需要支持同样范围语义。

## Decision (ADR-lite)

**Context**: 当前用 `days=1` 同时表达“自然日今天”和“滚动 24h”，会让前后端语义和 UI 文案错位。

**Decision**: 将 `今天` 固定为本地自然日口径，并新增独立 `24h` 选项承载滚动 24 小时统计。

**Consequences**: 需要前后端都区分时间语义，相关下拉文案和接口参数要同步调整；但后续不会再把两种不同口径混用在一个值里。

## Technical Notes

* 已读取 `.agents/skills/trellis-start/SKILL.md`。
* 已读取 Trellis phase context：当前处于 Phase 1 planning 前置状态。
* 已读取 `.trellis/spec/guides/index.md`。
* 已读取 `.trellis/spec/backend/index.md`。
* 已检查 `frontend/src/pages/Overview.tsx`、`frontend/src/pages/Analysis.tsx`、`frontend/src/components/charts/Heatmap.tsx`、`frontend/src/stores/useStatsStore.ts`、`src/api.rs`、`src/db.rs` 的相关实现。
