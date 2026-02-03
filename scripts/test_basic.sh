#!/bin/bash
set -e

echo "=== AEGISTRACE 基本功能测试 ==="
echo ""

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

# 测试 1: 构建检查
echo "1. 测试构建..."
cd apps/aegis-tauri
if cargo check 2>&1 | grep -q "error"; then
    echo "❌ 构建失败"
    exit 1
fi
echo "✓ 构建检查通过"
cd "$ROOT_DIR"

# 测试 2: 验证器构建
echo "2. 测试验证器..."
if ! cargo build -p aegis-verifier --release 2>&1 | grep -q "Finished"; then
    echo "❌ 验证器构建失败"
    exit 1
fi
echo "✓ 验证器构建成功"
cd "$ROOT_DIR"

# 测试 3: 检查配置文件
echo "3. 检查配置文件..."
if [ ! -f config/config.json ]; then
    echo "❌ config/config.json 不存在"
    exit 1
fi
echo "✓ 配置文件存在"

# 测试 4: 检查录屏工具（如果已构建）
echo "4. 检查录屏工具..."
if [ -f collectors/macos/native_recorder/.build/release/aegis-native-recorder ]; then
    echo "✓ 录屏工具已构建"
else
    echo "⚠ 录屏工具未构建（可选，运行时会自动构建）"
fi

# 测试 5: 检查核心库
echo "5. 检查核心库..."
if ! cargo check -p aegis-core 2>&1 | grep -q "Finished"; then
    echo "❌ 核心库检查失败"
    exit 1
fi
echo "✓ 核心库检查通过"

echo ""
echo "=== 所有基本测试通过 ==="
echo ""
echo "下一步："
echo "1. 运行 'cd apps/aegis-tauri && cargo tauri dev' 启动 GUI"
echo "2. 在 GUI 中测试启动/停止功能"
echo "3. 验证生成的证据包：cargo run -p aegis-verifier -- verify ~/Downloads/Evidence_*"
