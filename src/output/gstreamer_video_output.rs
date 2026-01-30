// 该文件是 Shanan （山南西风） 项目的一部分。
// src/output/gstreamer_video_output.rs - GStreamer 视频文件输出
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

//! # GStreamer 视频文件输出模块
//!
//! 将处理后的视频帧保存为视频文件，支持多种格式。
//!
//! ## 支持的格式
//!
//! - **MP4** (H.264) - 最常用的视频格式
//! - **MKV** (Matroska) - 开放标准容器格式
//! - **AVI** - 传统视频格式
//! - **WebM** (VP8) - Web 友好格式
//!
//! ## URL Scheme
//!
//! `gstvideo://`
//!
//! ## 基本用法
//!
//! ```no_run
//! use shanan::{FromUrl, output::GStreamerVideoOutput};
//! use url::Url;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 创建 MP4 视频输出
//! let url = Url::parse("gstvideo:///output.mp4?width=1280&height=720&fps=30")?;
//! let output = GStreamerVideoOutput::from_url(&url)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## 参数说明
//!
//! - `width`: 视频宽度（像素），默认 640
//! - `height`: 视频高度（像素），默认 480
//! - `fps`: 帧率（帧/秒），默认 30
//!
//! ## 完整示例
//!
//! ```no_run
//! use shanan::{
//!     FromUrl,
//!     input::GStreamerInput,
//!     output::GStreamerVideoOutput,
//!     model::{CocoLabel, DetectResult, Model},
//! };
//! use url::Url;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 输入: RTSP 流
//! let input_url = Url::parse(
//!     "gst://rtspsrc location=rtsp://camera.local/stream ! \
//!      decodebin ! videoconvert ! video/x-raw,format=RGB"
//! )?;
//! let input = GStreamerInput::from_url(&input_url)?;
//!
//! // 输出: 视频文件
//! let output_url = Url::parse("gstvideo:///output.mp4?width=1280&height=720&fps=30")?;
//! let output = GStreamerVideoOutput::from_url(&output_url)?;
//!
//! // 处理并保存
//! for frame in input.into_nhwc() {
//!     // output.render_result(&frame, &result)?;
//! }
//! # Ok(())
//! # }
//! ```

use std::sync::{Arc, Mutex};

use crate::{
  FromUrl,
  frame::{RgbNchwFrame, RgbNhwcFrame},
  model::{DetectResult, WithLabel},
  output::{
    Render,
    draw::{draw_detections_nchw_to_nhwc, draw_detections_nhwc_to_nhwc},
  },
};

use gstreamer::{self as gst, prelude::*};
use gstreamer_app as gst_app;
use thiserror::Error;
use tracing::{error, info};
use url::Url;

