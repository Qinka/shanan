// 该文件是 Shanan （山南西风） 项目的一部分。
// src/output/save_image_file.rs - 保存图像文件
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use std::path::Path;

use image::{ImageBuffer, Rgb, RgbImage};
use thiserror::Error;
use tracing::warn;
use url::Url;

use crate::{
  frame::RgbNchwFrame,
  input::AsNchwFrame,
  model::{DetectItem, DetectResult},
  output::Render,
};

// 在图像上绘制一个矩形边框，bbox 为归一化坐标 [x_min, y_min, x_max, y_max]
fn draw_bbox(image: &mut RgbImage, bbox: &[f32; 4], color: [u8; 3]) {
  let (w, h) = (image.width() as f32, image.height() as f32);

  let mut x_min = (bbox[0] * w).floor() as i32;
  let mut y_min = (bbox[1] * h).floor() as i32;
  let mut x_max = (bbox[2] * w).ceil() as i32;
  let mut y_max = (bbox[3] * h).ceil() as i32;

  // Clamp to image bounds
  x_min = x_min.clamp(0, w as i32 - 1);
  y_min = y_min.clamp(0, h as i32 - 1);
  x_max = x_max.clamp(0, w as i32 - 1);
  y_max = y_max.clamp(0, h as i32 - 1);

  if x_min >= x_max || y_min >= y_max {
    return;
  }

  // Top and bottom edges
  for x in x_min..=x_max {
    let top = image.get_pixel_mut(x as u32, y_min as u32);
    *top = Rgb(color);
    let bottom = image.get_pixel_mut(x as u32, y_max as u32);
    *bottom = Rgb(color);
  }

  // Left and right edges
  for y in y_min..=y_max {
    let left = image.get_pixel_mut(x_min as u32, y as u32);
    *left = Rgb(color);
    let right = image.get_pixel_mut(x_max as u32, y as u32);
    *right = Rgb(color);
  }
}

pub struct SaveImageFileOutput {
  path: String,
}

#[derive(Error, Debug)]
pub enum SaveImageFileError {
  #[error("I/O 错误: {0}")]
  IoError(std::io::Error),
  #[error("图像错误: {0}")]
  ImageError(image::ImageError),
  #[error("URI 方案不匹配: {0}")]
  SchemaMismatch(String),
}

const SAVE_IMAGE_FILE_SCHEME: &str = "image";

impl SaveImageFileOutput {
  pub fn new(uri: &Url) -> Result<Self, SaveImageFileError> {
    if uri.scheme() != SAVE_IMAGE_FILE_SCHEME {
      return Err(SaveImageFileError::SchemaMismatch(format!(
        "期望保存方式 '{}', 实际保存方式 '{}'",
        SAVE_IMAGE_FILE_SCHEME,
        uri.scheme()
      )));
    }

    Ok(SaveImageFileOutput {
      path: uri.path().to_string(),
    })
  }
}

impl Render for SaveImageFileOutput {
  type Frame = RgbNchwFrame;
  type Output = DetectResult;
  type Error = SaveImageFileError;

  fn render_result(&self, frame: &Self::Frame, result: Self::Output) -> Result<(), Self::Error> {
    let width = frame.width() as u32;
    let height = frame.height() as u32;
    let data = frame.as_nchw();

    // 将 NCHW 转为 RGB 图像
    let mut image: RgbImage = ImageBuffer::from_fn(width, height, |x, y| {
      let x = x as usize;
      let y = y as usize;
      let idx = y * (width as usize) + x;
      let r = data[idx];
      let g = data[(height as usize * width as usize) + idx];
      let b = data[(2 * height as usize * width as usize) + idx];
      Rgb([r, g, b])
    });

    // 绘制检测框
    for DetectItem {
      class_id: _,
      score: _,
      bbox,
    } in result.items.iter()
    {
      draw_bbox(
        &mut image,
        bbox,
        [255, 0, 0], // 红色边框
      );
    }

    if let Some(parent) = Path::new(&self.path).parent()
      && !parent.as_os_str().is_empty()
    {
      std::fs::create_dir_all(parent).map_err(SaveImageFileError::IoError)?;
    }

    image
      .save(&self.path)
      .map_err(SaveImageFileError::ImageError)?;

    warn!("保存图像到文件: {}", self.path);

    Ok(())
  }

  fn from_uri(uri: &url::Url) -> Result<Self, Self::Error> {
    SaveImageFileOutput::new(uri)
  }
}
