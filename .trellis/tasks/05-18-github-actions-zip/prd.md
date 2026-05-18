# GitHub Actions 手动触发三端构建并打包 zip

## Goal

创建 GitHub Actions workflow，支持手动触发（workflow_dispatch），在 Linux x86_64、macOS ARM64、Windows x86_64 三个平台上构建 release 二进制，并将每个平台的产物打包为 zip 上传为 artifact。

## Requirements

* workflow_dispatch 手动触发
* 三平台原生构建：
  - Linux x86_64 (ubuntu-latest)
  - macOS ARM64 (macos-latest, Apple Silicon)
  - Windows x86_64 (windows-latest)
* 构建步骤：先 npm ci + npm run build 生成 dist/，再 cargo build --release
* 每个平台产物打包为 zip 并上传为 GitHub Actions artifact
* zip 内含：二进制文件 + config.toml.example

## Acceptance Criteria

* [ ] 手动触发 workflow 后，三个平台均构建成功
* [ ] 每个平台生成一个 zip artifact 可下载
* [ ] zip 解压后包含可执行文件和 config.toml.example
* [ ] artifact 命名清晰标识平台（如 deeplx-monitor-linux-x86_64.zip）

## Definition of Done

* Workflow 文件语法正确
* 推送后可在 GitHub Actions 页面手动触发

## Technical Approach

* 使用 matrix strategy 定义三个平台
* 每个 job：checkout → setup node → npm ci + build → setup rust → cargo build --release → 打包 zip → upload artifact
* 二进制名：Linux/macOS 为 `deeplx_monitor`，Windows 为 `deeplx_monitor.exe`
* Rust toolchain: stable
* Node: 20

## Out of Scope

* 自动触发（push/tag/release）
* 代码签名
* Docker 镜像构建
* 自动发布 GitHub Release
* 交叉编译

## Technical Notes

* `rust-embed` 要求 `dist/` 在 cargo build 前存在
* rusqlite 使用 bundled feature，无系统依赖
* reqwest 使用 rustls-tls，无 OpenSSL 依赖
* macOS runner (macos-latest) 现在默认是 ARM64
