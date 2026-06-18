#!/bin/bash
# ============================================================
# iOS 构建环境初始化脚本 (macOS only)
# ============================================================
# 在 macOS 上运行此脚本以初始化 iOS 编译环境
# 执行: bash scripts/init-ios.sh
# ============================================================

set -euo pipefail

echo "============================================"
echo "  jmcomic-downloader iOS 初始化"
echo "============================================"

# 1. 安装 Rust iOS targets
echo ""
echo "[1/5] 安装 Rust iOS targets..."
rustup target add aarch64-apple-ios
rustup target add aarch64-apple-ios-sim
rustup target add x86_64-apple-ios
echo "  ✓ iOS targets 安装完成"

# 2. 检查 Xcode
echo ""
echo "[2/5] 检查 Xcode..."
if ! xcode-select -p &>/dev/null; then
    echo "  ✗ 错误: 未找到 Xcode，请从 App Store 安装"
    exit 1
fi
echo "  ✓ Xcode 已安装: $(xcode-select -p)"

# 3. 检查 Xcode CLI 工具
echo ""
echo "[3/5] 检查 Xcode CLI 工具..."
if ! xcodebuild -version &>/dev/null; then
    echo "  ✗ 错误: Xcode CLI 工具未安装"
    echo "  运行: xcode-select --install"
    exit 1
fi
echo "  ✓ Xcode CLI 工具可用"

# 4. 安装 npm 依赖 (如果 node_modules 不存在)
echo ""
echo "[4/5] 检查 npm 依赖..."
if [ ! -d "node_modules" ]; then
    echo "  安装 pnpm 依赖..."
    pnpm install
fi
echo "  ✓ npm 依赖就绪"

# 5. 初始化 iOS 项目
echo ""
echo "[5/5] 初始化 iOS Xcode 项目..."
echo ""
echo "  注意: 你需要在 Xcode 中配置以下内容："
echo "  - Development Team (签名)"
echo "  - Bundle Identifier"
echo ""
echo "  或者通过环境变量设置:"
echo "  export APPLE_DEVELOPMENT_TEAM=你的TeamID"
echo ""

# 询问是否立即初始化
read -p "  是否立即初始化 iOS 项目? (y/n): " choice
case "$choice" in
  y|Y )
    echo "  运行: pnpm tauri ios init"
    pnpm tauri ios init
    echo "  ✓ iOS 项目初始化完成"
    ;;
  * )
    echo "  跳过初始化，稍后可运行: pnpm tauri ios init"
    ;;
esac

echo ""
echo "============================================"
echo "  iOS 环境初始化完成！"
echo "============================================"
echo ""
echo "构建命令:"
echo "  pnpm tauri ios build                    # Release (IPA)"
echo "  pnpm tauri ios build --target aarch64-sim --open  # Simulator + Xcode"
echo ""
echo "如需配置开发团队，编辑 tauri.conf.json:"
echo "  bundle.iOS.developmentTeam = \"YOUR_TEAM_ID\""
echo "或设置环境变量:"
echo "  export APPLE_DEVELOPMENT_TEAM=YOUR_TEAM_ID"
