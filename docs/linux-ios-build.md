# Linux 编译 iOS：实验记录与复现

本分支基于 `codex/ios-ui-export-fix` 的 `82a3fc7a420e9a41c13c2da14bc191278f86a285`，不改动应用业务代码、依赖锁文件和原有 macOS 工作流。

## 当前结果

Linux 上可以将本分支的前端、Rust 应用和 Swift 插件编译并链接成 arm64 iOS 可执行文件，再封装为未签名 IPA。此结论仅覆盖构建和静态验证，不代表真机功能、安装或上架已通过。

实际使用的路线：

1. `pnpm build`：TypeScript 检查和 Vite 生产构建。
2. `tauri-xtool` 的 Rust 构建帮助代码：带 `tauri/custom-protocol` 将网页资源嵌入 Rust iOS 静态库。
3. `tauri-xtool` 生成 SwiftPM 工程，纳入 Tauri iOS API、SwiftRs、dialog、opener，以及本仓库的 file-actions 插件。
4. SwiftPM 的 `native` 构建后端编译 Swift/C 源码。
5. Linux LLVM 的 `ld64.lld` 链接 Rust 静态库、全部原生对象文件及 Apple iOS compiler-rt，生成 Mach-O，再由 Python 封装 IPA。

**这不是 `pnpm tauri ios build` 在 Linux 上直接成功，也不是 `xtool dev build` 原样跑通。** 实测 xtool 1.24.0 拒绝复用 xcross 格式的 Darwin SDK，要求重新生成它自己的新版 SDK。这里保留同一份合法取得的 Apple SDK，通过 SwiftPM/LLVM 完成其余构建，没有伪造 xtool 的 SDK 版本标记。

## 验证边界

- `pnpm build` 通过；Rust release 静态库、Swift 原生代码和最终链接通过。
- IPA 做 ZIP CRC 校验；动态依赖路径限定为系统 Frameworks 和 /usr/lib（这不是在旧版真机上验证这些库都可用）；脚本核查 arm64 Mach-O、最低 iOS 15.0、无代码签名，以及 `_main`、`_start_app`、三个插件初始化入口、`FileActionsPlugin`、`StartupRotation` 符号。
- 当前 SDK/Swift 工具链把 Swift 对象的最低版本提高到 iOS 15.0。实验包因此统一声明 iOS 15.0，不沿用源分支配置的 14.0。
- 锁定的 `tao 0.33.0` 没有 `TaoSceneDelegate`。生成配置关闭社区帮助工具默认添加的 UIScene manifest，保留旧版 Tauri 的启动方式。使用新 SDK 在新系统上的启动兼容性仍需真机验证。
- 没有运行 macOS/iPhone 模拟器的启动旋转测试，没有验证 iOS 文件分享、预览、下载或导出操作。没有访问漫画服务、账号或实际漫画内容。
- 当前未编译资产目录，**没有自定义 App 图标 / Assets.car**。不适合作为 App Store 成品。
- 未签名，无 provisioning profile，不能当作已可直接安装的发行包。安装/分发需要另外处理合法签名。
- 源分支 `pnpm exec eslint src` 报告 39 项问题，本实验没有修复这些业务代码问题。TypeScript 和生产构建通过不等于所有检查都通过。
- SwiftRs 1.0.7 的临时指针和旧 iOS API 弃用警告仍存在，未因编译成功而忽略其运行时验证需求。
- GitHub Actions 未验证。提交使用 `[skip ci]`，避免本实验推送自动消耗原有 macOS 构建任务。

## 准备工具

本次环境：Linux x86_64、Rust 1.99.1、Swift 6.4.2、pnpm 9.5.0、LLVM/LLD 19.1.7，以及从用户提供的 Xcode 27.2 提取的 iPhoneOS 27.2 SDK。项目锁定的 Rust Tauri 是 2.5.1，tao 是 0.33.0；保留原锁文件。

准备：

