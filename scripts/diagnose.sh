#!/bin/bash

echo "=== AEGISTRACE 诊断工具 ==="
echo ""

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

# 检查 1: 录屏工具
echo "1. 检查录屏工具..."
RECORDER_PATH="collectors/macos/native_recorder/.build/release/aegis-native-recorder"
if [ -f "$RECORDER_PATH" ]; then
    echo "✓ 录屏工具存在: $RECORDER_PATH"
    ls -lh "$RECORDER_PATH"
    
    # 检查是否可执行
    if [ -x "$RECORDER_PATH" ]; then
        echo "✓ 录屏工具可执行"
    else
        echo "❌ 录屏工具不可执行，尝试修复..."
        chmod +x "$RECORDER_PATH"
    fi
else
    echo "❌ 录屏工具不存在"
    echo "   构建命令: cd collectors/macos/native_recorder && swift build -c release"
fi

# 检查 2: 配置文件
echo ""
echo "2. 检查配置文件..."
if [ -f "config/config.json" ]; then
    echo "✓ 配置文件存在"
    cat config/config.json | head -5
else
    echo "❌ 配置文件不存在"
fi

# 检查 3: 权限检查（macOS）
echo ""
echo "3. 检查 macOS 权限..."
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "   请检查系统设置 → 安全性与隐私 → 屏幕录制"
    echo "   确保以下应用有权限："
    echo "   - Terminal (如果从终端运行)"
    echo "   - aegis-tauri (如果作为应用运行)"
fi

# 检查 4: 环境变量
echo ""
echo "4. 检查环境变量..."
if [ -n "$AEGIS_NATIVE_RECORDER" ]; then
    echo "   AEGIS_NATIVE_RECORDER: $AEGIS_NATIVE_RECORDER"
    if [ -f "$AEGIS_NATIVE_RECORDER" ]; then
        echo "   ✓ 路径有效"
    else
        echo "   ❌ 路径无效"
    fi
else
    echo "   AEGIS_NATIVE_RECORDER: 未设置（将使用默认路径）"
fi

# 检查 5: 临时目录
echo ""
echo "5. 检查临时目录..."
TMP_DIR="/tmp"
if [ -d "$TMP_DIR" ] && [ -w "$TMP_DIR" ]; then
    echo "✓ 临时目录可写: $TMP_DIR"
else
    echo "❌ 临时目录不可写: $TMP_DIR"
fi

# 检查 6: 证据包目录
echo ""
echo "6. 检查证据包目录..."
EVIDENCE_DIR="$HOME/Downloads"
if [ -d "$EVIDENCE_DIR" ] && [ -w "$EVIDENCE_DIR" ]; then
    echo "✓ 证据包目录可写: $EVIDENCE_DIR"
    EVIDENCE_COUNT=$(ls -d "$EVIDENCE_DIR"/Evidence_* 2>/dev/null | wc -l | tr -d ' ')
    echo "   现有证据包数量: $EVIDENCE_COUNT"
else
    echo "❌ 证据包目录不可写: $EVIDENCE_DIR"
fi

# 检查 7: 测试录屏工具
echo ""
echo "7. 测试录屏工具..."
if [ -f "$RECORDER_PATH" ] && [ -x "$RECORDER_PATH" ]; then
    echo "   尝试运行录屏工具（3秒测试）..."
    timeout 3 "$RECORDER_PATH" /tmp/test_recording.mov 3 2>&1 || echo "   录屏工具测试完成（可能因权限失败，这是正常的）"
    if [ -f "/tmp/test_recording.mov" ]; then
        echo "   ✓ 录屏工具可以创建文件"
        rm -f /tmp/test_recording.mov
    else
        echo "   ⚠ 录屏工具无法创建文件（可能是权限问题）"
    fi
fi

echo ""
echo "=== 诊断完成 ==="
echo ""
echo "如果录屏工具不存在，运行："
echo "  cd collectors/macos/native_recorder && swift build -c release"
echo ""
echo "如果权限有问题，检查："
echo "  系统设置 → 安全性与隐私 → 屏幕录制"
