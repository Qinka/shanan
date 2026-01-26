// 该文件是 Shanan （山南西风） 项目的一部分。
// src/input/image_source.rs - 图片输入源
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use anyhow::{Context, Result};
use image::{ImageReader, RgbImage};

use super::{Frame, InputSource, InputSourceType};

/// 图片输入源
pub struct ImageSource {
  /// 图片数据
  image: Option<RgbImage>,
  /// 图片宽度
  width: u32,
  /// 图片高度
  height: u32,
  /// 是否已读取
  consumed: bool,
}

impl ImageSource {
  /// 创建一个新的图片输入源
  pub fn new(path: &str) -> Result<Self> {
    let img = ImageReader::open(path)
      .with_context(|| format!("无法打开图片文件: {}", path))?
      .decode()
      .with_context(|| format!("无法解码图片文件: {}", path))?
      .to_rgb8();

    let width = img.width();
    let height = img.height();

    Ok(Self {
      image: Some(img),
      width,
      height,
      consumed: false,
    })
  }
}

impl Iterator for ImageSource {
  type Item = Result<Frame>;

  fn next(&mut self) -> Option<Self::Item> {
    if self.consumed {
      return None;
    }

    self.consumed = true;

    self.image.take().map(|image| {
      Ok(Frame {
        image,
        index: 0,
        timestamp_ms: 0,
      })
    })
  }
}

impl InputSource for ImageSource {
  fn source_type(&self) -> InputSourceType {
    InputSourceType::Image
  }

  fn width(&self) -> u32 {
    self.width
  }

  fn height(&self) -> u32 {
    self.height
  }

  fn fps(&self) -> Option<f64> {
    None
  }
}
