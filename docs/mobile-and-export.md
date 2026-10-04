# 手机界面与导出修复

## 导出完整性

下载清单支持 JPG、JPEG、PNG、WebP、GIF，遇到空清单或未知格式会报错。恢复下载时会解码已有图片，避免把中断写入的文件算作完成；图片和元数据都在同目录的临时文件中写完，再原子替换。

新下载的章节元数据保存 `pageCount`。PDF / CBZ 导出会先从磁盘同步章节状态，再校验图片数量和解码结果。旧章节元数据没有这个字段时仍可导出，但无法仅凭旧文件判断历史下载是否曾静默丢页。

PDF 创建和合并拒绝空输入；合并时对象编号从 1 开始，保持章节顺序。GIF 在静态 PDF 中使用第一帧。章节 PDF 和整本 PDF 都先写临时文件，重新读取并检查页数、图片资源和 `Do` 绘制引用，通过后才替换正式文件。失败时保留原有 PDF；重复导出同一漫画的同一格式会被拒绝。

PDF / CBZ 逐章生成，减少同时解码多个章节的内存压力。写完后释放内存中的 PDF，再重新读取校验。整本合并仍需要内存容纳最终 PDF，大型漫画还应在目标设备上检查峰值内存。

已验证用户提供的真实单章案例（漫画 ID `1236729`）：原 PDF 仅 237 字节，`Pages.Count` 为 0，没有图片对象。用旧合并逻辑构造空章节输入，生成的文件与原 PDF 逐字节相同。旧代码会在没有已完成章节时继续合并并报告成功；修复后的导出先同步磁盘状态，再拒绝空章节、空页面文档。

这个案例的 PDF 修改时间为 2026-03-28 00:48:34.931，最后一张源图片为 00:48:42.335，章节完成元数据为 00:48:46.346。用户补充：当时已看到“下载完成”，随后立即导出。文件修改时间无法还原界面提示、内存状态或多次操作的顺序，不能据此认定用户在下载完成前点击导出。

下载完成事件、前端任务状态和漫画章节状态原先通过不同步骤更新：前端先设任务为 `Completed`，再等待异步命令刷新漫画对象；旧导出又直接使用前端传入的章节完成标记。后台刷新返回较慢或失败时，任务已完成而漫画对象仍可能滞后；刷新期间切换漫画，也可能被旧响应重新打开上一部漫画。此外，旧后端保存章节元数据失败时仅记录日志，仍然继续发送 `Completed`，这也能造成“提示完成但导出找不到完成章节”。当前后端在写完图片、目录迁移和章节完成元数据后才发送 `Completed`，元数据保存失败则转为 `Failed` 并立即返回。

前端收到完成事件后，现在同步更新任务和对应漫画的章节完成标记、页数及实际下载路径，再启动后台列表刷新；提示与导出按钮不依赖刷新延迟。刷新响应仅在用户仍持有原漫画对象时应用，并合并已经收到的完成事件，避免旧结果倒退章节状态。后端导出仍以磁盘上的完成元数据为准，并拒绝空输入。

现有 51 张 JPG 均通过解码校验，使用当前创建和合并函数重建出 51 页、16,555,277 字节的副本。独立 PyMuPDF 解析确认无需修复，各页绘制引用和尺寸有效；每页内嵌 JPEG 与相应源文件逐字节一致，页序为 `0001.jpg` 至 `0051.jpg`。该诊断未渲染、预览、OCR 或进行视觉识别，原 PDF、源图片和元数据的内容与修改时间均未改变。旧元数据未记录服务器应有页数，因此这证明现有 51 张图片完整进入 PDF，不代表独立核实了服务器上的总页数。

## 手机操作

- 底部四个入口：发现、收藏、书库、任务。每周必看位于发现页。
- 章节作为独立页面打开，可返回原列表；章节独立滚动，下载和导出按钮固定在底部。
- 使用实际按钮和勾选操作；手机上不启用鼠标框选，任务有暂停、继续、重试和取消按钮。
- 设置使用可滚动标签表单，窄屏数字输入不再与长标签共挤一行。
- 常规页面用 CSS 固定到 WebView 的上下边缘，预留设备安全区域；仅在文本输入框获得焦点且视口明显缩小时，使用 `visualViewport` 避让键盘。首次启动、恢复前台和旋转屏幕不会把偏小的初始视口高度固化为页面高度，键盘关闭后也不依赖视口事件恢复底栏。短横屏使用更紧凑的章节操作区。
- iOS 隐藏桌面文件夹选择和文件管理器入口。书库及导出任务提供原生 PDF 预览、分享和“存储到文件”。

## iOS 原生文件操作

本地插件位于 `src-tauri/plugins`，通过目标平台依赖仅在 iOS 注册。Rust 的 `open_exported_files` 检查导出目录、文件类型及可读文件，再调用 Swift 插件。Swift 在主线程展示 Quick Look 或 `UIActivityViewController`；iPad 使用 popover 锚点。

分享使用应用沙盒里的文件 URL，不依赖 WebView 的 iframe 或桌面 opener。插件由 Tauri 构建脚本加入生成的 Apple 工程，无需提交 `src-tauri/gen/apple`。保留现有 iOS workflow，可在 macOS 上执行 `pnpm tauri ios init` 和构建。

本地环境为 Windows，iOS 构建由现有 GitHub Actions 在 macOS 上执行。提交 `3571492` 已通过原生编译并生成 IPA，用户反馈手机基本操作可用，但首次竖屏打开时底栏偏高，横屏后再竖屏会恢复。提交 `d35804f` 移除了正常布局对初始 `visualViewport.height` 的依赖，浏览器回归通过，但用户真机反馈问题仍然存在。

