# GStreamer RTSP 输入使用指南

## RTSP 拉流输入

使用 GStreamer 输入源可以轻松地从 RTSP 流中读取视频数据。

### 基本用法

```rust
use shanan::{FromUrl, input::GStreamerInput};
use url::Url;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 从 RTSP 流读取
    let url = Url::parse(
        "gst://rtspsrc location=rtsp://192.168.1.100:8554/stream ! \
         decodebin ! videoconvert ! video/x-raw,format=RGB"
    )?;
    let input = GStreamerInput::from_url(&url)?;
    
    // 处理视频帧
    for frame in input.into_nhwc() {
        // 处理每一帧
        println!("收到帧: {}x{}", frame.width(), frame.height());
    }
    
    Ok(())
}
```

### RTSP 流配置示例

#### 1. 基本 RTSP 流

```rust
let url = Url::parse(
    "gst://rtspsrc location=rtsp://example.com/stream ! \
     decodebin ! videoconvert ! video/x-raw,format=RGB"
)?;
```

#### 2. 带认证的 RTSP 流

```rust
let url = Url::parse(
    "gst://rtspsrc location=rtsp://username:password@192.168.1.100:554/stream ! \
     decodebin ! videoconvert ! video/x-raw,format=RGB"
)?;
```

#### 3. 指定分辨率的 RTSP 流

```rust
let url = Url::parse(
    "gst://rtspsrc location=rtsp://camera.local/stream ! \
     decodebin ! videoscale ! video/x-raw,width=1280,height=720 ! \
     videoconvert ! video/x-raw,format=RGB"
)?;
```

#### 4. 低延迟配置

```rust
let url = Url::parse(
    "gst://rtspsrc location=rtsp://192.168.1.100:8554/stream latency=0 ! \
     decodebin ! videoconvert ! video/x-raw,format=RGB"
)?;
```

#### 5. UDP 传输模式

```rust
let url = Url::parse(
    "gst://rtspsrc location=rtsp://192.168.1.100:8554/stream protocols=udp ! \
     decodebin ! videoconvert ! video/x-raw,format=RGB"
)?;
```

### 与模型集成示例

```rust
use shanan::{
    FromUrl,
    input::GStreamerInput,
    model::{CocoLabel, DetectResult, Model},
    output::Render,
};
use url::Url;
use anyhow::Result;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    // RTSP 输入
    let input_url = Url::parse(
        "gst://rtspsrc location=rtsp://camera.local/stream ! \
         decodebin ! videoconvert ! video/x-raw,format=RGB"
    )?;
    let input = GStreamerInput::from_url(&input_url)?;
    
    // 加载模型
    let model_url = Url::parse("file:///path/to/model.rknn")?;
    let model = shanan::model::Yolo26Builder::from_url(&model_url)?.build()?;
    
    // 创建输出
    let output_url = Url::parse("image:///path/to/output.jpg")?;
    let output = shanan::output::SaveImageFileOutput::from_url(&output_url)?;
    
    // 处理视频流
    for frame in input.into_nhwc() {
        let result: DetectResult<CocoLabel> = model.infer(&frame)?;
        output.render_result(&frame, &result)?;
        
        // 可以在这里添加中断逻辑
        // if should_stop { break; }
    }
    
    Ok(())
}
```

### 常见 RTSP 源

#### IP 摄像头
```rust
// 海康威视
"gst://rtspsrc location=rtsp://admin:password@192.168.1.64:554/Streaming/Channels/1 ! ..."

// 大华摄像头
"gst://rtspsrc location=rtsp://admin:password@192.168.1.108:554/cam/realmonitor?channel=1&subtype=0 ! ..."

// 通用 ONVIF 摄像头
"gst://rtspsrc location=rtsp://192.168.1.100:554/stream1 ! ..."
```

#### 媒体服务器
```rust
// MediaMTX (formerly rtsp-simple-server)
"gst://rtspsrc location=rtsp://localhost:8554/mystream ! ..."

// VLC 流
"gst://rtspsrc location=rtsp://localhost:8554/ ! ..."
```

### 性能优化

#### 1. 减少延迟
```rust
let url = Url::parse(
    "gst://rtspsrc location=rtsp://camera.local/stream latency=0 buffer-mode=0 ! \
     queue max-size-buffers=1 leaky=downstream ! \
     decodebin ! videoconvert ! video/x-raw,format=RGB"
)?;
```

#### 2. 使用硬件解码
```rust
// 树莓派/Rockchip 平台
let url = Url::parse(
    "gst://rtspsrc location=rtsp://camera.local/stream ! \
     rtph264depay ! h264parse ! v4l2h264dec ! \
     videoconvert ! video/x-raw,format=RGB"
)?;
```

### 错误处理

```rust
use shanan::input::GStreamerInputError;

match GStreamerInput::from_url(&url) {
    Ok(input) => {
        // 成功创建输入
        for frame in input.into_nhwc() {
            // 处理帧
        }
    }
    Err(GStreamerInputError::SchemeMismatch) => {
        eprintln!("错误: URL scheme 不正确");
    }
    Err(GStreamerInputError::PipelineError(msg)) => {
        eprintln!("管道错误: {}", msg);
    }
    Err(e) => {
        eprintln!("其他错误: {}", e);
    }
}
```

### 故障排除

#### 连接超时
- 检查网络连接
- 验证 RTSP URL 是否正确
- 确认摄像头或服务器正在运行

#### 解码错误
- 确保安装了必要的 GStreamer 插件（gst-plugins-good, gst-plugins-bad, gst-plugins-ugly）
- 尝试不同的解码器

#### 延迟过高
- 使用 `latency=0` 参数
- 启用硬件解码
- 使用 UDP 而不是 TCP 传输
- 减少缓冲区大小

### 测试 RTSP 流

在使用前，可以用 `gst-launch-1.0` 命令测试 RTSP 流：

```bash
# 测试播放
gst-launch-1.0 rtspsrc location=rtsp://camera.local/stream ! \
  decodebin ! autovideosink

# 测试到文件
gst-launch-1.0 rtspsrc location=rtsp://camera.local/stream ! \
  decodebin ! videoconvert ! x264enc ! mp4mux ! filesink location=test.mp4
```
