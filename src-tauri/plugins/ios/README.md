# iOS 文件面板

`Sources/FileActionsPlugin.swift` 接收导出文件 URL，在主线程展示 `QLPreviewController` 或 `UIActivityViewController`。插件保留 Quick Look 的数据源，并为 iPad 分享面板提供 popover 锚点；已有预览或分享面板时返回错误，避免重复展示。

通过应用的 `pnpm tauri ios init` / `pnpm tauri ios build` 构建；Tauri 自动提供 `../.tauri/tauri-api` package。Swift 和 UIKit 行为需要 macOS / 真机验证。
