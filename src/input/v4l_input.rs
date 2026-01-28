// 该文件是 Shanan （山南西风） 项目的一部分。
// src/input/v4l_input.rs - V4L 视频输入
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

//! V4L (Video4Linux) 视频输入模块
//!
//! 此模块提供从 V4L 设备读取视频帧的功能，类似于 ImageFileInput。
//!
//! # 使用示例
//!
//! ```no_run
//! use shanan::{FromUrl, input::V4lInput};
//! use url::Url;
//!
//! // 从默认设备读取
//! let url = Url::parse("v4l:///dev/video0").unwrap();
//! let input = V4lInput::from_url(&url).unwrap();
//!
//! // 使用 NHWC 格式迭代帧
//! for frame in input.into_nhwc() {
//!     // 处理帧数据
//!     println!("Captured frame: {}x{}", frame.width(), frame.height());
//! }
//! ```
//!
//! # URL 格式
//!
//! - `v4l:///dev/video0` - 指定视频设备路径
//! - `v4l://` - 使用默认设备 `/dev/video0`

use crate::{
  frame::{RgbNchwFrame, RgbNhwcFrame},
  FromUrl,
};

use std::path::Path;
use thiserror::Error;
use tracing::{error, info, warn};
use url::Url;
use v4l::{io::traits::CaptureStream, video::Capture};

/// V4L 输入错误类型
#[derive(Error, Debug)]
pub enum V4lInputError {
  #[error("URI scheme mismatch")]
  SchemeMismatch,
  #[error("I/O error: {0}")]
  IoError(std::io::Error),
  #[error("V4L error: {0}")]
  V4lError(String),
  #[error("Device not found: {0}")]
  DeviceNotFound(String),
  #[error("Invalid device path: {0}")]
  InvalidDevicePath(String),
  #[error("Unsupported pixel format")]
  UnsupportedPixelFormat,
  #[error("Permission denied: {0}")]
  PermissionDenied(String),
}

impl From<std::io::Error> for V4lInputError {
  fn from(err: std::io::Error) -> Self {
    V4lInputError::IoError(err)
  }
}

const V4L_SCHEME: &str = "v4l";

/// V4L 视频输入源
///
/// 通过 Video4Linux API 从视频设备读取帧数据。
/// 支持转换为 NCHW 或 NHWC 格式的帧迭代器。
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
      return Err(V4lInputError::SchemeMismatch);
    }

    // Extract device path from URL
    // Expected format: v4l:///dev/video0 or v4l://localhost/dev/video0
    let device_path = if url.path().is_empty() || url.path() == "/" {
      "/dev/video0".to_string()
    } else {
      url.path().to_string()
    };

    info!("尝试打开 V4L 设备: {}", device_path);

    // Check if the device path exists
    if !Path::new(&device_path).exists() {
      error!("设备不存在: {}", device_path);
      return Err(V4lInputError::DeviceNotFound(format!(
        "设备文件不存在: {}. 请确认设备已连接并且路径正确。",
        device_path
      )));
    }

    // Check if the path is a character device (typical for V4L devices)
    let metadata = std::fs::metadata(&device_path).map_err(|e| {
      error!("无法读取设备元数据: {}: {}", device_path, e);
      if e.kind() == std::io::ErrorKind::PermissionDenied {
        V4lInputError::PermissionDenied(format!(
          "没有权限访问设备: {}. 请检查用户权限或使用 sudo 运行。",
          device_path
        ))
      } else {
        V4lInputError::IoError(e)
      }
    })?;

    // On Unix systems, check if it's a character device
    #[cfg(unix)]
    {
      use std::os::unix::fs::FileTypeExt;
      if !metadata.file_type().is_char_device() {
        warn!("路径不是字符设备: {}", device_path);
        return Err(V4lInputError::InvalidDevicePath(format!(
          "{} 不是一个字符设备。V4L 设备通常是字符设备。",
          device_path
        )));
      }
    }

    // Open the device to validate and get format information
    let mut device = v4l::Device::with_path(&device_path).map_err(|e| {
      error!("无法打开 V4L 设备 {}: {}", device_path, e);
      let err_msg = e.to_string();
      if err_msg.contains("Permission denied") {
        V4lInputError::PermissionDenied(format!(
          "打开设备 {} 时权限被拒绝。请检查用户权限或使用 sudo 运行。",
          device_path
        ))
      } else if err_msg.contains("Invalid argument") {
        V4lInputError::V4lError(format!(
          "打开设备 {} 时参数无效。设备可能不支持 V4L2 或正在被其他程序使用。",
          device_path
        ))
      } else {
        V4lInputError::V4lError(format!("打开设备失败: {}", err_msg))
      }
    })?;

    info!("成功打开设备: {}", device_path);

    // Try to get current format
    let format = match device.format() {
      Ok(fmt) => {
        info!(
          "当前设备格式: {}x{}, fourcc: {:?}",
          fmt.width, fmt.height, fmt.fourcc
        );
        fmt
      }
      Err(e) => {
        warn!("无法获取设备格式，尝试设置默认格式: {}", e);

        // Try to set a common format (640x480, YUYV)
        let mut fmt = v4l::Format::new(640, 480, v4l::FourCC::new(b"YUYV"));
        match device.set_format(&fmt) {
          Ok(set_fmt) => {
            info!("成功设置默认格式: {}x{}", set_fmt.width, set_fmt.height);
            set_fmt
          }
          Err(set_err) => {
            error!("无法设置格式: {}", set_err);
            return Err(V4lInputError::V4lError(format!(
              "无法获取或设置设备格式。设备: {}, 获取错误: {}, 设置错误: {}",
              device_path, e, set_err
            )));
          }
        }
      }
    };

    let width = format.width as usize;
    let height = format.height as usize;

    info!("V4L 输入初始化成功: {}x{}", width, height);

    Ok(V4lInput {
      device_path,
      width,
      height,
    })
  }
}

