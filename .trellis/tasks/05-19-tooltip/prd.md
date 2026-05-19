# 上游端点健康状态 tooltip

## Goal

在上游端点详情面板中，鼠标悬停"健康/不健康" badge 时显示 tooltip，展示该端点最近一次健康检查的错误信息，帮助用户判断为什么端点不健康。

## Requirements

* 后端：每个端点存储最近一次失败的错误信息（`last_error`）
* API：`/api/upstream/status` 响应中包含 `last_error` 字段
* 前端：hover "不健康" badge 时 tooltip 显示错误信息
* 健康时不需要 tooltip（或显示"正常"）

## Acceptance Criteria

* [ ] `EndpointStatus` 包含 `last_error: Option<String>` 字段
* [ ] `report_failure` 接受错误信息并存储
* [ ] `probe_unhealthy` 失败时也存储错误信息
* [ ] 前端 hover 不健康 badge 时显示错误原因
* [ ] 前端构建通过

## Definition of Done

* 前端构建通过
* 后端编译通过

## Out of Scope

* 显示延迟、成功率等统计（已有）
* per-endpoint 独立健康检查 API
* 健康检查历史记录

## Technical Notes

* `upstream.rs`: `EndpointState` 添加 `last_error: RwLock<Option<String>>`
* `proxy.rs`: `report_failure` 调用时传入错误信息
* `frontend/src/stores/monitor.ts`: `EndpointStatus` 接口添加 `last_error`
* `frontend/src/pages/Dashboard.vue`: badge 添加 `data-tooltip` + CSS
