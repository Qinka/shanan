// 该文件是 Shanan （山南西风） 项目的一部分。
// src/output/gstreamer_rtsp_output.rs - GStreamer RTSP 推流输出
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

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

#[derive(Error, Debug)]
pub enum GStreamerRtspOutputError {
  #[error("URI scheme mismatch")]
  SchemeMismatch,
  #[error("GStreamer error: {0}")]
  GStreamerError(#[from] gst::glib::Error),
  #[error("GStreamer boolean error: {0}")]
  GStreamerBoolError(#[from] gst::glib::BoolError),
  #[error("Failed to get appsrc element")]
  AppSrcNotFound,
  #[error("Failed to convert element to appsrc")]
  AppSrcConversionFailed,
  #[error("Pipeline error: {0}")]
  PipelineError(String),
  #[error("State change error: {0}")]
  StateChangeError(#[from] gst::StateChangeError),
  #[error("Buffer creation error")]
  BufferCreationError,
}

const GSTREAMER_RTSP_OUTPUT_SCHEME: &str = "rtsp";

pub struct GStreamerRtspOutput {
  pipeline: gst::Pipeline,
  appsrc: gst_app::AppSrc,
  _width: usize,
  _height: usize,
  fps: i32,
  frame_count: Arc<Mutex<u64>>,
}

impl FromUrl for GStreamerRtspOutput {
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
    let width: usize = query_pairs
      .get("width")
      .and_then(|v| v.parse().ok())
      .unwrap_or(640);
    let height: usize = query_pairs
      .get("height")
      .and_then(|v| v.parse().ok())
      .unwrap_or(480);
    let fps: i32 = query_pairs
      .get("fps")
      .and_then(|v| v.parse().ok())
      .unwrap_or(30);
    let port: u16 = query_pairs
      .get("port")
      .and_then(|v| v.parse().ok())
      .unwrap_or(8554);

    // Get the host and stream path
    let host = url.host_str().unwrap_or("0.0.0.0");
    let stream_path = url.path();

    // Build RTSP server pipeline using UDP sink
    // Note: This creates a simple UDP stream that can be consumed via RTSP
    // For a full RTSP server, you would need gst-rtsp-server library
    let pipeline_desc = format!(
      "appsrc name=src ! videoconvert ! video/x-raw,format=I420 ! \
       x264enc speed-preset=ultrafast tune=zerolatency bitrate=2000 ! \
       h264parse ! rtph264pay config-interval=1 pt=96 ! \
       udpsink host={} port={}",
      host, port
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
      .field("width", width as i32)
      .field("height", height as i32)
      .field("framerate", gst::Fraction::new(fps, 1))
      .build();

    appsrc.set_caps(Some(&caps));
    appsrc.set_format(gst::Format::Time);
    appsrc.set_property("is-live", true);

    // Start the pipeline
    pipeline.set_state(gst::State::Playing)?;

    info!(
      "RTSP output initialized: {}x{} @ {} fps on port {}",
      width, height, fps, port
    );

    Ok(GStreamerRtspOutput {
      pipeline,
      appsrc,
      _width: width,
      _height: height,
      fps,
      frame_count: Arc::new(Mutex::new(0)),
    })
  }
}

impl Drop for GStreamerRtspOutput {
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

impl GStreamerRtspOutput {
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

impl<T: WithLabel> Render<RgbNchwFrame, DetectResult<T>> for GStreamerRtspOutput {
  type Error = GStreamerRtspOutputError;

  fn render_result(
    &self,
    frame: &RgbNchwFrame,
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

impl<T: WithLabel> Render<RgbNhwcFrame, DetectResult<T>> for GStreamerRtspOutput {
  type Error = GStreamerRtspOutputError;

  fn render_result(
    &self,
    frame: &RgbNhwcFrame,
    _result: &DetectResult<T>,
  ) -> Result<(), Self::Error> {
    let data = frame.as_nhwc();
    self.push_frame(data)
  }
}
