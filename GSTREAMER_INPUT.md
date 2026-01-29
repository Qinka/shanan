# GStreamer 输入源使用说明

## 功能介绍

GStreamer 输入源允许使用 GStreamer 管道读取视频流数据，适用于各种项目需求，例如：
- 从文件读取视频
- 从摄像头捕获视频
- 从网络流读取视频
- 使用 GStreamer 的各种滤镜和转换

## 编译要求

### 系统依赖

在使用 GStreamer 输入源之前，需要安装以下系统库：

**Ubuntu/Debian:**
```bash
sudo apt-get install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev
```

**Fedora/RHEL:**
```bash
sudo dnf install gstreamer1-devel gstreamer1-plugins-base-devel
```

**macOS:**
```bash
brew install gstreamer
```

### Cargo 特性

在 `Cargo.toml` 中启用 `gstreamer_input` 特性：

```toml
[dependencies]
shanan = { version = "0.1", features = ["gstreamer_input"] }
```

或在构建时指定：
```bash
cargo build --features gstreamer_input
```

## 使用示例

### 基本用法

使用 URL scheme `gst://` 来指定 GStreamer 管道：

```rust
use shanan::{FromUrl, input::GStreamerInput};
use url::Url;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 从视频文件读取
    let url = Url::parse("gst://filesrc location=video.mp4 ! decodebin ! videoconvert ! video/x-raw,format=RGB")?;
    let input = GStreamerInput::from_url(&url)?;
    
    // 转换为 NHWC 格式进行迭代
    for frame in input.into_nhwc() {
        // 处理每一帧
        println!("处理帧: {}x{}", frame.width(), frame.height());
    }
    
    Ok(())
}
```

### 从摄像头读取

```rust
let url = Url::parse("gst://v4l2src device=/dev/video0 ! videoconvert ! video/x-raw,format=RGB")?;
let input = GStreamerInput::from_url(&url)?;
```

### 从网络流读取

```rust
let url = Url::parse("gst://rtspsrc location=rtsp://example.com/stream ! decodebin ! videoconvert ! video/x-raw,format=RGB")?;
let input = GStreamerInput::from_url(&url)?;
```

### 使用测试源

```rust
let url = Url::parse("gst://videotestsrc ! videoconvert ! video/x-raw,format=RGB")?;
let input = GStreamerInput::from_url(&url)?;
```

## 支持的视频格式

目前支持以下视频格式：
- RGB
- BGR

其他格式需要使用 `videoconvert` 插件转换为 RGB 或 BGR 格式。

## 完整示例

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
    // 初始化日志
    tracing_subscriber::fmt::init();
    
    // 创建 GStreamer 输入源
    let input_url = Url::parse("gst://filesrc location=input.mp4 ! decodebin ! videoconvert ! video/x-raw,format=RGB")?;
    let input = GStreamerInput::from_url(&input_url)?;
    
    // 加载模型
    let model_url = Url::parse("file:///path/to/model.rknn")?;
    let model = shanan::model::Yolo26Builder::from_url(&model_url)?.build()?;
    
    // 创建输出
    let output_url = Url::parse("image:///path/to/output.jpg")?;
    let output = shanan::output::SaveImageFileOutput::from_url(&output_url)?;
    
    // 处理视频帧
    for frame in input.into_nhwc() {
        let result: DetectResult<CocoLabel> = model.infer(&frame)?;
        output.render_result(&frame, &result)?;
    }
    
    Ok(())
}
```

## GStreamer 管道语法

URL 路径部分应该是一个有效的 GStreamer 管道描述，不包括最后的 `appsink`（会自动添加）。

### 管道示例

**从文件读取:**
```
filesrc location=video.mp4 ! decodebin ! videoconvert ! video/x-raw,format=RGB
```

**从摄像头读取:**
```
v4l2src device=/dev/video0 ! videoconvert ! video/x-raw,format=RGB,width=640,height=480
```

**测试视频源:**
```
videotestsrc pattern=smpte ! videoconvert ! video/x-raw,format=RGB,width=1280,height=720
```

**RTSP 网络流:**
```
rtspsrc location=rtsp://192.168.1.100:8554/stream ! decodebin ! videoconvert ! video/x-raw,format=RGB
```

## 注意事项

1. **管道格式**: 确保管道输出为 `video/x-raw,format=RGB` 或 `video/x-raw,format=BGR`
2. **性能**: GStreamer 提供硬件加速支持，可以使用相应的插件提高性能
3. **错误处理**: 管道错误会通过 `GStreamerInputError` 返回
4. **资源清理**: `GStreamerInput` 在 drop 时会自动停止管道
5. **安全性**: GStreamer 管道描述直接传递给 GStreamer 解析器。在生产环境中使用不可信的输入时，应验证或限制管道描述以防止资源滥用

## 故障排除

### 管道启动失败
- 检查 GStreamer 插件是否安装完整
- 使用 `gst-launch-1.0` 命令行工具测试管道是否正常工作

### 视频格式不支持
- 确保管道最后有 `videoconvert ! video/x-raw,format=RGB`
- 检查视频源格式是否被 GStreamer 支持

### 性能问题
- 考虑使用硬件解码器（如 `vaapidecodebin`）
- 调整管道中的缓冲区大小
- 使用适当的视频分辨率
