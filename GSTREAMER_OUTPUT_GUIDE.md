# GStreamer 视频输出使用指南

## 功能概述

GStreamer 输出模块提供两种输出方式：
1. **视频文件输出** - 将处理后的视频保存为文件
2. **RTSP 推流输出** - 通过 RTSP 实时推流

## 视频文件输出 (GStreamerVideoOutput)

### URL Scheme
`gstvideo://`

### 基本用法

```rust
use shanan::{FromUrl, output::GStreamerVideoOutput};
use url::Url;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建视频文件输出
    let url = Url::parse("gstvideo:///path/to/output.mp4?width=1280&height=720&fps=30")?;
    let output = GStreamerVideoOutput::from_url(&url)?;
    
    // 使用 output 与 Render trait
    // output.render_result(&frame, &result)?;
    
    Ok(())
}
```

### 支持的格式

#### 1. MP4 (H.264)
```rust
let url = Url::parse("gstvideo:///output/video.mp4?width=1920&height=1080&fps=30")?;
```

#### 2. MKV (Matroska)
```rust
let url = Url::parse("gstvideo:///output/video.mkv?width=1920&height=1080&fps=30")?;
```

#### 3. AVI
```rust
let url = Url::parse("gstvideo:///output/video.avi?width=1280&height=720&fps=25")?;
```

#### 4. WebM (VP8)
```rust
let url = Url::parse("gstvideo:///output/video.webm?width=1280&height=720&fps=30")?;
```

### 参数说明

| 参数 | 默认值 | 说明 |
|------|-------|------|
| width | 640 | 视频宽度（像素）|
| height | 480 | 视频高度（像素）|
| fps | 30 | 帧率（帧/秒）|

### 完整示例

```rust
use shanan::{
    FromUrl,
    input::GStreamerInput,
    output::GStreamerVideoOutput,
    model::{CocoLabel, DetectResult, Model},
};
use url::Url;
use anyhow::Result;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    // 输入：RTSP 流
    let input_url = Url::parse(
        "gst://rtspsrc location=rtsp://camera.local/stream ! \
         decodebin ! videoconvert ! video/x-raw,format=RGB,width=1280,height=720"
    )?;
    let input = GStreamerInput::from_url(&input_url)?;
    
    // 输出：视频文件
    let output_url = Url::parse(
        "gstvideo:///output/processed.mp4?width=1280&height=720&fps=30"
    )?;
    let output = GStreamerVideoOutput::from_url(&output_url)?;
    
    // 加载模型
    let model_url = Url::parse("file:///path/to/model.rknn")?;
    let model = shanan::model::Yolo26Builder::from_url(&model_url)?.build()?;
    
    // 处理并保存视频
    for frame in input.into_nhwc() {
        let result: DetectResult<CocoLabel> = model.infer(&frame)?;
        output.render_result(&frame, &result)?;
    }
    
    Ok(())
}
```

## RTSP 推流输出 (GStreamerRtspOutput)

### URL Scheme
`gstrtsp://`

### 基本用法

```rust
use shanan::{FromUrl, output::GStreamerRtspOutput};
use url::Url;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 RTSP 推流输出
    let url = Url::parse(
        "gstrtsp://0.0.0.0/live?width=1280&height=720&fps=30&port=8554"
    )?;
    let output = GStreamerRtspOutput::from_url(&url)?;
    
    // 使用 output 与 Render trait
    // output.render_result(&frame, &result)?;
    
    Ok(())
}
```

### 参数说明

| 参数 | 默认值 | 说明 |
|------|-------|------|
| width | 640 | 视频宽度（像素）|
| height | 480 | 视频高度（像素）|
| fps | 30 | 帧率（帧/秒）|
| port | 8554 | UDP 端口 |

### 客户端连接

推流启动后，可以使用以下方式连接：

#### VLC 播放器
```bash
vlc rtsp://服务器IP:8554/live
```

#### FFplay
```bash
ffplay -rtsp_transport udp rtsp://服务器IP:8554/live
```

#### GStreamer 客户端
```bash
gst-launch-1.0 rtspsrc location=rtsp://服务器IP:8554/live ! \
  decodebin ! autovideosink
```