- Python 3.11+、Node.js、pnpm 9.5.0。
- Rust 和 `aarch64-apple-ios` 目标：`rustup target add aarch64-apple-ios`。
- Swift 6.4.2 Linux 工具链，其 `swift`、`swiftc`、`clang`、`clang++`、`llvm-ar` 在 PATH 中。
- 带 Darwin/iOS 支持的 Linux `ld64.lld`、`llvm-libtool-darwin`、`llvm-nm`、`llvm-objdump`。本次使用解包到用户目录的 Debian LLVM 19，不需要 root。不要误用 Swift 包里实测不支持本次 iOS 链接的 lld。
- 自行合法取得的 Apple SDK。不得把 Apple SDK 随本仓库或公开构建镜像重新分发。脚本不包含或自动下载 Xcode。

`--sdk-bundle` 指向一个已有的 xcross/Darwin Swift SDK artifactbundle，至少包含：

```text
info.json
swift-sdk.json
toolset.json
Developer/Platforms/iPhoneOS.platform/Developer/SDKs/iPhoneOS27.2.sdk/
Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/
Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib/clang/21/lib/darwin/libclang_rt.ios.a
```

本实验复用此前准备好的 SDK。SDK 的 Swift interface、宿主工具链和 linker 必须兼容；任意 Xcode 目录不能直接冒充这个 artifactbundle。

## 执行

把目录替换为自己的绝对路径：

```bash
export PATH="/your/swift/usr/bin:/your/llvm19/usr/lib/llvm-19/bin:$PATH"
# 如使用从 .deb 解包的动态链接 LLVM 工具，也设置其依赖目录：
export LD_LIBRARY_PATH="/your/llvm19/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

pnpm --version  # 应为 9.5.0
rustup target add aarch64-apple-ios
python3 scripts/linux-ios/build.py \
  --sdk-bundle /your/xcross-darwin.artifactbundle \
  --ld64 /your/llvm19/usr/lib/llvm-19/bin/ld64.lld \
  --libtool /your/llvm19/usr/lib/llvm-19/bin/llvm-libtool-darwin
```

这里复现的是构建流程，不承诺 IPA 字节完全一致；ZIP 时间戳等因素会改变哈希。校验报告记录工具版本与 SDKSettings 校验值。此脚本只适用于本次固定分支和三个插件，不是通用 Tauri 打包器。

首次执行会冻结安装前端依赖，并从公开上游下载固定提交的帮助模块，按本仓库的 SHA-256 校验后使用：

- 来源：[to-be-continuous/tauri](https://gitlab.com/to-be-continuous/tauri)
- 固定提交：`47b0a5e04f2e701d1a16591e1d4457e25e41d0ff`
- 校验表：`scripts/linux-ios/helper-sha256.json`
- SwiftRs 1.0.7 来源归档也由帮助模块校验固定 SHA-256。

下载失败、校验不匹配或任一编译阶段失败都会停止，不会用空壳输出冒充成功。不要删除错误来强行继续。

已完成冻结安装时可加 `--skip-install`。可通过 `--work-dir` 指定独立工作目录，通过 `--rust-target` 复用 Rust 构建缓存。默认工作目录在已忽略的 `src-tauri/target/linux-ios/`，不会改写原有源码或 macOS 工程。脚本会将上一次 output 改名保留为 previous-output-时间戳，只有本次验证通过才生成新的 IPA；它会重建自己工作目录中的生成工程和 App bundle，不要把工作目录设为存放个人文件的目录。

输出：

```text
src-tauri/target/linux-ios/output/
  jmcomic-downloader_0.17.0_linux_unsigned.ipa
  SHA256SUMS
  verification.json
  macho-load-commands.txt
```

SwiftPM `--build-system native` 已被上游标记为弃用，因此这里固定已验证版本。后续 Swift 升级或迁移到 xtool 新版 SDK，需要重新验证，不能假定永远可用。
