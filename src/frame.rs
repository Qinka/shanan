// 该文件是 Shanan （山南西风） 项目的一部分。
// src/frame.rs - NCHW 帧定义
//
// 本文件根据 Apache 许可证第 2.0 版（以下简称“许可证”）授权使用；
// 除非遵守该许可证条款，否则您不得使用本文件。
// 您可通过以下网址获取许可证副本：
// http://www.apache.org/licenses/LICENSE-2.0
// 除非适用法律要求或书面同意，根据本许可协议分发的软件均按“原样”提供，
// 不附带任何形式的明示或暗示的保证或条件。
// 有关许可权限与限制的具体条款，请参阅本许可协议。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, Wareless Group

use crate::input::{AsNchwFrame, AsNhwcFrame};

const RGB_CHANNELS: usize = 3;

pub trait FrameFormat {
  fn tensor_format(&self) -> rknpu::TensorFormat;
  fn tensor_type(&self) -> rknpu::TensorType;
}

#[derive(Debug, Clone)]
pub struct RgbNchwFrame<const W: u32, const H: u32> {
  data: Box<[u8]>,
}

impl<const W: u32, const H: u32> From<Vec<u8>> for RgbNchwFrame<W, H> {
  fn from(data: Vec<u8>) -> Self {
    if data.len() != (RGB_CHANNELS * W as usize * H as usize) {
      panic!(
        "数据长度不匹配: 期望长度 {}, 实际长度 {}",
        RGB_CHANNELS * W as usize * H as usize,
        data.len()
      );
    }

    Self {
      data: data.into_boxed_slice(),
    }
  }
}

impl<const W: u32, const H: u32> FrameFormat for RgbNchwFrame<W, H> {
  fn tensor_format(&self) -> rknpu::TensorFormat {
    rknpu::TensorFormat::NCHW
  }

  fn tensor_type(&self) -> rknpu::TensorType {
    rknpu::TensorType::UInt8
  }
}

impl<const W: u32, const H: u32> Default for RgbNchwFrame<W, H> {
  fn default() -> Self {
    let size = RGB_CHANNELS * (W as usize) * (H as usize);
    let data = vec![0u8; size].into_boxed_slice();
    Self { data }
  }
}

impl<const W: u32, const H: u32> RgbNchwFrame<W, H> {
  pub fn height(&self) -> usize {
    H as usize
  }

  pub fn width(&self) -> usize {
    W as usize
  }

  pub fn channels(&self) -> usize {
    RGB_CHANNELS
  }
}

impl<const W: u32, const H: u32> AsMut<[u8]> for RgbNchwFrame<W, H> {
  fn as_mut(&mut self) -> &mut [u8] {
    &mut self.data
  }
}

impl<const W: u32, const H: u32> AsNchwFrame<W, H> for RgbNchwFrame<W, H> {
  fn as_nchw(&self) -> &[u8] {
    &self.data
  }
}

#[derive(Debug, Clone)]
pub struct RgbNhwcFrame<const W: u32, const H: u32> {
  data: Box<[u8]>,
}

impl<const W: u32, const H: u32> From<Vec<u8>> for RgbNhwcFrame<W, H> {
  fn from(data: Vec<u8>) -> Self {
    if data.len() != (RGB_CHANNELS * W as usize * H as usize) {
      panic!(
        "数据长度不匹配: 期望长度 {}, 实际长度 {}",
        RGB_CHANNELS * W as usize * H as usize,
        data.len()
      );
    }

    Self {
      data: data.into_boxed_slice(),
    }
  }
}

impl<const W: u32, const H: u32> FrameFormat for RgbNhwcFrame<W, H> {
  fn tensor_format(&self) -> rknpu::TensorFormat {
    rknpu::TensorFormat::NHWC
  }

  fn tensor_type(&self) -> rknpu::TensorType {
    rknpu::TensorType::UInt8
  }
}

impl<const W: u32, const H: u32> Default for RgbNhwcFrame<W, H> {
  fn default() -> Self {
    let size = RGB_CHANNELS * (W as usize) * (H as usize);
    let data = vec![0u8; size].into_boxed_slice();
    Self { data }
  }
}

impl<const W: u32, const H: u32> RgbNhwcFrame<W, H> {
  pub fn height(&self) -> usize {
    H as usize
  }

  pub fn width(&self) -> usize {
    W as usize
  }

  pub fn channels(&self) -> usize {
    RGB_CHANNELS
  }
}

impl<const W: u32, const H: u32> AsMut<[u8]> for RgbNhwcFrame<W, H> {
  fn as_mut(&mut self) -> &mut [u8] {
    &mut self.data
  }
}

impl<const W: u32, const H: u32> AsNhwcFrame<W, H> for RgbNhwcFrame<W, H> {
  fn as_nhwc(&self) -> &[u8] {
    &self.data
  }
}
