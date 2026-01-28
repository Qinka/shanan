// 该文件是 Shanan （山南西风） 项目的一部分。
// src/input/gstreamer_input.rs - GStreamer 输入
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use crate::{
  FromUrl,
  frame::{RgbNchwFrame, RgbNhwcFrame},
};

use gstreamer::{self as gst, prelude::*};
use gstreamer_app as gst_app;
use gstreamer_video as gst_video;
use thiserror::Error;
use tracing::error;
use url::Url;

#[derive(Error, Debug)]
pub enum GStreamerInputError {
  #[error("URI scheme mismatch")]
  SchemeMismatch,
  #[error("GStreamer error: {0}")]
  GStreamerError(#[from] gst::glib::Error),
  #[error("GStreamer boolean error: {0}")]
  GStreamerBoolError(#[from] gst::glib::BoolError),
  #[error("Failed to get appsink element")]
  AppSinkNotFound,
  #[error("Failed to convert element to appsink")]
  AppSinkConversionFailed,
  #[error("Failed to get video info from caps")]
  VideoInfoError,
  #[error("Unsupported video format")]
  UnsupportedFormat,
  #[error("Pipeline error: {0}")]
  PipelineError(String),
  #[error("Buffer size mismatch: expected {expected} bytes, got {actual} bytes")]
  BufferSizeMismatch { expected: usize, actual: usize },
}

const GSTREAMER_INPUT_SCHEME: &str = "gst";

pub struct GStreamerInput {
  pipeline: gst::Pipeline,
  appsink: gst_app::AppSink,
}

impl FromUrl for GStreamerInput {
  type Error = GStreamerInputError;

  fn from_url(url: &Url) -> Result<Self, Self::Error> {
    if url.scheme() != GSTREAMER_INPUT_SCHEME {
      error!(
        "URI scheme mismatch: expected '{}', found '{}'",
        GSTREAMER_INPUT_SCHEME,
        url.scheme()
      );
      return Err(GStreamerInputError::SchemeMismatch);
    }

    // Initialize GStreamer (subsequent calls are safe no-ops)
    gst::init()?;

    // Parse the pipeline description from the URL path
    let pipeline_desc = url.path();
    
    // Create a full pipeline description with appsink at the end
    let full_pipeline = format!("{} ! appsink name=sink", pipeline_desc);
    
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

impl Drop for GStreamerInput {
  fn drop(&mut self) {
    if let Err(e) = self.pipeline.set_state(gst::State::Null) {
      tracing::warn!("Failed to stop GStreamer pipeline: {}", e);
    }
  }
}

impl GStreamerInput {
  pub fn into_nchw(self) -> GStreamerInputNchw {
    GStreamerInputNchw { inner: self }
  }

  pub fn into_nhwc(self) -> GStreamerInputNhwc {
    GStreamerInputNhwc { inner: self }
  }

  fn pull_sample(&self) -> Option<gst::Sample> {
    self.appsink.pull_sample().ok()
  }
}

pub struct GStreamerInputNchw {
  inner: GStreamerInput,
}

impl Iterator for GStreamerInputNchw {
  type Item = RgbNchwFrame;

  fn next(&mut self) -> Option<Self::Item> {
    let sample = self.inner.pull_sample()?;
    convert_sample_to_nchw(sample).ok()
  }
}

pub struct GStreamerInputNhwc {
  inner: GStreamerInput,
}

impl Iterator for GStreamerInputNhwc {
  type Item = RgbNhwcFrame;

  fn next(&mut self) -> Option<Self::Item> {
    let sample = self.inner.pull_sample()?;
    convert_sample_to_nhwc(sample).ok()
  }
}

fn convert_sample_to_nchw(sample: gst::Sample) -> Result<RgbNchwFrame, GStreamerInputError> {
  let buffer = sample.buffer().ok_or_else(|| {
    GStreamerInputError::PipelineError("No buffer in sample".to_string())
  })?;
  let caps = sample.caps().ok_or_else(|| {
    GStreamerInputError::PipelineError("No caps in sample".to_string())
  })?;
  
  let video_info = gst_video::VideoInfo::from_caps(caps)
    .map_err(|_| GStreamerInputError::VideoInfoError)?;

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

  let mut frame = RgbNchwFrame::with_shape(height, width);
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

fn convert_sample_to_nhwc(sample: gst::Sample) -> Result<RgbNhwcFrame, GStreamerInputError> {
  let buffer = sample.buffer().ok_or_else(|| {
    GStreamerInputError::PipelineError("No buffer in sample".to_string())
  })?;
  let caps = sample.caps().ok_or_else(|| {
    GStreamerInputError::PipelineError("No caps in sample".to_string())
  })?;
  
  let video_info = gst_video::VideoInfo::from_caps(caps)
    .map_err(|_| GStreamerInputError::VideoInfoError)?;

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

  let mut frame = RgbNhwcFrame::with_shape(height, width);
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
