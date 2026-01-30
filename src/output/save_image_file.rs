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

use thiserror::Error;
use tracing::warn;
use url::Url;

use crate::{
  FromUrl,
  frame::{RgbNchwFrame, RgbNhwcFrame},
  model::{DetectResult, WithLabel},
  output::{
    Render,
    draw::{draw_detections, nchw_to_image, nhwc_to_image},
  },
};

pub struct SaveImageFileOutput<const W: u32, const H: u32> {
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

impl<const W: u32, const H: u32> FromUrl for SaveImageFileOutput<W, H> {
  type Error = SaveImageFileError;

  fn from_url(uri: &Url) -> Result<Self, Self::Error> {
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

impl<const W: u32, const H: u32> SaveImageFileOutput<W, H> {
  fn save_image_with_detections<T: WithLabel>(
    &self,
    mut image: image::RgbImage,
    result: &DetectResult<T>,
  ) -> Result<(), SaveImageFileError> {
    // 绘制检测框和标签
    draw_detections(&mut image, result);

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
}

impl<const W: u32, const H: u32, T: WithLabel> Render<RgbNchwFrame<W, H>, DetectResult<T>>
  for SaveImageFileOutput<W, H>
{
  type Error = SaveImageFileError;

  fn render_result(
    &self,
    frame: &RgbNchwFrame<W, H>,
    result: &DetectResult<T>,
  ) -> Result<(), Self::Error> {
    let image = nchw_to_image(frame);
    self.save_image_with_detections(image, result)
  }
}

impl<const W: u32, const H: u32, T: WithLabel> Render<RgbNhwcFrame<W, H>, DetectResult<T>>
  for SaveImageFileOutput<W, H>
{
  type Error = SaveImageFileError;

  fn render_result(
    &self,
    frame: &RgbNhwcFrame<W, H>,
    result: &DetectResult<T>,
  ) -> Result<(), Self::Error> {
    let image = nhwc_to_image(frame);
    self.save_image_with_detections(image, result)
  }
}
