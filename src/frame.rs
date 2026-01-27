// 该文件是 Shanan （山南西风） 项目的一部分。
// src/frame.rs - NCHW 帧定义
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use crate::input::AsNchwFrame;

const RGB_CHANNELS: usize = 3;

pub struct RgbNchwFrame {
  data: Box<[u8]>,
  height: usize,
  width: usize,
}

impl RgbNchwFrame {
  pub fn with_shape(height: usize, width: usize) -> Self {
    let size = RGB_CHANNELS * height * width;
    let data = vec![0u8; size].into_boxed_slice();
    Self {
      data,
      height,
      width,
    }
  }

  pub fn height(&self) -> usize {
    self.height
  }

  pub fn width(&self) -> usize {
    self.width
  }

  pub fn channels(&self) -> usize {
    RGB_CHANNELS
  }
}

impl AsMut<[u8]> for RgbNchwFrame {
  fn as_mut(&mut self) -> &mut [u8] {
    &mut self.data
  }
}

impl<'a> AsNchwFrame<'a> for RgbNchwFrame {
  fn as_nchw(&'a self) -> &'a [u8] {
    &self.data
  }
}
