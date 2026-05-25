# 上游端点添加总字符数统计

## 目标

在"上游端点"卡片中为每个端点单独显示总字符数（源字符 + 目标字符），以便用户了解各端点的实际翻译负载。

## 需求

1. **后端**：在 `EndpointState` 中新增 `total_chars` 原子计数器，在 `report_success` 时累加字符数。
2. **后端**：在 `EndpointStatus` 中暴露 `total_chars` 字段，通过 `/api/upstream/status` 返回给前端。
3. **前端**：在每个端点卡片中显示总字符数，格式化为可读数字（如 1.2K、3.4M）。

## 设计要点

- `report_success` 需要接收字符数参数（source_chars + target_chars）。
- `proxy.rs` 在翻译成功时将字符数传递给 `report_success`。
- `reload()` 时计数器归零（与 total_requests 行为一致）。
- 前端复用已有的数字格式化逻辑。

## 不做

- 不持久化到数据库（与 total_requests 一致，仅内存统计）。
- 不区分源/目标字符（合并为一个总数）。
