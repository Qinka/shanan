// 该文件是 Shanan （山南西风） 项目的一部分。
// src/output/save_image_file.rs - 保存图像文件
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

use std::path::Path;

use thiserror::Error;
use tracing::warn;
use url::Url;

use crate::{
  FromUrl, FromUrlWithScheme,
  frame::{RgbNchwFrame, RgbNhwcFrame},
  model::{DetectResult, WithLabel},
  output::{
    Render,
    draw::{Draw, DrawDetectionOnFrame},
  },
};

pub struct SaveImageFileOutput<'a, const W: u32, const H: u32> {
  path: String,
  draw: Draw<'a>,
}

#[derive(Error, Debug)]
pub enum SaveImageFileError {
  #[error("I/O 错误: {0}")]
  IoError(std::io::Error),
  #[error("图像错误: {0}")]
  ImageError(image::ImageError),
  #[error("URI 方案不匹配: {0}")]
  SchemeMismatch(String),
}

impl<'a, const W: u32, const H: u32> FromUrlWithScheme for SaveImageFileOutput<'a, W, H> {
  const SCHEME: &'static str = "image";
}

impl<'a, const W: u32, const H: u32> FromUrl for SaveImageFileOutput<'a, W, H> {
  type Error = SaveImageFileError;

  fn from_url(uri: &Url) -> Result<Self, Self::Error> {
    if uri.scheme() != Self::SCHEME {
      return Err(SaveImageFileError::SchemeMismatch(format!(
        "期望保存方式 '{}', 实际保存方式 '{}'",
        Self::SCHEME,
        uri.scheme()
      )));
    }

    Ok(SaveImageFileOutput {
      path: uri.path().to_string(),
      draw: Draw::default(),
    })
  }
}

impl<'a, const W: u32, const H: u32> SaveImageFileOutput<'a, W, H> {
  fn save_image(&self, image: image::RgbImage) -> Result<(), SaveImageFileError> {
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

impl<'a, const W: u32, const H: u32, T: WithLabel> Render<RgbNchwFrame<W, H>, DetectResult<T>>
  for SaveImageFileOutput<'a, W, H>
{
  type Error = SaveImageFileError;

  fn render_result(
    &self,
    frame: &RgbNchwFrame<W, H>,
    result: &DetectResult<T>,
  ) -> Result<(), Self::Error> {
    let image = self.draw.draw_detection(frame, result);
    self.save_image(image)
  }
}

impl<'a, const W: u32, const H: u32, T: WithLabel> Render<RgbNhwcFrame<W, H>, DetectResult<T>>
  for SaveImageFileOutput<'a, W, H>
{
  type Error = SaveImageFileError;

  fn render_result(
    &self,
    frame: &RgbNhwcFrame<W, H>,
    result: &DetectResult<T>,
  ) -> Result<(), Self::Error> {
    let image = self.draw.draw_detection(frame, result);
    self.save_image(image)
  }
}