### 完整推流示例

```rust
use shanan::{
    FromUrl,
    input::GStreamerInput,
    output::GStreamerRtspOutput,
    model::{CocoLabel, DetectResult, Model},
};
use url::Url;
use anyhow::Result;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    // 输入：本地摄像头
    let input_url = Url::parse(
        "gst://v4l2src device=/dev/video0 ! \
         videoconvert ! video/x-raw,format=RGB,width=1280,height=720"
    )?;
    let input = GStreamerInput::from_url(&input_url)?;
    
    // 输出：RTSP 推流
    let output_url = Url::parse(
        "gstrtsp://0.0.0.0/camera?width=1280&height=720&fps=30&port=8554"
    )?;
    let output = GStreamerRtspOutput::from_url(&output_url)?;
    
    // 加载模型
    let model_url = Url::parse("file:///path/to/model.rknn")?;
    let model = shanan::model::Yolo26Builder::from_url(&model_url)?.build()?;
    
    println!("RTSP 流已启动，可通过以下地址访问：");
    println!("rtsp://localhost:8554/camera");
    
    // 实时处理并推流
    for frame in input.into_nhwc() {
        let result: DetectResult<CocoLabel> = model.infer(&frame)?;
        output.render_result(&frame, &result)?;
    }
    
    Ok(())
}
```

## 实际应用场景

### 场景 1: 视频文件处理
读取视频文件，进行目标检测，输出带标注的视频

```rust
// 输入视频
let input_url = Url::parse(
    "gst://filesrc location=input.mp4 ! decodebin ! \
     videoconvert ! video/x-raw,format=RGB"
)?;

// 输出视频
let output_url = Url::parse("gstvideo:///output.mp4?width=1920&height=1080&fps=30")?;
```

### 场景 2: RTSP 转推流
从一个 RTSP 源读取，处理后推送到另一个 RTSP 流

```rust
// 输入: IP 摄像头
let input_url = Url::parse(
    "gst://rtspsrc location=rtsp://camera.local/stream ! \
     decodebin ! videoconvert ! video/x-raw,format=RGB"
)?;

// 输出: RTSP 推流
let output_url = Url::parse("gstrtsp://0.0.0.0/processed?port=8554")?;
```

### 场景 3: 本地摄像头实时推流
从本地摄像头读取，实时推流

```rust
// 输入: USB 摄像头
let input_url = Url::parse(
    "gst://v4l2src device=/dev/video0 ! \
     videoconvert ! video/x-raw,format=RGB"
)?;

// 输出: RTSP 推流
let output_url = Url::parse("gstrtsp://0.0.0.0/webcam?port=8554")?;
```

## 性能优化

### 硬件编码
使用硬件编码器可以显著提高性能（需要修改源码中的管道配置）：

```rust
// 使用 VAAPI 硬件编码 (Intel)
"appsrc ! videoconvert ! vaapih264enc ! h264parse ! mp4mux ! filesink"

// 使用 V4L2 硬件编码 (Raspberry Pi/Rockchip)
"appsrc ! videoconvert ! v4l2h264enc ! h264parse ! mp4mux ! filesink"
```

### 调整比特率
在推流时可以调整比特率以平衡质量和带宽：
- 低比特率 (500-1000 kbps): 适合移动网络
- 中比特率 (2000-4000 kbps): 适合局域网
- 高比特率 (8000+ kbps): 适合高质量录制

## 注意事项

1. **资源清理**: 输出对象被 drop 时会自动关闭文件或停止推流
2. **帧率匹配**: 确保输入和输出的分辨率、帧率匹配
3. **缓冲**: 推流时使用 `is-live=true` 减少延迟
4. **网络**: RTSP 推流需要确保端口未被占用

## 故障排除

### 视频文件无法播放
- 确保所有帧都已写入（等待 drop 或显式调用 EOS）
- 检查文件权限和磁盘空间

### RTSP 客户端无法连接
- 确认端口未被防火墙阻止
- 检查服务器地址和端口是否正确
- 尝试使用 UDP 传输模式

### 性能问题
- 考虑使用硬件编码
- 降低分辨率或帧率
- 调整编码预设（speed-preset）
