// 该文件是 Shanan （山南西风） 项目的一部分。
// src/output/mod.rs - 输出模块
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

mod image_output;
mod video_output;
mod visualizer;

pub use image_output::ImageOutput;
pub use video_output::VideoOutput;
pub use visualizer::Visualizer;

use anyhow::Result;
use image::RgbImage;

use crate::detector::Detection;

/// 输出写入器 trait
pub trait OutputWriter {
  /// 写入一帧
  fn write_frame(&mut self, image: &RgbImage, detections: &[Detection]) -> Result<()>;

  /// 完成写入
  fn finish(&mut self) -> Result<()>;
}

/// 创建输出写入器
pub fn create_output_writer(
  output_path: &str,
  width: u32,
  height: u32,
  fps: Option<f64>,
) -> Result<Box<dyn OutputWriter>> {
  let lower = output_path.to_lowercase();

  if lower.ends_with(".jpg")
    || lower.ends_with(".jpeg")
    || lower.ends_with(".png")
    || lower.ends_with(".bmp")
  {
    Ok(Box::new(ImageOutput::new(output_path)?))
  } else {
    Ok(Box::new(VideoOutput::new(
      output_path,
      width,
      height,
      fps.unwrap_or(30.0),
    )?))
  }
}
