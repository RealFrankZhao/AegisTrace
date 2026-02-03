# 快速测试指南

## 1. 基本测试（5 分钟）

### 步骤 1: 运行基本检查

```bash
# 在项目根目录
./scripts/test_basic.sh
```

或者手动运行：

```bash
# 检查构建
cd apps/aegis-tauri
cargo check

# 检查验证器
cd ../..
cargo build -p aegis-verifier --release
```

### 步骤 2: 启动应用

```bash
cd apps/aegis-tauri
cargo tauri dev
```

### 步骤 3: 测试基本功能

1. **启动会话**
   - 点击 "Start Session"
   - 等待 1-2 秒
   - 状态应变为 `running`
   - 会话目录应显示

2. **等待录制**
   - 等待 5-10 秒（让录屏开始）

3. **停止会话**
   - 点击 "Stop Session"
   - 等待 3-5 秒
   - 状态应变为 `idle`

### 步骤 4: 验证结果

```bash
# 查找证据包
ls -la ~/Downloads/Evidence_*/

# 验证证据包
cargo run -p aegis-verifier -- verify ~/Downloads/Evidence_YYYYMMDD_HHMMSS

# 检查视频文件
ls -lh ~/Downloads/Evidence_*/files/screen_*.mov
```

**预期结果**：
- ✅ 验证器输出 `PASS`
- ✅ 至少有一个视频文件
- ✅ 文件大小 > 0

## 2. 功能测试清单

### ✅ 基本功能
- [ ] 应用可以启动
- [ ] 可以开始会话
- [ ] 可以停止会话
- [ ] 状态正确更新
- [ ] 证据包已创建

### ✅ 文件验证
- [ ] `session.json` 存在
- [ ] `events.jsonl` 存在
- [ ] `manifest.json` 存在
- [ ] `files/` 目录存在
- [ ] 视频文件存在且大小 > 0

### ✅ 验证器测试
- [ ] 验证器输出 `PASS`
- [ ] 没有错误信息

## 3. 性能测试

### 测试启动速度

```bash
time cargo tauri dev
```

**目标**：< 3 秒

### 测试停止响应

1. 启动会话
2. 点击停止
3. 测量从点击到状态变为 `idle` 的时间

**目标**：< 5 秒

### 测试资源占用

```bash
# macOS
top -pid $(pgrep -f aegis-tauri)

# 或使用 Activity Monitor
```

**目标**：
- CPU（空闲）: < 5%
- 内存: < 200MB

## 4. 常见问题

### 问题：录屏无法启动

**检查**：
1. macOS 屏幕录制权限（系统设置 → 安全性与隐私）
2. 录屏工具是否存在：
   ```bash
   ls -la collectors/macos/native_recorder/.build/release/aegis-native-recorder
   ```
3. 如果不存在，构建它：
   ```bash
   cd collectors/macos/native_recorder
   swift build -c release
   ```

### 问题：没有视频文件

**检查**：
1. 查看终端日志（应该有 "Added video segment" 消息）
2. 检查临时目录：
   ```bash
   ls -la /tmp/aegis_screen_*
   ```
3. 检查证据包目录：
   ```bash
   ls -la ~/Downloads/Evidence_*/files/
   ```

### 问题：验证器失败

**检查**：
1. 运行验证器查看详细错误：
   ```bash
   cargo run -p aegis-verifier -- verify ~/Downloads/Evidence_*
   ```
2. 检查文件完整性：
   ```bash
   ls -la ~/Downloads/Evidence_*/
   ```

## 5. 自动化测试

### 运行基本测试脚本

```bash
./scripts/test_basic.sh
```

### 测试证据包（需要先运行应用）

```bash
./scripts/test_evidence_bundle.sh
```

## 6. 详细测试文档

查看完整测试指南：`docs/TESTING_GUIDE.md`
