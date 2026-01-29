# Shanan 山南西风

高性能视频流处理库，专为嵌入式 AI 推理设计，特别优化 Rockchip NPU。

## 功能特性

- 🎥 **多源视频输入**
  - 图像文件（JPEG, PNG）
  - GStreamer 管道（视频文件、摄像头、RTSP 流）
  - V4L2 摄像头直接支持

- 🤖 **AI 推理支持**
  - Rockchip NPU 硬件加速（RKNN）
  - YOLOv2.6 目标检测

- 📤 **灵活的输出方式**
  - 图像文件保存（带检测框标注）
  - 视频文件输出（MP4, MKV, AVI, WebM）
  - RTSP 实时推流

- ⚡ **高性能设计**
  - 零拷贝帧传递
  - NCHW/NHWC 格式支持
  - 硬件加速编解码

## 系统要求

### 必需依赖

```bash
# Ubuntu/Debian
sudo apt-get install build-essential pkg-config

# Rockchip NPU 支持（RK3588/RK3566 等平台）
# 需要安装 librknpu 开发库
```

### 可选依赖

**GStreamer 支持** (视频流输入/输出):
```bash
# Ubuntu/Debian
sudo apt-get install \
    libgstreamer1.0-dev \
    libgstreamer-plugins-base1.0-dev \
    gstreamer1.0-plugins-good \
    gstreamer1.0-plugins-bad \
    gstreamer1.0-plugins-ugly

# Fedora/RHEL
sudo dnf install \
    gstreamer1-devel \
    gstreamer1-plugins-base-devel

# macOS
brew install gstreamer
```

## 安装

### 添加依赖

在 `Cargo.toml` 中添加：

```toml
[dependencies]
shanan = { git = "https://github.com/Qinka/shanan.git", branch = "main" }
```

### 功能特性标志

```toml
[dependencies]
shanan = {
    git = "https://github.com/Qinka/shanan.git",
    branch = "main",
    features = [
        "read_image_file",      # 图像文件输入
        "save_image_file",      # 图像文件输出（带标注）
        "gstreamer_input",      # GStreamer 视频输入
        "gstreamer_output",     # GStreamer 视频/RTSP 输出
    ]
}
```

**默认特性**: `read_image_file`, `save_image_file`, `gstreamer_input`

## 快速开始

### 图像文件推理

```rust
use shanan::{
    FromUrl,
    input::ImageFileInput,
    output::SaveImageFileOutput,
    model::{CocoLabel, DetectResult, Model},
};
use url::Url;
use anyhow::Result;

fn main() -> Result<()> {
    // 加载图像
    let input_url = Url::parse("image:///path/to/input.jpg")?;
    let input = ImageFileInput::from_url(&input_url)?;

    // 加载模型
    let model_url = Url::parse("file:///path/to/model.rknn")?;
    let model = shanan::model::Yolo26Builder::from_url(&model_url)?.build()?;

    // 创建输出
    let output_url = Url::parse("image:///path/to/output.jpg")?;
    let output = SaveImageFileOutput::from_url(&output_url)?;

    // 推理并保存
    for frame in input.into_nhwc() {
        let result: DetectResult<CocoLabel> = model.infer(&frame)?;
        output.render_result(&frame, &result)?;
    }

    Ok(())
}
```

### RTSP 流处理

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
    // RTSP 流输入
    let input_url = Url::parse(
        "gst://rtspsrc location=rtsp://192.168.1.100:8554/stream ! \
         decodebin ! videoconvert ! video/x-raw,format=RGB"
    )?;
    let input = GStreamerInput::from_url(&input_url)?;

    // 视频文件输出
    let output_url = Url::parse(
        "gstvideo:///output/processed.mp4?width=1280&height=720&fps=30"
    )?;
    let output = GStreamerVideoOutput::from_url(&output_url)?;

    // 加载模型
    let model_url = Url::parse("file:///path/to/model.rknn")?;
    let model = shanan::model::Yolo26Builder::from_url(&model_url)?.build()?;

    // 处理视频流
    for frame in input.into_nhwc() {
        let result: DetectResult<CocoLabel> = model.infer(&frame)?;
        output.render_result(&frame, &result)?;
    }

    Ok(())
}
```

### 摄像头实时推流

```rust
use shanan::{
    FromUrl,
    input::GStreamerInputPipelineBuilder,
    output::GStreamerRtspOutput,
};
use url::Url;
use anyhow::Result;

fn main() -> Result<()> {
    // 摄像头输入
    let input = GStreamerInputPipelineBuilder::new()
        .camera("/dev/video0", 1280, 720, 30)
        .target_format("RGB")
        .build()?;

    // RTSP 推流输出
    let output_url = Url::parse(
        "gstrtsp://0.0.0.0/camera?width=1280&height=720&fps=30&port=8554"
    )?;
    let output = GStreamerRtspOutput::from_url(&output_url)?;

    println!("RTSP 流已启动: rtsp://localhost:8554/camera");

    // 推流
    for frame in input.into_nhwc() {
        // output.render_result(&frame, &result)?;
    }

    Ok(())
}
```

## URL Scheme 说明

| Scheme | 用途 | 示例 |
|--------|------|------|
| `image://` | 图像文件输入/输出 | `image:///path/to/file.jpg` |
| `file://` | RKNN 模型文件 | `file:///path/to/model.rknn` |
| `gst://` | GStreamer 管道输入 | `gst://filesrc location=video.mp4 ! ...` |
| `gstvideo://` | 视频文件输出 | `gstvideo:///output.mp4?width=1920&height=1080&fps=30` |
| `gstrtsp://` | RTSP 推流输出 | `gstrtsp://0.0.0.0/live?port=8554` |

## 文档

完整的 API 文档请运行：

```bash
cargo doc --open --features "read_image_file,save_image_file,gstreamer_input,gstreamer_output"
```

## 示例程序

### 运行图像推理示例

```bash
cargo run --bin simple-image -- \
    --model file:///path/to/model.rknn \
    --input image:///path/to/input.jpg \
    --output image:///path/to/output.jpg
```

### 运行摄像头示例

```bash
cargo run --bin simple-camera --features gstreamer_input -- \
    --model file:///path/to/model.rknn \
    --camera /dev/video0 \
    --output image:///path/to/output.jpg
```

## 许可证

本项目采用 GNU Affero 通用公共许可证 v3.0 (AGPL-3.0)。

详见 [LICENSE](LICENSE) 文件。

## 作者

Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

## 贡献

欢迎提交 Issue 和 Pull Request！

## 相关链接

- [Rockchip RKNN Toolkit](https://github.com/rockchip-linux/rknn-toolkit2)
- [GStreamer 文档](https://gstreamer.freedesktop.org/documentation/)
