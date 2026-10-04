# iOS 导出文件操作

应用内部使用的本地 Tauri 2 插件，仅在 iOS 构建中启用。

Rust 的 `FileActionsExt::file_actions().open(paths, preview)` 调用 Swift：

- `preview = true`：使用 Quick Look 预览 PDF。
- `preview = false`：使用系统分享面板分享文件，支持“存储到文件”。

路径由应用的 `open_exported_files` 命令先行校验。插件没有直接暴露给前端的命令，也不需要额外的文件访问权限。`build.rs` 的 `ios_path` 让 Tauri 自动把本地 Swift package 接入 Apple 工程。

详见 [手机界面与导出修复](../../docs/mobile-and-export.md)。
