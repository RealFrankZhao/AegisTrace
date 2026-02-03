# 快速开始 - 客户端测试

## 一键启动（推荐）

```bash
./scripts/test_client.sh
```

## 手动启动

### 1. 构建核心组件

```bash
# 构建核心服务器
cargo build --release -p aegis-core-server

# macOS: 构建原生录屏工具
cd collectors/macos/native_recorder
swift build -c release
cd ../../..
```

### 2. 启动客户端

```bash
cd apps/aegis-tauri
cargo tauri dev
```

## 使用客户端

1. 点击 **"Start Session"** 开始录制
2. 等待几秒钟让系统启动
3. 点击 **"Stop Session"** 停止录制
4. 证据包会保存在 `~/Downloads/Evidence_YYYYMMDD_HHMMSS/`

## 验证证据包

```bash
cargo run -p aegis-verifier -- verify ~/Downloads/Evidence_YYYYMMDD_HHMMSS
```

## 详细文档

查看 `docs/CLIENT_TESTING.md` 获取完整测试指南。
