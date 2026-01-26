// 该文件是 Shanan （山南西风） 项目的一部分。
// src/output/image_output.rs - 图片输出
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use anyhow::{Context, Result};
use image::RgbImage;

use super::{OutputWriter, Visualizer};
use crate::detector::Detection;

/// 图片输出
pub struct ImageOutput {
  /// 输出路径
  output_path: String,
  /// 可视化工具
  visualizer: Visualizer,
}

impl ImageOutput {
  /// 创建一个新的图片输出
  pub fn new(output_path: &str) -> Result<Self> {
    Ok(Self {
      output_path: output_path.to_string(),
      visualizer: Visualizer::new(),
    })
  }
}

impl OutputWriter for ImageOutput {
  fn write_frame(&mut self, image: &RgbImage, detections: &[Detection]) -> Result<()> {
    let mut output_image = image.clone();
    self
      .visualizer
      .draw_detections(&mut output_image, detections);

    output_image
      .save(&self.output_path)
      .with_context(|| format!("无法保存图片: {}", self.output_path))?;

    Ok(())
  }

  fn finish(&mut self) -> Result<()> {
    Ok(())
  }
}
