// 该文件是 Shanan （山南西风） 项目的一部分。
// src/input/v4l2_source.rs - V4L2 摄像头输入源
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use anyhow::{Context, Result};
use image::RgbImage;
use std::time::Instant;
use v4l::FourCC;
use v4l::buffer::Type;
use v4l::io::mmap::Stream;
use v4l::io::traits::CaptureStream;
use v4l::prelude::*;
use v4l::video::Capture;

use super::{Frame, InputSource, InputSourceType};

/// V4L2 摄像头输入源
pub struct V4l2Source {
  /// V4L2 设备（保留所有权以保持设备打开）
  #[allow(dead_code)]
  device: Device,
  /// 捕获流
  stream: Stream<'static>,
  /// 帧索引
  frame_index: u64,
  /// 视频宽度
  width: u32,
  /// 视频高度
  height: u32,
  /// 开始时间
  start_time: Instant,
}

impl V4l2Source {
  /// 创建一个新的 V4L2 摄像头输入源
  pub fn new(device_path: &str) -> Result<Self> {
    let device =
      Device::with_path(device_path).with_context(|| format!("无法打开设备: {}", device_path))?;

    // 设置视频格式
    let mut format = device.format()?;
    format.width = 640;
    format.height = 480;
    format.fourcc = FourCC::new(b"YUYV");
    let format = device.set_format(&format)?;

    let width = format.width;
    let height = format.height;

    // 创建捕获流
    // 使用 unsafe 将设备的生命周期延长
    let device_ptr = &device as *const Device;
    let stream = unsafe {
      Stream::with_buffers(&*device_ptr, Type::VideoCapture, 4).context("无法创建捕获流")?
    };

    Ok(Self {
      device,
      stream: unsafe { std::mem::transmute(stream) },
      frame_index: 0,
      width,
      height,
      start_time: Instant::now(),
    })
  }

  /// 将 YUYV 格式转换为 RGB
  fn yuyv_to_rgb(yuyv: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut rgb = Vec::with_capacity((width * height * 3) as usize);

    for chunk in yuyv.chunks(4) {
      if chunk.len() < 4 {
        break;
      }

      let y0 = chunk[0] as f32;
      let u = chunk[1] as f32 - 128.0;
      let y1 = chunk[2] as f32;
      let v = chunk[3] as f32 - 128.0;

      // 第一个像素
      let r = (y0 + 1.402 * v).clamp(0.0, 255.0) as u8;
      let g = (y0 - 0.344 * u - 0.714 * v).clamp(0.0, 255.0) as u8;
      let b = (y0 + 1.772 * u).clamp(0.0, 255.0) as u8;
      rgb.extend_from_slice(&[r, g, b]);

      // 第二个像素
      let r = (y1 + 1.402 * v).clamp(0.0, 255.0) as u8;
      let g = (y1 - 0.344 * u - 0.714 * v).clamp(0.0, 255.0) as u8;
      let b = (y1 + 1.772 * u).clamp(0.0, 255.0) as u8;
      rgb.extend_from_slice(&[r, g, b]);
    }

    rgb
  }
}

impl Iterator for V4l2Source {
  type Item = Result<Frame>;

  fn next(&mut self) -> Option<Self::Item> {
    match self.stream.next() {
      Ok((buffer, _meta)) => {
        let rgb_data = Self::yuyv_to_rgb(buffer, self.width, self.height);

        let image = match RgbImage::from_raw(self.width, self.height, rgb_data) {
          Some(img) => img,
          None => {
            return Some(Err(anyhow::anyhow!("无法创建 RGB 图像")));
          }
        };

        let timestamp_ms = self.start_time.elapsed().as_millis() as u64;

        let frame = Frame {
          image,
          index: self.frame_index,
          timestamp_ms,
        };

        self.frame_index += 1;
        Some(Ok(frame))
      }
      Err(e) => Some(Err(anyhow::anyhow!("无法捕获帧: {}", e))),
    }
  }
}

impl InputSource for V4l2Source {
  fn source_type(&self) -> InputSourceType {
    InputSourceType::V4l2
  }

  fn width(&self) -> u32 {
    self.width
  }

  fn height(&self) -> u32 {
    self.height
  }

  fn fps(&self) -> Option<f64> {
    Some(30.0) // V4L2 默认帧率
  }
}