按用户要求，iPhone 现在在 WebView 挂到活动窗口后，通过临时全屏原生控制器执行一次横屏、一次竖屏；确认对应方向完成布局后才进入下一步。切换结束撤销临时控制器，把 WebView 重新布局到父视图的完整边界，并通知前端重新读取视口。使用全屏控制器的 `preferredInterfaceOrientationForPresentation`，兼容当前 Tauri 无 `UIWindowScene` 的窗口；有场景且系统为 iOS 16 以上时，也可在方向尚未更新时请求场景几何更新。执行有超时和失活清理，只在本次 WebView 启动时执行一次，不锁定后续手动旋转，也不对 iPad 执行。

工作流增加独立 iPhone 模拟器应用验证，仅编译和运行同一个 `StartupRotation.swift`，不连接漫画 API 或读取用户文件。应用从高度少 80 的原生 WebView 开始，检查实际横屏和竖屏布局、临时控制器撤销、WebView 填满父视图、WebKit 视口及底栏恢复，以及重复调用不再次旋转。真机首次启动、安全区域和键盘效果仍需安装新版确认；Quick Look、分享与存储到文件，以及大型漫画内存占用也需真机验证。

## 本地验证

```powershell
pnpm build
cd src-tauri
$env:JM_EXPORT_TEST_DIR = '..\output\pdf'
cargo test --lib
cd ..
pnpm tauri build --no-bundle
```

Rust 默认回归共 13 项，覆盖图片清单、损坏图片、单章多格式 PDF、合并页序、导出失败保留旧文件、成功替换旧文件、无效绘制引用、重复导出保护、历史零页 PDF 拒绝，以及从磁盘刷新滞后的章节完成状态。设置 `JM_EXPORT_TEST_DIR` 会保留合成的单章样例 PDF，便于用独立阅读器检查。

真实本地文件诊断单独标记为 ignored，不会被默认测试读取。需要显式设置 `JM_LOCAL_COMIC_DIR`、`JM_LOCAL_EXPORT_DIR` 后，在 `src-tauri` 目录运行：

```powershell
cargo test --lib export::tests::rebuild_local_comic_without_rendering -- --ignored --exact --nocapture
```

两个变量必须指向已经存在、互不包含的目录。诊断只读源文件和完成元数据，在输出目录生成逐章 PDF、`rebuilt.pdf` 和用于故障对比的 `legacy-empty.pdf`；只输出结构统计，不展示图片。可选变量 `JM_LOCAL_ORIGINAL_PDF` 用于将原文件与旧逻辑的空文档逐字节对比。此测试不执行旧元数据迁移，也不修改下载目录。当前案例的统计和源文件保护校验保存在忽略的 `output/case-1236729/diagnosis.json`。

浏览器验证使用虚构漫画和 Tauri bridge，不连接漫画 API，不读取用户下载目录。安装 Playwright CLI 后：

```powershell
pnpm preview --host 127.0.0.1 --port 5006
```

在另一个终端中执行：

```powershell
npx --yes --package @playwright/cli playwright-cli -s=jm-ui open http://127.0.0.1:5006/
npx --yes --package @playwright/cli playwright-cli -s=jm-ui run-code --filename scripts/ui-fixtures.js
npx --yes --package @playwright/cli playwright-cli -s=jm-ui run-code --filename scripts/ui-smoke.js
npx --yes --package @playwright/cli playwright-cli -s=jm-ui run-code --filename scripts/ui-tasks.js
npx --yes --package @playwright/cli playwright-cli -s=jm-ui run-code --filename scripts/ui-library.js
npx --yes --package @playwright/cli playwright-cli -s=jm-ui run-code --filename scripts/ui-dialogs.js
```

模拟 iOS UA 检查 320×568、390×844、844×390 下的章节操作区及滚动；检查设置保存、PDF 预览与分享、CBZ 分享、添加任务和暂停 / 继续 / 重试 / 取消，以及批量导出、登录、收藏、每周必看和日志。测试脚本只确认 WebView 到原生命令的调用，不代表 UIKit 面板已在真机上验证。截图输出到忽略的 `output/playwright` 目录。

视口回归脚本 `scripts/ui-viewport.js` 模拟页面高 844、启动时 `visualViewport.height` 仅为 763 的场景，并检查恢复前台、横竖屏切换、键盘展开、键盘关闭后视口数值仍滞后、缩放和不支持 Visual Viewport 的情况。常规页面及底栏必须始终到达屏幕底部；键盘出现时输入区必须避让，关闭后底栏恢复。该脚本注入的视口替身在重新加载后仍然保留，应在独立浏览器会话中运行：

```powershell
npx --yes --package @playwright/cli playwright-cli -s=jm-viewport open http://127.0.0.1:5006/
npx --yes --package @playwright/cli playwright-cli -s=jm-viewport run-code --filename scripts/ui-fixtures.js
npx --yes --package @playwright/cli playwright-cli -s=jm-viewport run-code --filename scripts/ui-viewport.js
npx --yes --package @playwright/cli playwright-cli -s=jm-viewport close
```

在预览地址加 `?platform=windows` 可检查桌面 UI，加 `?books=30` 可检查书库翻页。每次更改地址后需要重新运行 fixture 脚本。

桌面回归脚本 `scripts/ui-desktop.js` 在 `?platform=windows` 的 fixture 初始化之后执行，检查 800×600 和 1280×800 下的章节操作与导出。

完成与导出的竞态回归使用 `scripts/ui-completion-race.js`。在 `?platform=windows` 初始化 fixture 后运行该脚本；脚本显式挂起刷新 Promise，检查单章完成后立即导出的参数、刷新期间切换漫画、双章刷新逆序返回，以及刷新失败时仍可导出。桥接数据按 IPC 序列化复制，防止共享对象掩盖状态不同步。
