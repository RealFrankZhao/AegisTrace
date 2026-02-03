# AEGISTRACE 客户端测试指南

本指南说明如何生成和测试 AEGISTRACE 客户端（Tauri GUI 应用）。

## 前置要求

### 必需
- **Rust**: 最新稳定版 (https://rustup.rs/)
- **Node.js**: v16 或更高版本（Tauri 需要）

### macOS 特定
- **Xcode Command Line Tools**: `xcode-select --install`
- **Swift**: 用于构建原生录屏工具

## 快速开始

### 方法 1: 使用测试脚本（推荐）

```bash
./scripts/test_client.sh
```

这个脚本会：
1. 检查并安装必要的工具
2. 构建核心组件（aegis-core-server）
3. 构建 macOS 原生录屏工具（如果可用）
4. 启动 Tauri 客户端开发模式

### 方法 2: 手动步骤

#### 步骤 1: 安装 Tauri CLI

```bash
cargo install tauri-cli --locked
```

#### 步骤 2: 构建核心组件

```bash
# 构建核心服务器
cargo build --release -p aegis-core-server

# macOS: 构建原生录屏工具
cd collectors/macos/native_recorder
swift build -c release
cd ../../..
```

#### 步骤 3: 启动客户端

```bash
cd apps/aegis-tauri
cargo tauri dev
```

## 客户端使用

1. **启动会话**:
   - 在 GUI 中点击 "Start Session" 按钮
   - 客户端会自动启动核心服务器和录屏工具
   - 状态会显示为 "running"

2. **停止会话**:
   - 点击 "Stop Session" 按钮
   - 客户端会停止录制并生成证据包

3. **查看状态**:
   - 点击 "Refresh Status" 查看当前状态
   - 状态信息包括：
     - 运行状态（running/idle）
     - 会话目录路径
     - 服务器地址

## 证据包位置

默认情况下，证据包保存在：
- macOS/Linux: `~/Downloads/Evidence_YYYYMMDD_HHMMSS/`
- Windows: `%USERPROFILE%\Downloads\Evidence_YYYYMMDD_HHMMSS\`

## 验证证据包

生成证据包后，可以使用验证器验证：

```bash
cargo run -p aegis-verifier -- verify ~/Downloads/Evidence_YYYYMMDD_HHMMSS
```

## 故障排除

### 问题: "aegis-core-server not found"

**解决方案**: 确保已构建核心服务器
```bash
cargo build --release -p aegis-core-server
```

### 问题: "aegis-native-recorder not found" (macOS)

**解决方案**: 构建原生录屏工具
```bash
cd collectors/macos/native_recorder
swift build -c release
```

### 问题: Tauri 窗口无法打开

**可能原因**:
- 缺少系统依赖（Linux 需要 GTK）
- 权限问题（macOS 需要屏幕录制权限）

**解决方案**:
- Linux: 安装 GTK 库
  ```bash
  sudo apt-get install libgtk-3-dev libwebkit2gtk-4.1-dev
  ```
- macOS: 在系统设置中授予屏幕录制权限

### 问题: 录屏无法启动

**可能原因**:
- macOS: 缺少屏幕录制权限
- 原生录屏工具未构建

**解决方案**:
- macOS: 系统设置 → 安全性与隐私 → 屏幕录制 → 允许终端/Tauri 应用
- 确保已构建原生录屏工具

## 开发模式 vs 生产模式

### 开发模式 (`cargo tauri dev`)
- 自动重新加载
- 显示调试信息
- 适合开发和测试

### 生产模式 (`cargo tauri build`)
- 优化构建
- 生成可分发应用
- 适合最终用户

构建生产版本：
```bash
cd apps/aegis-tauri
cargo tauri build
```

输出位置: `apps/aegis-tauri/src-tauri/target/release/bundle/`

## 测试清单

- [ ] 客户端可以正常启动
- [ ] 可以开始会话
- [ ] 录屏功能正常（macOS）
- [ ] 可以停止会话
- [ ] 证据包正确生成
- [ ] 证据包可以通过验证器验证
- [ ] 状态刷新功能正常

## 更多信息

- 项目 README: `README.md`
- 完整技术指南: `docs/AEGISTRACE_fullstack_guide.txt`
- 项目概览: `docs/PROJECT_OVERVIEW.md`
