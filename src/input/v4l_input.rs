// 该文件是 Shanan （山南西风） 项目的一部分。
// src/input/v4l_input.rs - V4L 视频输入
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

use thiserror::Error;
use tracing::error;
use url::Url;

#[derive(Error, Debug)]
pub enum V4lInputError {
  #[error("URI schema mismatch")]
  SchemaMismatch,
  #[error("I/O error: {0}")]
  IoError(std::io::Error),
  #[error("V4L error: {0}")]
  V4lError(String),
  #[error("Invalid device path")]
  InvalidDevicePath,
  #[error("Unsupported pixel format")]
  UnsupportedPixelFormat,
}

impl From<std::io::Error> for V4lInputError {
  fn from(err: std::io::Error) -> Self {
    V4lInputError::IoError(err)
  }
}

const V4L_SCHEME: &str = "v4l";

pub struct V4lInput {
  device_path: String,
  width: usize,
  height: usize,
}

impl FromUrl for V4lInput {
  type Error = V4lInputError;

  fn from_url(url: &Url) -> Result<Self, Self::Error> {
    if url.scheme() != V4L_SCHEME {
      error!(
        "URI scheme mismatch: expected '{}', found '{}'",
        V4L_SCHEME,
        url.scheme()
      );
      return Err(V4lInputError::SchemaMismatch);
    }

    // Extract device path from URL
    // Expected format: v4l:///dev/video0 or v4l://localhost/dev/video0
    let device_path = if url.path().is_empty() {
      "/dev/video0".to_string()
    } else {
      url.path().to_string()
    };

    // Open the device to validate and get format information
    let device = v4l::Device::with_path(&device_path)
      .map_err(|e| V4lInputError::V4lError(e.to_string()))?;

    // Get current format to determine dimensions
    let format = device.format()
      .map_err(|e| V4lInputError::V4lError(e.to_string()))?;
    
    let width = format.width as usize;
    let height = format.height as usize;

    Ok(V4lInput {
      device_path,
      width,
      height,
    })
  }
}

impl V4lInput {
  pub fn into_nchw(self) -> V4lInputNchw {
    V4lInputNchw { inner: self }
  }

  pub fn into_nhwc(self) -> V4lInputNhwc {
    V4lInputNhwc { inner: self }
  }

  fn capture_frame(&mut self) -> Result<Vec<u8>, V4lInputError> {
    // Open device for this capture
    let mut device = v4l::Device::with_path(&self.device_path)
      .map_err(|e| V4lInputError::V4lError(e.to_string()))?;

    // Create a stream for capturing with memory-mapped buffers
    let mut stream = v4l::io::mmap::Stream::with_buffers(&mut device, v4l::buffer::Type::VideoCapture, 4)
      .map_err(|e| V4lInputError::V4lError(e.to_string()))?;

    // Capture one frame
    let (buf, _meta) = stream.next()
      .map_err(|e| V4lInputError::V4lError(e.to_string()))?;

    // Convert the buffer to RGB format
    // This is a simplified implementation - in practice, you'd need to handle
    // different pixel formats and convert them appropriately
    Ok(buf.to_vec())
  }
}

pub struct V4lInputNchw {
  inner: V4lInput,
}

impl Iterator for V4lInputNchw {
  type Item = RgbNchwFrame;

  fn next(&mut self) -> Option<Self::Item> {
    match self.inner.capture_frame() {
      Ok(data) => {
        // Convert raw buffer to RgbNchwFrame
        let mut frame = RgbNchwFrame::with_shape(self.inner.height, self.inner.width);
        
        // Note: This assumes the data is already in RGB format
        // In a real implementation, you'd need to convert from the actual
        // pixel format (e.g., YUYV, MJPEG, etc.) to RGB
        let channels = frame.channels();
        let height = frame.height();
        let width = frame.width();
        let slice = frame.as_mut();

        // Simple copy assuming RGB24 format
        // For NCHW: data is organized as [R0...Rn, G0...Gn, B0...Bn]
        if data.len() >= channels * height * width {
          for c in 0..channels {
            for h in 0..height {
              for w in 0..width {
                let src_idx = (h * width + w) * channels + c;
                let dst_idx = c * height * width + h * width + w;
                slice[dst_idx] = data[src_idx];
              }
            }
          }
          Some(frame)
        } else {
          error!("Captured buffer size mismatch");
          None
        }
      }
      Err(e) => {
        error!("Failed to capture frame: {}", e);
        None
      }
    }
  }
}

pub struct V4lInputNhwc {
  inner: V4lInput,
}

impl Iterator for V4lInputNhwc {
  type Item = RgbNhwcFrame;

  fn next(&mut self) -> Option<Self::Item> {
    match self.inner.capture_frame() {
      Ok(data) => {
        // Convert raw buffer to RgbNhwcFrame
        let mut frame = RgbNhwcFrame::with_shape(self.inner.height, self.inner.width);
        
        // Note: This assumes the data is already in RGB format
        // In a real implementation, you'd need to convert from the actual
        // pixel format (e.g., YUYV, MJPEG, etc.) to RGB
        let channels = frame.channels();
        let height = frame.height();
        let width = frame.width();
        let slice = frame.as_mut();

        // Simple copy assuming RGB24 format
        // For NHWC: data is already in the right format [R0,G0,B0, R1,G1,B1, ...]
        let copy_size = std::cmp::min(data.len(), channels * height * width);
        slice[..copy_size].copy_from_slice(&data[..copy_size]);
        
        Some(frame)
      }
      Err(e) => {
        error!("Failed to capture frame: {}", e);
        None
      }
    }
  }
}
