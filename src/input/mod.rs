// 该文件是 Shanan （山南西风） 项目的一部分。
// src/input/mod.rs - 输入源模块
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

mod image_source;
mod v4l2_source;
mod video_source;

use anyhow::Result;
use image::RgbImage;

pub use image_source::ImageSource;
pub use v4l2_source::V4l2Source;
pub use video_source::VideoSource;

/// 帧数据
pub struct Frame {
  /// RGB 图像数据
  pub image: RgbImage,
  /// 帧索引
  pub index: u64,
  /// 时间戳（毫秒）
  pub timestamp_ms: u64,
}

/// 输入源类型
pub enum InputSourceType {
  /// 图片文件
  Image,
  /// 视频文件
  Video,
  /// V4L2 摄像头
  V4l2,
}

/// 输入源 trait
pub trait InputSource: Iterator<Item = Result<Frame>> {
  /// 获取输入源类型
  fn source_type(&self) -> InputSourceType;

  /// 获取帧宽度
  fn width(&self) -> u32;

  /// 获取帧高度
  fn height(&self) -> u32;

  /// 获取帧率（如果适用）
  fn fps(&self) -> Option<f64>;
}

/// 从路径创建输入源
pub fn create_input_source(source: &str) -> Result<Box<dyn InputSource>> {
  // 检查是否是 V4L2 设备
  if source.starts_with("/dev/video") || source.starts_with("v4l2://") {
    let device_path = if source.starts_with("v4l2://") {
      source.trim_start_matches("v4l2://")
    } else {
      source
    };
    return Ok(Box::new(V4l2Source::new(device_path)?));
  }

  // 检查是否是图片文件
  let lower = source.to_lowercase();
  if lower.ends_with(".jpg")
    || lower.ends_with(".jpeg")
    || lower.ends_with(".png")
    || lower.ends_with(".bmp")
    || lower.ends_with(".gif")
    || lower.ends_with(".webp")
  {
    return Ok(Box::new(ImageSource::new(source)?));
  }

  // 否则视为视频文件
  Ok(Box::new(VideoSource::new(source)?))
}
