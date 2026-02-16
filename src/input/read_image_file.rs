// 该文件是 Shanan （山南西风） 项目的一部分。
// src/input/image_file.rs - 图像文件输入
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

use crate::{
  FromUrl, FromUrlWithScheme,
  frame::{RgbNchwFrame, RgbNhwcFrame},
};

use image::{ImageReader, RgbImage};
use thiserror::Error;
use tracing::error;
use url::Url;

#[derive(Error, Debug)]
pub enum ImageFileInputError {
  #[error("URI scheme mismatch")]
  SchemeMismatch,
  #[error("I/O error: {0}")]
  IoError(#[from] std::io::Error),
  #[error("Image loading error: {0}")]
  ImageLoadError(#[from] image::ImageError),
}

pub struct ImageFileInput<const W: u32, const H: u32> {
  image: Option<RgbImage>,
}

impl<const W: u32, const H: u32> FromUrlWithScheme for ImageFileInput<W, H> {
  const SCHEME: &'static str = "image";
}

impl<const W: u32, const H: u32> FromUrl for ImageFileInput<W, H> {
  type Error = ImageFileInputError;

  fn from_url(url: &Url) -> Result<Self, Self::Error> {
    if url.scheme() != Self::SCHEME {
      error!(
        "URI scheme mismatch: expected '{}', found '{}'",
        Self::SCHEME,
        url.scheme()
      );
      return Err(ImageFileInputError::SchemeMismatch);
    }

    let path = url.path();
    let image = ImageReader::open(path)?.decode()?;

    Ok(ImageFileInput {
      image: Some(image.into()),
    })
  }
}

impl<const W: u32, const H: u32> ImageFileInput<W, H> {
  pub fn into_nchw(self) -> ImageFileInputNchw<W, H> {
    ImageFileInputNchw { inner: self }
  }

  pub fn into_nhwc(self) -> ImageFileInputNhwc<W, H> {
    ImageFileInputNhwc { inner: self }
  }
}

pub struct ImageFileInputNchw<const W: u32, const H: u32> {
  inner: ImageFileInput<W, H>,
}

impl<const W: u32, const H: u32> Iterator for ImageFileInputNchw<W, H> {
  type Item = RgbNchwFrame<W, H>;

  fn next(&mut self) -> Option<Self::Item> {
    self.inner.image.take().map(RgbNchwFrame::from)
  }
}

impl<const W: u32, const H: u32> From<RgbImage> for RgbNchwFrame<W, H> {
  fn from(image: RgbImage) -> Self {
    let (mut frame, image) = {
      let image = image::imageops::resize(&image, W, H, image::imageops::FilterType::Nearest);
      (RgbNchwFrame::<W, H>::default(), image)
    };

    let channels = frame.channels() as u32;
    let height = frame.height() as u32;
    let width = frame.width() as u32;
    let slice = frame.as_mut();

    for c in 0..channels {
      for h in 0..height {
        for w in 0..width {
          let pixel = image.get_pixel(w, h);
          let value = pixel[c as usize];
          let index = (c as usize) * (height as usize) * (width as usize)
            + (h as usize) * (width as usize)
            + (w as usize);
          slice[index] = value;
        }
      }
    }
    frame
  }
}

pub struct ImageFileInputNhwc<const W: u32, const H: u32> {
  inner: ImageFileInput<W, H>,
}

impl<const W: u32, const H: u32> Iterator for ImageFileInputNhwc<W, H> {
  type Item = RgbNhwcFrame<W, H>;

  fn next(&mut self) -> Option<Self::Item> {
    self.inner.image.take().map(RgbNhwcFrame::from)
  }
}

impl<const W: u32, const H: u32> From<RgbImage> for RgbNhwcFrame<W, H> {
  fn from(image: RgbImage) -> Self {
    let (mut frame, image) = {
      let image = image::imageops::resize(&image, W, H, image::imageops::FilterType::Nearest);
      (RgbNhwcFrame::<W, H>::default(), image)
    };

    let channels = frame.channels() as u32;
    let height = frame.height() as u32;
    let width = frame.width() as u32;
    let slice = frame.as_mut();

    for h in 0..height {
      for w in 0..width {
        for c in 0..channels {
          let pixel = image.get_pixel(w, h);
          let value = pixel[c as usize];
          let index = (h as usize) * (width as usize) * (channels as usize)
            + (w as usize) * (channels as usize)
            + (c as usize);
          slice[index] = value;
        }
      }
    }
    frame
  }
}
