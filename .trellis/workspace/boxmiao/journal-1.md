# Journal - boxmiao (Part 1)

> AI development session journal
> Started: 2026-05-15

---



## Session 1: 重构日志保留策略与请求日志简化

**Date**: 2026-05-18
**Task**: 重构日志保留策略与请求日志简化
**Branch**: `main`

### Summary

将日志保留从按天数改为按数量(max_log_entries)，移除stats_anchor累计模式改为实时统计，移除请求日志时间筛选器简化为纯分页

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `1fc3bea` | (see git log) |
| `8d68ea0` | (see git log) |
| `f42c839` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 2: 设置抽屉版本号与 GitHub 图标

**Date**: 2026-05-19
**Task**: 设置抽屉版本号与 GitHub 图标
**Branch**: `main`

### Summary

实现设置抽屉显示版本号和 GitHub 跳转图标，新增 /api/version 端点，升级版本至 v1.1.4，添加 README 截图

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `538b7c0` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 3: 版本号显示、GitHub 图标、Release 工作流

**Date**: 2026-05-19
**Task**: 版本号显示、GitHub 图标、Release 工作流
**Branch**: `main`

### Summary

设置抽屉添加版本号和 GitHub 跳转图标，升级至 v1.1.4，添加 README 截图，新增 Release 工作流自动发布 GitHub Release

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `538b7c0` | (see git log) |
| `06686ae` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 4: 健康检查全端点探测与 tooltip 修复

**Date**: 2026-05-19
**Task**: 健康检查全端点探测与 tooltip 修复
**Branch**: `main`

### Summary

health_check API 改用 check_all() 探测所有端点并立即刷新前端状态；修复 tooltip 被 overflow:hidden 裁剪和 scoped CSS 优先级问题

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `5fb1bc3` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 5: Fix cumulative stats decreasing after log cleanup

**Date**: 2026-05-19
**Task**: Fix cumulative stats decreasing after log cleanup
**Branch**: `main`

### Summary

修复总调用次数/总字符数在 cleanup_old_logs 后减少的 bug。恢复 stats_anchor 累计计数：log_translation 递增 anchor，get_current_log_totals 从 anchor 读取，init 迁移补种已有数据。

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `3eab7b5` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 6: Fix three tested issues + add 64 unit tests

**Date**: 2026-05-19
**Task**: Fix three tested issues + add 64 unit tests
**Branch**: `main`

### Summary

修复 page_size clamp、source_lang 被忽略、source_chars 列迁移三个问题，添加 64 个单元测试覆盖全部核心模块

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `b6854df` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 7: Fix outdated documentation

**Date**: 2026-05-19
**Task**: Fix outdated documentation
**Branch**: `main`

### Summary

修正 CLAUDE.md 和 README.md 中 3 处过时/缺失的描述

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `00ac1bd` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete


## Session 8: 修复前端 UI 三个问题

**Date**: 2026-05-20
**Task**: 修复前端 UI 三个问题
**Branch**: `main`

### Summary

缓存卡片添加今天/总计切换（后端新增每日缓存计数器），调用趋势图表等高分布，暂无数据文字居中

### Main Changes

(Add details)

### Git Commits

| Hash | Message |
|------|---------|
| `c977c8b` | (see git log) |

### Testing

- [OK] (Add test results)

### Status

[OK] **Completed**

### Next Steps

- None - task complete
