// 该文件是 Shanan （山南西风） 项目的一部分。
// src/input/gstreamer_input.rs - GStreamer 输入
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

//! # GStreamer 视频输入模块
//!
//! 本模块提供基于 GStreamer 的视频输入功能，支持多种视频源：
//! - 视频文件读取
//! - 摄像头捕获（V4L2）
//! - RTSP 网络流
//! - 测试视频源
//!
//! ## 系统依赖
//!
//! 使用前需要安装 GStreamer 开发库：
//!
//! **Ubuntu/Debian:**
//! ```bash
//! sudo apt-get install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev
//! ```
//!
//! **Fedora/RHEL:**
//! ```bash
//! sudo dnf install gstreamer1-devel gstreamer1-plugins-base-devel
//! ```
//!
//! **macOS:**
//! ```bash
//! brew install gstreamer
//! ```
//!
//! ## Cargo 特性
//!
//! 在 `Cargo.toml` 中启用 `gstreamer_input` 特性：
//!
//! ```toml
//! [dependencies]
//! shanan = { version = "0.1", features = ["gstreamer_input"] }
//! ```
//!
//! ## 基本用法
//!
//! ```no_run
//! use shanan::{FromUrl, input::GStreamerInput};
//! use url::Url;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 从视频文件读取
//! let url = Url::parse("gst://filesrc location=video.mp4 ! decodebin ! videoconvert ! video/x-raw,format=RGB")?;
//! let input = GStreamerInput::from_url(&url)?;
//!
//! // 处理每一帧
//! for frame in input.into_nhwc() {
//!     println!("处理帧: {}x{}", frame.width(), frame.height());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## RTSP 流示例
//!
//! ```no_run
//! use shanan::{FromUrl, input::GStreamerInput};
//! use url::Url;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 从 RTSP 流读取
//! let url = Url::parse(
//!     "gst://rtspsrc location=rtsp://192.168.1.100:8554/stream ! \
//!      decodebin ! videoconvert ! video/x-raw,format=RGB"
//! )?;
//! let input = GStreamerInput::from_url(&url)?;
//!
//! for frame in input.into_nhwc() {
//!     // 处理帧
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## 摄像头捕获
//!
//! ```no_run
//! use shanan::{FromUrl, input::GStreamerInput};
//! use url::Url;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let url = Url::parse(
//!     "gst://v4l2src device=/dev/video0 ! \
//!      videoconvert ! video/x-raw,format=RGB,width=640,height=480"
//! )?;
//! let input = GStreamerInput::from_url(&url)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Pipeline Builder
//!
//! 使用 `GStreamerInputPipelineBuilder` 构建复杂管道：
//!
//! ```no_run
//! use shanan::input::GStreamerInputPipelineBuilder;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let input = GStreamerInputPipelineBuilder::new()
//!     .camera("/dev/video0", 640, 480, 30)
//!     .target_format("RGB")
//!     .build()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## 支持的视频格式
//!
//! - RGB - 标准 RGB 格式
//! - BGR - BGR 格式（会自动转换为 RGB）
//!
//! 其他格式需要在管道中使用 `videoconvert` 插件转换。
//!
//! ## 安全性注意
//!
//! GStreamer 管道描述直接传递给解析器。在生产环境中使用不可信输入时，
//! 应验证或限制管道描述以防止资源滥用。

use std::collections::HashMap;

use crate::{
  FromUrl,
  frame::{RgbNchwFrame, RgbNhwcFrame},
};

use gstreamer::{self as gst, prelude::*};
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;
use thiserror::Error;
use tracing::{error, info};
use url::Url;

