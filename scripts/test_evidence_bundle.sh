#!/bin/bash
set -e

echo "=== AEGISTRACE 证据包测试 ==="
echo ""

# 查找最新的证据包
EVIDENCE_DIR=$(ls -td ~/Downloads/Evidence_* 2>/dev/null | head -1)

if [ -z "$EVIDENCE_DIR" ]; then
    echo "❌ 未找到证据包目录"
    echo "请先运行应用并创建一个会话"
    exit 1
fi

echo "测试证据包: $EVIDENCE_DIR"
echo ""

# 测试 1: 检查目录结构
echo "1. 检查目录结构..."
REQUIRED_FILES=("session.json" "events.jsonl" "manifest.json")
for file in "${REQUIRED_FILES[@]}"; do
    if [ ! -f "$EVIDENCE_DIR/$file" ]; then
        echo "❌ 缺少文件: $file"
        exit 1
    fi
    echo "✓ $file 存在"
done

# 测试 2: 检查 files 目录
echo "2. 检查 files 目录..."
if [ ! -d "$EVIDENCE_DIR/files" ]; then
    echo "❌ files 目录不存在"
    exit 1
fi
echo "✓ files 目录存在"

VIDEO_COUNT=$(find "$EVIDENCE_DIR/files" -name "screen_*.mov" | wc -l | tr -d ' ')
echo "  找到 $VIDEO_COUNT 个视频文件"

if [ "$VIDEO_COUNT" -eq 0 ]; then
    echo "⚠ 警告: 没有视频文件"
else
    echo "✓ 视频文件存在"
fi

# 测试 3: 运行验证器
echo "3. 运行验证器..."
cd "$(dirname "$0")/.."
if cargo run -p aegis-verifier -- verify "$EVIDENCE_DIR" 2>&1 | grep -q "PASS"; then
    echo "✓ 验证器测试通过"
else
    echo "❌ 验证器测试失败"
    cargo run -p aegis-verifier -- verify "$EVIDENCE_DIR"
    exit 1
fi

# 测试 4: 检查文件大小
echo "4. 检查文件大小..."
TOTAL_SIZE=$(du -sh "$EVIDENCE_DIR" | cut -f1)
echo "  证据包总大小: $TOTAL_SIZE"

if [ "$VIDEO_COUNT" -gt 0 ]; then
    FIRST_VIDEO=$(find "$EVIDENCE_DIR/files" -name "screen_*.mov" | head -1)
    VIDEO_SIZE=$(du -h "$FIRST_VIDEO" | cut -f1)
    echo "  第一个视频大小: $VIDEO_SIZE"
fi

# 测试 5: 检查事件序列
echo "5. 检查事件序列..."
EVENT_COUNT=$(wc -l < "$EVIDENCE_DIR/events.jsonl" | tr -d ' ')
echo "  事件总数: $EVENT_COUNT"

if [ "$EVENT_COUNT" -lt 2 ]; then
    echo "⚠ 警告: 事件数量过少（至少应该有 session_started 和 session_stopped）"
else
    echo "✓ 事件数量正常"
fi

echo ""
echo "=== 所有证据包测试通过 ==="
