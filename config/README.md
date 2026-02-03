# AEGISTRACE 配置文件说明

## 配置文件位置

配置文件位于 `config/config.json`，程序启动时会自动查找并加载。

## 配置项说明

### server（服务器配置）

- `default_addr`: 默认服务器地址（默认：`127.0.0.1:7878`）
- `stop_wait_ms`: 停止服务器前的等待时间（毫秒，默认：300）
- `stop_retry_count`: 停止录屏时的重试次数（默认：10）
- `stop_retry_interval_ms`: 停止录屏时的重试间隔（毫秒，默认：200）

### recording（录制配置）

- `segment_duration_seconds`: 录屏分段时长（秒，默认：600，即 10 分钟）
- `poll_interval_ms`: 轮询间隔（毫秒，默认：200）
- `video`: 视频编码配置
  - `codec`: 编码格式（默认：`hevc`，即 H.265）
  - `resolution`: 分辨率
    - `width`: 宽度（默认：1280）
    - `height`: 高度（默认：720）
  - `fps`: 帧率（默认：30）
  - `bitrate_bps`: 码率（比特每秒，默认：2000000，即 2Mbps）

### paths（路径配置）

- `default_save_dir`: 默认保存目录（支持 `~/` 路径，默认：`~/Downloads`）
- `temp_dir`: 临时文件目录（`null` 表示使用系统临时目录）

### app（应用配置）

- `platform`: 平台标识（默认：`macos`）
- `version`: 应用版本（默认：`0.1.0`）

## 使用示例

### 修改录屏分段时长为 5 分钟

```json
{
  "recording": {
    "segment_duration_seconds": 300,
    ...
  }
}
```

### 修改默认保存目录

```json
{
  "paths": {
    "default_save_dir": "~/Documents/AEGISTRACE",
    ...
  }
}
```

### 修改服务器地址

```json
{
  "server": {
    "default_addr": "127.0.0.1:8888",
    ...
  }
}
```

### 修改视频质量（更高码率）

```json
{
  "recording": {
    "video": {
      "bitrate_bps": 5000000,
      ...
    }
  }
}
```

## 注意事项

1. 修改配置文件后需要重启应用程序才能生效
2. 路径支持 `~/` 表示用户主目录
3. 视频配置项（`codec`、`resolution`、`fps`、`bitrate_bps`）目前仅用于配置，实际录屏参数由 Swift 录屏器控制（未来版本将支持从配置文件读取）