/// GStreamer 输入错误类型
///
/// 包含所有可能的 GStreamer 输入相关错误。
#[derive(Error, Debug)]
pub enum GStreamerInputError {
  /// URI scheme 不匹配（期望 "gst://"）
  #[error("URI scheme mismatch")]
  SchemeMismatch,
  /// GStreamer 库错误
  #[error("GStreamer error: {0}")]
  GStreamerError(#[from] gst::glib::Error),
  /// GStreamer 布尔操作错误
  #[error("GStreamer boolean error: {0}")]
  GStreamerBoolError(#[from] gst::glib::BoolError),
  /// 无法获取 appsink 元素
  #[error("Failed to get appsink element")]
  AppSinkNotFound,
  /// 无法转换元素为 appsink
  #[error("Failed to convert element to appsink")]
  AppSinkConversionFailed,
  /// 无法从 caps 获取视频信息
  #[error("Failed to get video info from caps")]
  VideoInfoError,
  /// 不支持的视频格式
  #[error("Unsupported video format")]
  UnsupportedFormat,
  /// 管道错误
  #[error("Pipeline error: {0}")]
  PipelineError(String),
  /// 缓冲区大小不匹配
  #[error("Buffer size mismatch: expected {expected} bytes, got {actual} bytes")]
  BufferSizeMismatch { expected: usize, actual: usize },
  /// 状态改变错误
  #[error("State change error: {0}")]
  StateChangeError(#[from] gst::StateChangeError),
}

const GSTREAMER_INPUT_SCHEME: &str = "gst";

pub enum GStreamerInputBuilderItem {
  FileSource(String),
  CameraSource {
    camera: String,
    io_mode: Option<u32>,
    format: String,
    width: u32,
    height: u32,
    fps: u32,
  },
  TargetFormat {
    format: String,
  },
  AspectRatio {
    ratio: (u32, u32),
  },
  VideoFlip {
    method: u32,
    direction: u32,
  },
}

impl GStreamerInputBuilderItem {
  fn to_pipeline(&self) -> String {
    match self {
      GStreamerInputBuilderItem::FileSource(path) => {
        format!("filesrc location={} ! decodebin", path)
      }
      GStreamerInputBuilderItem::CameraSource {
        camera,
        io_mode,
        format,
        width,
        height,
        fps,
      } => {
        let io_mode_str = if let Some(mode) = io_mode {
          format!(" io-mode={}", mode)
        } else {
          "".to_string()
        };
        format!(
          "v4l2src device={}{} ! video/x-raw,format={},width={},height={},framerate={}/1",
          camera, io_mode_str, format, width, height, fps
        )
      }
      GStreamerInputBuilderItem::TargetFormat { format } => {
        format!("videoconvert ! video/x-raw,format={}", format)
      }
      GStreamerInputBuilderItem::AspectRatio { ratio } => {
        format!("aspectratiocrop aspect-ratio={}/{}", ratio.0, ratio.1)
      }
      GStreamerInputBuilderItem::VideoFlip { method, direction } => {
        format!("videoflip method={} video-direction={}", method, direction)
      }
    }
  }
}

/// GStreamer 输入管道构建器
///
/// 用于构建复杂的 GStreamer 输入管道。
///
/// # 示例
///
/// ```no_run
/// use shanan::input::GStreamerInputPipelineBuilder;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let input = GStreamerInputPipelineBuilder::new()
///     .camera("/dev/video0", 640, 480, 30)
///     .target_format("RGB")
///     .build()?;
/// # Ok(())
/// # }
/// ```
pub struct GStreamerInputPipelineBuilder<const W: u32, const H: u32> {
  items: Vec<GStreamerInputBuilderItem>,
}

impl<const W: u32, const H: u32> GStreamerInputPipelineBuilder<W, H> {
  fn build_video_pipline(
    path: &str,
    query: &HashMap<String, String>,
  ) -> Result<Self, GStreamerInputError> {
    let camera = path.to_string();
    let io_mode = query.get("io-mode").and_then(|v| v.parse::<u32>().ok());
    let format = query
      .get("format")
      .map(String::from)
      .unwrap_or(String::from("RGB"));
    let width = query
      .get("width")
      .and_then(|v| v.parse::<u32>().ok())
      .unwrap_or(W);
    let height = query
      .get("height")
      .and_then(|v| v.parse::<u32>().ok())
      .unwrap_or(H);
    let fps = query
      .get("fps")
      .and_then(|v| v.parse::<u32>().ok())
      .unwrap_or(15);

    let mut items = Vec::new();
    items.push(GStreamerInputBuilderItem::CameraSource {
      camera,
      io_mode,
      format,
      width,
      height,
      fps,
    });
    items.push(GStreamerInputBuilderItem::AspectRatio { ratio: (W, H) });

    if let Some(video_flip) = Self::video_flip(query.get("rotate").map(|s| s.as_ref())) {
      items.push(video_flip);
    }

    Ok(GStreamerInputPipelineBuilder { items })
  }

  fn build_file_pipeline(
    path: &str,
    query: &HashMap<String, String>,
  ) -> Result<Self, GStreamerInputError> {
    let mut items = Vec::new();
    items.push(GStreamerInputBuilderItem::FileSource(path.to_string()));
    items.push(GStreamerInputBuilderItem::AspectRatio { ratio: (W, H) });

    if let Some(video_flip) = Self::video_flip(query.get("rotate").map(|s| s.as_ref())) {
      items.push(video_flip);
    }

    Ok(GStreamerInputPipelineBuilder { items })
  }

  fn video_flip(rotate: Option<&str>) -> Option<GStreamerInputBuilderItem> {
    if let Some(rotate) = rotate {
      let (method, direction) = match rotate {
        "0" => (0, 0),
        "90" => (1, 1),
        "180" => (2, 2),
        "270" => (3, 3),
        _ => (0, 0),
      };
      Some(GStreamerInputBuilderItem::VideoFlip { method, direction })
    } else {
      None
    }
  }

  pub fn build(self) -> Result<GStreamerInput<W, H>, GStreamerInputError> {
    gst::init()?;

    let basic_pipeline = self
      .items
      .iter()
      .map(GStreamerInputBuilderItem::to_pipeline)
      .collect::<Vec<String>>()
      .join(" ! ");
    let full_pipeline = format!(
      "{} ! appsink max-buffers=2 drop=true name=sink",
      basic_pipeline
    );

    info!("GStreamer pipeline description: {}", full_pipeline);

    // Create the pipeline from the description
    let pipeline = gst::parse::launch(&full_pipeline)?
      .downcast::<gst::Pipeline>()
      .map_err(|_| GStreamerInputError::PipelineError("Failed to create pipeline".to_string()))?;

    // Get the appsink element
    let appsink = pipeline
      .by_name("sink")
      .ok_or(GStreamerInputError::AppSinkNotFound)?
      .downcast::<gst_app::AppSink>()
      .map_err(|_| GStreamerInputError::AppSinkConversionFailed)?;

    // Start the pipeline
    pipeline.set_state(gst::State::Playing)?;

    Ok(GStreamerInput { pipeline, appsink })
  }
}

impl<const W: u32, const H: u32> FromUrl for GStreamerInputPipelineBuilder<W, H> {
  type Error = GStreamerInputError;

  fn from_url(url: &Url) -> Result<Self, Self::Error> {
    if url.scheme() != GSTREAMER_INPUT_SCHEME {
      return Err(GStreamerInputError::SchemeMismatch);
    }

    let query: HashMap<String, String> = url
      .query_pairs()
      .map(|(k, v)| (String::from(k), String::from(v)))
      .collect();

    // unpack url
    let mut builder = match url.host_str() {
      Some("camera") => Self::build_video_pipline(url.path(), &query)?,
      Some("file") => Self::build_file_pipeline(url.path(), &query)?,
      _ => {
        return Err(GStreamerInputError::SchemeMismatch);
      }
    };

    builder.items.push(GStreamerInputBuilderItem::TargetFormat {
      format: "RGB".to_string(),
    });

    Ok(builder)
  }
}

/// GStreamer 视频输入
///
/// 管理 GStreamer 管道和 appsink，提供视频帧迭代功能。
///
/// # 示例
///
/// ```no_run
/// use shanan::{FromUrl, input::GStreamerInput};
/// use url::Url;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let url = Url::parse("gst://videotestsrc ! videoconvert ! video/x-raw,format=RGB")?;
/// let input = GStreamerInput::from_url(&url)?;
///
/// for frame in input.into_nhwc() {
///     // 处理帧
/// }
/// # Ok(())
/// # }
/// ```
pub struct GStreamerInput<const W: u32, const H: u32> {
  pipeline: gst::Pipeline,
  appsink: gst_app::AppSink,
}

impl<const W: u32, const H: u32> Drop for GStreamerInput<W, H> {
  fn drop(&mut self) {
    if let Err(e) = self.pipeline.set_state(gst::State::Null) {
      tracing::warn!("Failed to stop GStreamer pipeline: {}", e);
    }
  }
}

impl<const W: u32, const H: u32> GStreamerInput<W, H> {
  pub fn into_nchw(self) -> GStreamerInputNchw<W, H> {
    GStreamerInputNchw { inner: self }
  }

  pub fn into_nhwc(self) -> GStreamerInputNhwc<W, H> {
    GStreamerInputNhwc { inner: self }
  }

  fn pull_sample(&self) -> Option<gst::Sample> {
    self
      .appsink
      .pull_sample()
      .map_err(|e| {
        error!("Failed to pull sample: {}", e);
        e
      })
      .ok()
  }
}

/// GStreamer 输入的 NCHW 格式迭代器
///
/// 将视频帧转换为 NCHW 格式（Channels × Height × Width）。
pub struct GStreamerInputNchw<const W: u32, const H: u32> {
  inner: GStreamerInput<W, H>,
}

impl<const W: u32, const H: u32> Iterator for GStreamerInputNchw<W, H> {
  type Item = RgbNchwFrame<W, H>;

  fn next(&mut self) -> Option<Self::Item> {
    let sample = self.inner.pull_sample()?;
    convert_sample_to_nchw(sample)
      .map_err(|e| {
        error!("Failed to fetch sample: {}", e);
        e
      })
      .ok()
  }
}

/// GStreamer 输入的 NHWC 格式迭代器
///
/// 将视频帧转换为 NHWC 格式（Height × Width × Channels）。
pub struct GStreamerInputNhwc<const W: u32, const H: u32> {
  inner: GStreamerInput<W, H>,
}

impl<const W: u32, const H: u32> Iterator for GStreamerInputNhwc<W, H> {
  type Item = RgbNhwcFrame<W, H>;

  fn next(&mut self) -> Option<Self::Item> {
    let sample = self.inner.pull_sample()?;
    convert_sample_to_nhwc(sample)
      .map_err(|e| {
        error!("Failed to fetch sample: {}", e);
        e
      })
      .ok()
  }
}

fn convert_sample_to_nchw<const W: u32, const H: u32>(
  sample: gst::Sample,
) -> Result<RgbNchwFrame<W, H>, GStreamerInputError> {
  let buffer = sample
    .buffer()
    .ok_or_else(|| GStreamerInputError::PipelineError("No buffer in sample".to_string()))?;
  let caps = sample
    .caps()
    .ok_or_else(|| GStreamerInputError::PipelineError("No caps in sample".to_string()))?;

  let video_info =
    gst_video::VideoInfo::from_caps(caps).map_err(|_| GStreamerInputError::VideoInfoError)?;

  let width = video_info.width() as usize;
  let height = video_info.height() as usize;

  let map = buffer.map_readable().map_err(|e| {
    GStreamerInputError::PipelineError(format!("Failed to map buffer for reading: {}", e))
  })?;
  let data = map.as_slice();

  // Validate buffer size
  let expected_size = height * width * 3;
  let actual_size = data.len();
  if actual_size < expected_size {
    return Err(GStreamerInputError::BufferSizeMismatch {
      expected: expected_size,
      actual: actual_size,
    });
  }

  let mut frame = RgbNchwFrame::<W, H>::default();
  let frame_slice = frame.as_mut();

  // Convert from whatever format to RGB NCHW
  // This assumes the input is RGB or can be converted to RGB
  match video_info.format() {
    gst_video::VideoFormat::Rgb => {
      // RGB to NCHW: reorganize from HWC to CHW
      for h in 0..height {
        for w in 0..width {
          for c in 0..3 {
            let src_idx = (h * width + w) * 3 + c;
            let dst_idx = c * height * width + h * width + w;
            frame_slice[dst_idx] = data[src_idx];
          }
        }
      }
    }
    gst_video::VideoFormat::Bgr => {
      // BGR to RGB NCHW
      for h in 0..height {
        for w in 0..width {
          for c in 0..3 {
            let src_idx = (h * width + w) * 3 + (2 - c); // Reverse BGR to RGB
            let dst_idx = c * height * width + h * width + w;
            frame_slice[dst_idx] = data[src_idx];
          }
        }
      }
    }
    _ => return Err(GStreamerInputError::UnsupportedFormat),
  }

  Ok(frame)
}

fn convert_sample_to_nhwc<const W: u32, const H: u32>(
  sample: gst::Sample,
) -> Result<RgbNhwcFrame<W, H>, GStreamerInputError> {
  let buffer = sample
    .buffer()
    .ok_or_else(|| GStreamerInputError::PipelineError("No buffer in sample".to_string()))?;
  let caps = sample
    .caps()
    .ok_or_else(|| GStreamerInputError::PipelineError("No caps in sample".to_string()))?;

  let video_info =
    gst_video::VideoInfo::from_caps(caps).map_err(|_| GStreamerInputError::VideoInfoError)?;

  let width = video_info.width() as usize;
  let height = video_info.height() as usize;

  let map = buffer.map_readable().map_err(|e| {
    GStreamerInputError::PipelineError(format!("Failed to map buffer for reading: {}", e))
  })?;
  let data = map.as_slice();

  // Validate buffer size
  let expected_size = height * width * 3;
  let actual_size = data.len();
  if actual_size < expected_size {
    return Err(GStreamerInputError::BufferSizeMismatch {
      expected: expected_size,
      actual: actual_size,
    });
  }

  let mut frame = RgbNhwcFrame::<W, H>::default();
  let frame_slice = frame.as_mut();

  // Convert from whatever format to RGB NHWC
  match video_info.format() {
    gst_video::VideoFormat::Rgb => {
      // Already in HWC format, just copy
      let size = height * width * 3;
      frame_slice[..size].copy_from_slice(&data[..size]);
    }
    gst_video::VideoFormat::Bgr => {
      // BGR to RGB, keep HWC layout
      for h in 0..height {
        for w in 0..width {
          for c in 0..3 {
            let src_idx = (h * width + w) * 3 + (2 - c); // Reverse BGR to RGB
            let dst_idx = (h * width + w) * 3 + c;
            frame_slice[dst_idx] = data[src_idx];
          }
        }
      }
    }
    _ => return Err(GStreamerInputError::UnsupportedFormat),
  }

  Ok(frame)
}