impl V4lInput {
  /// 转换为 NCHW 格式的帧迭代器
  pub fn into_nchw(self) -> V4lInputNchw {
    V4lInputNchw { inner: self }
  }

  /// 转换为 NHWC 格式的帧迭代器
  pub fn into_nhwc(self) -> V4lInputNhwc {
    V4lInputNhwc { inner: self }
  }

  fn capture_frame(&mut self) -> Result<Vec<u8>, V4lInputError> {
    // NOTE: This implementation reopens the device for each frame capture.
    // For better performance, consider refactoring to keep the device and stream
    // open between captures. This requires handling lifetimes appropriately.

    // Open device for this capture
    let mut device = v4l::Device::with_path(&self.device_path).map_err(|e| {
      error!("重新打开设备失败 {}: {}", self.device_path, e);
      V4lInputError::V4lError(format!("无法重新打开设备: {}", e))
    })?;

    // Get the current format and ensure it matches our expected dimensions
    let mut format = device.format().map_err(|e| {
      error!("获取设备格式失败: {}", e);
      V4lInputError::V4lError(format!("无法获取设备格式: {}", e))
    })?;

    // Set the format to our desired dimensions
    format.width = self.width as u32;
    format.height = self.height as u32;

    let actual_format = device.set_format(&format).map_err(|e| {
      error!("设置设备格式失败: {}", e);
      V4lInputError::V4lError(format!(
        "无法设置设备格式为 {}x{}: {}",
        self.width, self.height, e
      ))
    })?;

    // Log if the device adjusted the format
    if actual_format.width as usize != self.width || actual_format.height as usize != self.height {
      warn!(
        "设备调整了格式: 请求 {}x{}, 实际 {}x{}",
        self.width, self.height, actual_format.width, actual_format.height
      );
    }

    // Create a stream for capturing with memory-mapped buffers
    let mut stream =
      v4l::io::mmap::Stream::with_buffers(&mut device, v4l::buffer::Type::VideoCapture, 4)
        .map_err(|e| {
          error!("创建捕获流失败: {}", e);
          V4lInputError::V4lError(format!(
            "无法创建视频捕获流。设备可能正在被其他程序使用或不支持内存映射: {}",
            e
          ))
        })?;

    // Capture one frame
    let (buf, meta) = stream.next().map_err(|e| {
      error!("捕获帧失败: {}", e);
      V4lInputError::V4lError(format!("无法捕获视频帧: {}", e))
    })?;

    info!("成功捕获帧: {} 字节, 序列号: {}", buf.len(), meta.sequence);

    // Convert the buffer to RGB format
    // This is a simplified implementation - in practice, you'd need to handle
    // different pixel formats and convert them appropriately
    Ok(buf.to_vec())
  }
}

/// NCHW 格式的 V4L 帧迭代器
///
/// 将 V4L 设备捕获的帧转换为 NCHW (Batch, Channel, Height, Width) 格式。
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

        // Simple copy assuming RGB24 format (interleaved: R,G,B,R,G,B,...)
        // Convert to NCHW: data is organized as [R0...Rn, G0...Gn, B0...Bn]
        let expected_size = channels * height * width;
        if data.len() < expected_size {
          error!(
            "Captured buffer size mismatch: expected {}, got {}",
            expected_size,
            data.len()
          );
          return None;
        }

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
      }
      Err(e) => {
        error!("Failed to capture frame: {}", e);
        None
      }
    }
  }
}

/// NHWC 格式的 V4L 帧迭代器
///
/// 将 V4L 设备捕获的帧转换为 NHWC (Batch, Height, Width, Channel) 格式。
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

        // Simple copy assuming RGB24 format (interleaved: R,G,B,R,G,B,...)
        // For NHWC: data is already in the right format [R0,G0,B0, R1,G1,B1, ...]
        let expected_size = channels * height * width;
        if data.len() < expected_size {
          error!(
            "Captured buffer size mismatch: expected {}, got {}",
            expected_size,
            data.len()
          );
          return None;
        }

        slice[..expected_size].copy_from_slice(&data[..expected_size]);

        Some(frame)
      }
      Err(e) => {
        error!("Failed to capture frame: {}", e);
        None
      }
    }
  }
}
