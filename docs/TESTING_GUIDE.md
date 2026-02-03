# AEGISTRACE 测试指南

本指南说明如何测试优化后的 AEGISTRACE 客户端应用。

## 快速测试

### 1. 构建应用

```bash
cd apps/aegis-tauri
cargo tauri dev
```

### 2. 基本功能测试

#### 测试 1: 启动和停止会话

1. **启动应用**：运行 `cargo tauri dev`，GUI 窗口应该打开
2. **检查状态**：点击 "Refresh Status"，应该显示 `idle`
3. **开始录制**：
   - 点击 "Start Session"
   - 等待 1-2 秒
   - 状态应该变为 `running`
   - 会话目录应该显示在界面上
4. **停止录制**：
   - 点击 "Stop Session"
   - 等待 3-5 秒（文件处理）
   - 状态应该变为 `idle`

#### 测试 2: 验证证据包

```bash
# 找到生成的证据包（通常在 ~/Downloads/）
ls -la ~/Downloads/Evidence_*/

# 验证证据包
cargo run -p aegis-verifier -- verify ~/Downloads/Evidence_YYYYMMDD_HHMMSS
```

预期结果：`PASS`

#### 测试 3: 检查视频文件

```bash
# 检查文件结构
ls -lh ~/Downloads/Evidence_*/files/

# 应该看到：
# - screen_1_*.mov (录制的视频文件)
# - events.jsonl
# - session.json
# - manifest.json
```

## 详细测试场景

### 场景 1: 短时间录制（5-10 秒）

**目的**：测试快速启动和停止

1. 启动会话
2. 等待 5-10 秒
3. 停止会话
4. 验证：
   - 证据包已创建
   - 至少有一个视频文件
   - 文件大小 > 0

### 场景 2: 长时间录制（1-2 分钟）

**目的**：测试分段功能

1. 启动会话
2. 等待 1-2 分钟
3. 停止会话
4. 验证：
   - 多个视频段（如果超过 10 分钟）
   - 所有段都已保存
   - 事件序列正确

### 场景 3: 多次启动停止

**目的**：测试状态管理

1. 启动会话 → 停止
2. 立即再次启动 → 停止
3. 重复 3-5 次
4. 验证：
   - 每次都能正常启动/停止
   - 没有资源泄漏
   - 每个会话都有独立的证据包

### 场景 4: 异常停止

**目的**：测试错误恢复

1. 启动会话
2. 快速停止（1-2 秒内）
3. 验证：
   - 应用不崩溃
   - 证据包仍然有效
   - 部分录制的文件也能保存

## 性能测试

### 测试 1: 启动速度

```bash
# 记录启动时间
time cargo tauri dev
```

**预期**：启动时间 < 3 秒

### 测试 2: 段切换速度

1. 启动录制
2. 等待第一个段完成（10 分钟）
3. 观察日志，检查段切换是否流畅

**预期**：段切换无延迟，下一段立即开始

### 测试 3: 停止响应时间

1. 启动录制
2. 点击停止
3. 测量从点击到状态变为 `idle` 的时间

**预期**：< 5 秒（包括文件处理）

### 测试 4: 资源占用

```bash
# 监控资源使用
top -pid $(pgrep -f aegis-tauri)
```

**预期**：
- CPU: < 10%（空闲时）
- 内存: < 200MB

## 自动化测试脚本

### 基本测试脚本

创建 `scripts/test_basic.sh`：

```bash
#!/bin/bash
set -e

echo "=== AEGISTRACE 基本功能测试 ==="

# 测试 1: 构建
echo "1. 测试构建..."
cd apps/aegis-tauri
cargo check || exit 1

# 测试 2: 验证器
echo "2. 测试验证器..."
cd ../..
cargo build -p aegis-verifier --release || exit 1

# 测试 3: 检查配置文件
echo "3. 检查配置文件..."
if [ ! -f config/config.json ]; then
    echo "错误: config/config.json 不存在"
    exit 1
fi

echo "✓ 所有基本测试通过"
```

### 功能测试脚本

创建 `scripts/test_functional.sh`：

```bash
#!/bin/bash
set -e

echo "=== AEGISTRACE 功能测试 ==="

# 需要手动运行 GUI 进行以下测试：
echo "请手动测试以下场景："
echo "1. 启动会话 → 等待 5 秒 → 停止"
echo "2. 检查 ~/Downloads/Evidence_* 目录"
echo "3. 运行验证器: cargo run -p aegis-verifier -- verify ~/Downloads/Evidence_*"
```

## 日志检查

### 查看应用日志

运行 `cargo tauri dev` 时，终端会显示日志：

```
Added video segment 1 (size: 1234567 bytes)
Added video segment 2 (size: 2345678 bytes)
```

### 检查错误日志

如果有问题，查看：
- 终端输出（stderr）
- 系统日志（macOS: Console.app）

## 常见问题排查

### 问题 1: 录屏无法启动

**检查**：
```bash
# 检查录屏工具是否存在
ls -la collectors/macos/native_recorder/.build/release/aegis-native-recorder

# 检查权限
# macOS: 系统设置 → 安全性与隐私 → 屏幕录制
```

### 问题 2: 没有视频文件

**检查**：
1. 查看终端日志，是否有错误信息
2. 检查临时目录：`ls -la /tmp/aegis_screen_*`
3. 检查证据包目录：`ls -la ~/Downloads/Evidence_*/files/`

### 问题 3: 应用卡住

**检查**：
1. 查看是否有进程挂起：`ps aux | grep aegis`
2. 检查锁竞争（查看日志中的警告）
3. 强制退出并重启

## 性能基准

### 预期性能指标

| 指标 | 目标值 | 测试方法 |
|------|--------|----------|
| 启动时间 | < 3 秒 | 测量从启动到 GUI 显示 |
| 开始录制延迟 | < 2 秒 | 从点击到录屏开始 |
| 停止响应 | < 5 秒 | 从点击到状态更新 |
| 段切换延迟 | < 100ms | 观察日志时间戳 |
| CPU 占用（空闲） | < 5% | top/Activity Monitor |
| 内存占用 | < 200MB | top/Activity Monitor |

## 回归测试清单

每次更新后，运行以下测试：

- [ ] 基本启动/停止功能
- [ ] 证据包生成和验证
- [ ] 视频文件保存
- [ ] 多次启动/停止
- [ ] 异常情况处理
- [ ] 性能指标检查

## 下一步

- 添加单元测试
- 添加集成测试
- 添加性能基准测试
- 添加自动化 CI 测试
