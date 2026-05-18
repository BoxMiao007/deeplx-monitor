# 设置抽屉添加版本号和 GitHub 跳转图标

## Goal

在设置抽屉的 header 中展示项目版本号，并将关闭按钮（×）替换为 GitHub 图标，点击跳转到项目 GitHub 页面。

## Requirements

* 在"设置"标题旁边显示当前版本号（如 v1.0.0）
* 将右侧的 × 关闭按钮替换为 GitHub 图标（SVG）
* 点击 GitHub 图标在新标签页打开 https://github.com/BoxMiao007/deeplx-monitor
* 版本号从后端获取（通过 Cargo.toml 编译时嵌入）

## Acceptance Criteria

* [ ] 设置抽屉 header 显示"设置 v1.0.0"格式的版本号
* [ ] × 按钮被 GitHub SVG 图标替代
* [ ] 点击 GitHub 图标在新标签页打开项目 GitHub 页面
* [ ] 抽屉仍可通过点击遮罩层关闭

## Technical Approach

* 后端：新增 `/api/version` 端点，返回 `env!("CARGO_PKG_VERSION")` 编译时版本
* 前端：store 中添加 `fetchVersion()` 方法，设置抽屉打开时获取版本
* 前端：drawer-header 布局调整为 [设置 vX.X.X] ... [GitHub SVG icon → link]
* GitHub 图标使用内联 SVG（不引入图标库）

## Out of Scope

* 检查更新功能
* 版本号点击交互
