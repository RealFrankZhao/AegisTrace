#!/bin/sh
set -e

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

echo "=== AEGISTRACE 客户端测试脚本 ==="
echo ""

# 检查 Rust 是否安装
if ! command -v cargo >/dev/null 2>&1; then
    echo "错误: 未找到 cargo，请先安装 Rust"
    exit 1
fi

# 检查 Tauri CLI 是否安装
if ! cargo tauri --version >/dev/null 2>&1; then
    echo "安装 Tauri CLI..."
    cargo install tauri-cli --locked
fi

echo "1. 构建核心组件..."
cargo build --release -p aegis-core-server

# 如果是 macOS，构建原生录屏工具
if [ "$(uname)" = "Darwin" ]; then
    if command -v swift >/dev/null 2>&1; then
        echo "2. 构建 macOS 原生录屏工具..."
        cd "$ROOT_DIR/collectors/macos/native_recorder"
        swift build -c release
        cd "$ROOT_DIR"
    else
        echo "警告: 未找到 Swift，跳过原生录屏工具构建"
    fi
fi

echo ""
echo "3. 启动 Tauri 客户端（开发模式）..."
echo "   提示: 客户端窗口将自动打开"
echo "   使用: 点击 'Start Session' 开始录制，点击 'Stop Session' 停止"
echo ""

cd "$ROOT_DIR/apps/aegis-tauri"
cargo tauri dev
