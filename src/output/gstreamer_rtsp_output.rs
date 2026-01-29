// 该文件是 Shanan （山南西风） 项目的一部分。
// src/output/gstreamer_rtsp_output.rs - GStreamer RTSP 推流输出
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

//! # GStreamer RTSP 推流输出模块
//!
//! 通过 RTSP 协议实时推送视频流。
//!
//! ## URL Scheme
//!
//! `gstrtsp://`
//!
//! ## 基本用法
//!
//! ```no_run
//! use shanan::{FromUrl, output::GStreamerRtspOutput};
//! use url::Url;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 创建 RTSP 推流输出
//! let url = Url::parse("gstrtsp://0.0.0.0/live?width=1280&height=720&fps=30&port=8554")?;
//! let output = GStreamerRtspOutput::from_url(&url)?;
//! # Ok(())
//! # }
//! ```
//!
//! ## 参数说明
//!
//! - `width`: 视频宽度（像素），默认 640
//! - `height`: 视频高度（像素），默认 480
//! - `fps`: 帧率（帧/秒），默认 30
//! - `port`: UDP 端口，默认 8554
//!
//! ## 客户端连接
//!
//! 推流启动后，可以使用以下方式连接：
//!
//! ```bash
//! # VLC
//! vlc rtsp://服务器IP:8554/live
//!
//! # FFplay
//! ffplay -rtsp_transport udp rtsp://服务器IP:8554/live
//!
//! # GStreamer
//! gst-launch-1.0 rtspsrc location=rtsp://服务器IP:8554/live ! decodebin ! autovideosink
//! ```
//!
//! ## 完整示例
//!
//! ```no_run
//! use shanan::{
//!     FromUrl,
//!     input::GStreamerInput,
//!     output::GStreamerRtspOutput,
//!     model::{CocoLabel, DetectResult, Model},
//! };
//! use url::Url;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 输入: 摄像头
//! let input_url = Url::parse(
//!     "gst://v4l2src device=/dev/video0 ! \
//!      videoconvert ! video/x-raw,format=RGB"
//! )?;
//! let input = GStreamerInput::from_url(&input_url)?;
//!
//! // 输出: RTSP 推流
//! let output_url = Url::parse("gstrtsp://0.0.0.0/camera?port=8554")?;
//! let output = GStreamerRtspOutput::from_url(&output_url)?;
//!
//! println!("RTSP 流已启动: rtsp://localhost:8554/camera");
//!
//! // 处理并推流
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
  input::{AsNchwFrame, AsNhwcFrame},
  model::{DetectResult, WithLabel},
  output::Render,
};

use gstreamer::{self as gst, prelude::*};
use gstreamer_app as gst_app;
use thiserror::Error;
use tracing::{error, info};
use url::Url;

/// GStreamer RTSP 输出错误类型
#[derive(Error, Debug)]
pub enum GStreamerRtspOutputError {
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

const GSTREAMER_RTSP_OUTPUT_SCHEME: &str = "rtsp";

/// GStreamer RTSP 推流输出
///
/// 管理 GStreamer RTSP 编码管道，实时推送视频流。
///
/// # 示例
///
/// ```no_run
/// use shanan::{FromUrl, output::GStreamerRtspOutput};
/// use url::Url;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let url = Url::parse("gstrtsp://0.0.0.0/live?width=1280&height=720&fps=30&port=8554")?;
/// let output = GStreamerRtspOutput::from_url(&url)?;
/// # Ok(())
/// # }
/// ```
pub struct GStreamerRtspOutput<const W: u32, const H: u32> {
  pipeline: gst::Pipeline,
  appsrc: gst_app::AppSrc,
  fps: i32,
  frame_count: Arc<Mutex<u64>>,
}

impl<const W: u32, const H: u32> FromUrl for GStreamerRtspOutput<W, H> {
  type Error = GStreamerRtspOutputError;

