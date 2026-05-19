# 修复总调用次数/总字符数在清理日志后减少的 bug

## 问题

Dashboard 显示的"总调用次数"和"总字符数"在使用过程中会出现减少的情况。

## 根因

- `总调用次数/总字符数` 通过 `SELECT COUNT(*), SUM(chars) FROM translation_logs` 实时计算（`db.rs:get_current_log_totals()`）
- 后台 `cleanup_old_logs()` 每小时删除超出 `max_log_entries` 的旧行
- 删除后 COUNT/SUM 自然减少，导致用户看到"总量"下降

## 修复方案

恢复 `stats_anchor` 表的累计计数功能：

1. `log_translation()` — 每次插入日志时同步递增 `stats_anchor.total_requests` 和 `stats_anchor.total_chars`
2. `get_current_log_totals()` — 改为从 `stats_anchor` 读取累计值，不再实时 COUNT
3. `init()` — 添加迁移逻辑：若 anchor 为 0 但已有日志，从现有日志补种累计值（兼容已有数据库）
4. `replace_with_demo_data()` — 插入 demo 数据后同步 anchor

## 验证标准

- `cleanup_old_logs()` 执行后，`/api/stats` 返回的 `total_requests` 和 `total_chars` 不减少
- 新翻译请求后，总量正确递增
- Demo 模式重置后，anchor 与实际日志一致