/// GStreamer 视频输出错误类型
#[derive(Error, Debug)]
pub enum GStreamerVideoOutputError {
  /// URI scheme 不匹配
  #[error("URI scheme mismatch")]
  SchemeMismatch,
  /// GStreamer 库错误
  #[error("GStreamer error: {0}")]
  GStreamerError(#[from] gst::glib::Error),
  /// GStreamer 布尔操作错误
  #[error("GStreamer boolean error: {0}")]
  GStreamerBoolError(#[from] gst::glib::BoolError),
  /// 无法获取 appsrc 元素
  #[error("Failed to get appsrc element")]
  AppSrcNotFound,
  /// 无法转换元素为 appsrc
  #[error("Failed to convert element to appsrc")]
  AppSrcConversionFailed,
  /// 管道错误
  #[error("Pipeline error: {0}")]
  PipelineError(String),
  /// 状态改变错误
  #[error("State change error: {0}")]
  StateChangeError(#[from] gst::StateChangeError),
  /// 缓冲区创建错误
  #[error("Buffer creation error")]
  BufferCreationError,
}

const GSTREAMER_VIDEO_OUTPUT_SCHEME: &str = "gst";

/// GStreamer 视频文件输出
///
/// 管理 GStreamer 编码管道，将视频帧保存为文件。
///
/// # 示例
///
/// ```no_run
/// use shanan::{FromUrl, output::GStreamerVideoOutput};
/// use url::Url;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let url = Url::parse("gstvideo:///output.mp4?width=1920&height=1080&fps=30")?;
/// let output = GStreamerVideoOutput::from_url(&url)?;
/// # Ok(())
/// # }
/// ```
pub struct GStreamerVideoOutput<const W: u32, const H: u32> {
  pipeline: gst::Pipeline,
  appsrc: gst_app::AppSrc,
  fps: i32,
  frame_count: Arc<Mutex<u64>>,
}

impl<const W: u32, const H: u32> FromUrl for GStreamerVideoOutput<W, H> {
  type Error = GStreamerVideoOutputError;

  fn from_url(url: &Url) -> Result<Self, Self::Error> {
    if url.scheme() != GSTREAMER_VIDEO_OUTPUT_SCHEME {
      error!(
        "URI scheme mismatch: expected '{}', found '{}'",
        GSTREAMER_VIDEO_OUTPUT_SCHEME,
        url.scheme()
      );
      return Err(GStreamerVideoOutputError::SchemeMismatch);
    }

    // Initialize GStreamer (subsequent calls are safe no-ops)
    gst::init()?;

    // Parse query parameters for width, height, fps
    let query_pairs: std::collections::HashMap<_, _> = url.query_pairs().collect();
    let fps: i32 = query_pairs
      .get("fps")
      .and_then(|v| v.parse().ok())
      .unwrap_or(30);

    // Get the output file path
    let file_path = url.path();

    // Build pipeline based on file extension
    let pipeline_desc = if file_path.ends_with(".mp4") {
      format!(
        "appsrc name=src ! videoconvert ! video/x-raw,format=I420 ! x264enc speed-preset=fast tune=zerolatency ! h264parse ! mp4mux ! filesink location={}",
        file_path
      )
    } else if file_path.ends_with(".mkv") {
      format!(
        "appsrc name=src ! videoconvert ! video/x-raw,format=I420 ! x264enc speed-preset=fast ! h264parse ! matroskamux ! filesink location={}",
        file_path
      )
    } else if file_path.ends_with(".avi") {
      format!(
        "appsrc name=src ! videoconvert ! video/x-raw,format=I420 ! x264enc ! avimux ! filesink location={}",
        file_path
      )
    } else if file_path.ends_with(".webm") {
      format!(
        "appsrc name=src ! videoconvert ! vp8enc ! webmmux ! filesink location={}",
        file_path
      )
    } else {
      // Default to MP4
      format!(
        "appsrc name=src ! videoconvert ! video/x-raw,format=I420 ! x264enc speed-preset=fast tune=zerolatency ! h264parse ! mp4mux ! filesink location={}",
        file_path
      )
    };

    info!("Creating video output pipeline: {}", pipeline_desc);

    // Create the pipeline
    let pipeline = gst::parse::launch(&pipeline_desc)?
      .downcast::<gst::Pipeline>()
      .map_err(|_| {
        GStreamerVideoOutputError::PipelineError("Failed to create pipeline".to_string())
      })?;

    // Get the appsrc element
    let appsrc = pipeline
      .by_name("src")
      .ok_or(GStreamerVideoOutputError::AppSrcNotFound)?
      .downcast::<gst_app::AppSrc>()
      .map_err(|_| GStreamerVideoOutputError::AppSrcConversionFailed)?;

    // Configure appsrc
    let caps = gst::Caps::builder("video/x-raw")
      .field("format", "RGB")
      .field("width", W as i32)
      .field("height", H as i32)
      .field("framerate", gst::Fraction::new(fps, 1))
      .build();

    appsrc.set_caps(Some(&caps));
    appsrc.set_format(gst::Format::Time);

    // Start the pipeline
    pipeline.set_state(gst::State::Playing)?;

    info!(
      "Video output initialized: {}x{} @ {} fps -> {}",
      W, H, fps, file_path
    );

    Ok(GStreamerVideoOutput {
      pipeline,
      appsrc,
      fps,
      frame_count: Arc::new(Mutex::new(0)),
    })
  }
}

impl<const W: u32, const H: u32> Drop for GStreamerVideoOutput<W, H> {
  fn drop(&mut self) {
    // Send EOS to properly close the file
    let _ = self.appsrc.end_of_stream();

    // Wait a bit for EOS to be processed
    std::thread::sleep(std::time::Duration::from_millis(100));

    if let Err(e) = self.pipeline.set_state(gst::State::Null) {
      tracing::warn!("Failed to stop GStreamer video output pipeline: {}", e);
    }

    let frame_count = self.frame_count.lock().unwrap();
    info!(
      "Video output closed. Total frames written: {}",
      *frame_count
    );
  }
}

impl<const W: u32, const H: u32> GStreamerVideoOutput<W, H> {
  fn push_frame(&self, data: &[u8]) -> Result<(), GStreamerVideoOutputError> {
    let size = data.len();
    let mut buffer =
      gst::Buffer::with_size(size).map_err(|_| GStreamerVideoOutputError::BufferCreationError)?;

    {
      let buffer_ref = buffer.get_mut().unwrap();
      let mut buffer_map = buffer_ref.map_writable().map_err(|_| {
        GStreamerVideoOutputError::PipelineError("Failed to map buffer".to_string())
      })?;
      buffer_map.copy_from_slice(data);
    }

    // Set timestamp
    let mut frame_count = self.frame_count.lock().unwrap();
    let timestamp = (*frame_count * 1_000_000_000) / (self.fps as u64);
    *frame_count += 1;

    {
      let buffer_ref = buffer.get_mut().unwrap();
      buffer_ref.set_pts(gst::ClockTime::from_nseconds(timestamp));
      buffer_ref.set_duration(gst::ClockTime::from_nseconds(
        1_000_000_000 / self.fps as u64,
      ));
    }

    self.appsrc.push_buffer(buffer).map_err(|e| {
      GStreamerVideoOutputError::PipelineError(format!("Failed to push buffer: {:?}", e))
    })?;

    Ok(())
  }
}

impl<const W: u32, const H: u32, T: WithLabel> Render<RgbNchwFrame<W, H>, DetectResult<T>>
  for GStreamerVideoOutput<W, H>
{
  type Error = GStreamerVideoOutputError;

  fn render_result(
    &self,
    frame: &RgbNchwFrame<W, H>,
    result: &DetectResult<T>,
  ) -> Result<(), Self::Error> {
    let rgb_data = draw_detections_nchw_to_nhwc(frame, result);
    self.push_frame(&rgb_data)
  }
}

impl<const W: u32, const H: u32, T: WithLabel> Render<RgbNhwcFrame<W, H>, DetectResult<T>>
  for GStreamerVideoOutput<W, H>
{
  type Error = GStreamerVideoOutputError;

  fn render_result(
    &self,
    frame: &RgbNhwcFrame<W, H>,
    result: &DetectResult<T>,
  ) -> Result<(), Self::Error> {
    let rgb_data = draw_detections_nhwc_to_nhwc(frame, result);
    self.push_frame(&rgb_data)
  }
}