  fn from_url(url: &Url) -> Result<Self, Self::Error> {
    if url.scheme() != GSTREAMER_RTSP_OUTPUT_SCHEME {
      error!(
        "URI scheme mismatch: expected '{}', found '{}'",
        GSTREAMER_RTSP_OUTPUT_SCHEME,
        url.scheme()
      );
      return Err(GStreamerRtspOutputError::SchemeMismatch);
    }

    // Initialize GStreamer (subsequent calls are safe no-ops)
    gst::init()?;

    // Parse query parameters for width, height, fps, port
    let query_pairs: std::collections::HashMap<_, _> = url.query_pairs().collect();
    let fps: i32 = query_pairs
      .get("fps")
      .and_then(|v| v.parse().ok())
      .unwrap_or(30);
    let port: u16 = query_pairs
      .get("port")
      .and_then(|v| v.parse().ok())
      .unwrap_or(8554);
    let proto = query_pairs
      .get("proto")
      .map(|v| v.as_ref())
      .unwrap_or("udp");

    // Get the host and stream path
    let host = url.host_str().unwrap_or("0.0.0.0");
    let stream_path = url.path();

    // Build RTSP server pipeline using UDP sink
    // Note: This creates a simple UDP stream that can be consumed via RTSP
    // For a full RTSP server, you would need gst-rtsp-server library
    let pipeline_desc = format!(
      "appsrc name=src ! videoconvert ! video/x-raw,format=I420 ! \
       mpph264enc ! \
       rtspclientsink protocols={} latency=0 location=rtsp://{}:{}{}",
      proto, host, port, stream_path
    );

    info!("Creating RTSP output pipeline: {}", pipeline_desc);
    info!(
      "Stream will be available at: rtsp://{}:{}{} (client needs to connect)",
      host, port, stream_path
    );

    // Create the pipeline
    let pipeline = gst::parse::launch(&pipeline_desc)?
      .downcast::<gst::Pipeline>()
      .map_err(|_| {
        GStreamerRtspOutputError::PipelineError("Failed to create pipeline".to_string())
      })?;

    // Get the appsrc element
    let appsrc = pipeline
      .by_name("src")
      .ok_or(GStreamerRtspOutputError::AppSrcNotFound)?
      .downcast::<gst_app::AppSrc>()
      .map_err(|_| GStreamerRtspOutputError::AppSrcConversionFailed)?;

    // Configure appsrc
    let caps = gst::Caps::builder("video/x-raw")
      .field("format", "RGB")
      .field("width", W as i32)
      .field("height", H as i32)
      .field("framerate", gst::Fraction::new(fps, 1))
      .build();

    appsrc.set_caps(Some(&caps));
    appsrc.set_format(gst::Format::Time);
    appsrc.set_property("is-live", true);

    // Start the pipeline
    pipeline.set_state(gst::State::Playing)?;

    info!(
      "RTSP output initialized: {}x{} @ {} fps on port {}",
      W, H, fps, port
    );

    Ok(GStreamerRtspOutput {
      pipeline,
      appsrc,
      fps,
      frame_count: Arc::new(Mutex::new(0)),
    })
  }
}

impl<const W: u32, const H: u32> Drop for GStreamerRtspOutput<W, H> {
  fn drop(&mut self) {
    if let Err(e) = self.pipeline.set_state(gst::State::Null) {
      tracing::warn!("Failed to stop GStreamer RTSP output pipeline: {}", e);
    }

    let frame_count = self.frame_count.lock().unwrap();
    info!(
      "RTSP output closed. Total frames streamed: {}",
      *frame_count
    );
  }
}

impl<const W: u32, const H: u32> GStreamerRtspOutput<W, H> {
  fn push_frame(&self, data: &[u8]) -> Result<(), GStreamerRtspOutputError> {
    let size = data.len();
    let mut buffer =
      gst::Buffer::with_size(size).map_err(|_| GStreamerRtspOutputError::BufferCreationError)?;

    {
      let buffer_ref = buffer.get_mut().unwrap();
      let mut buffer_map = buffer_ref
        .map_writable()
        .map_err(|_| GStreamerRtspOutputError::PipelineError("Failed to map buffer".to_string()))?;
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
      GStreamerRtspOutputError::PipelineError(format!("Failed to push buffer: {:?}", e))
    })?;

    Ok(())
  }
}

impl<const W: u32, const H: u32, T: WithLabel> Render<RgbNchwFrame<W, H>, DetectResult<T>>
  for GStreamerRtspOutput<W, H>
{
  type Error = GStreamerRtspOutputError;

  fn render_result(
    &self,
    frame: &RgbNchwFrame<W, H>,
    _result: &DetectResult<T>,
  ) -> Result<(), Self::Error> {
    let width = frame.width();
    let height = frame.height();
    let nchw_data = frame.as_nchw();

    // Convert NCHW to RGB (HWC format) for GStreamer
    let mut rgb_data = vec![0u8; width * height * 3];
    for h in 0..height {
      for w in 0..width {
        for c in 0..3 {
          let src_idx = c * height * width + h * width + w;
          let dst_idx = (h * width + w) * 3 + c;
          rgb_data[dst_idx] = nchw_data[src_idx];
        }
      }
    }

    self.push_frame(&rgb_data)
  }
}

impl<const W: u32, const H: u32, T: WithLabel> Render<RgbNhwcFrame<W, H>, DetectResult<T>>
  for GStreamerRtspOutput<W, H>
{
  type Error = GStreamerRtspOutputError;

  fn render_result(
    &self,
    frame: &RgbNhwcFrame<W, H>,
    _result: &DetectResult<T>,
  ) -> Result<(), Self::Error> {
    let data = frame.as_nhwc();
    self.push_frame(data)
  }
}
