// 该文件是 Shanan （山南西风） 项目的一部分。
// src/input/image_file.rs - 图像文件输入
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use crate::{
  FromUrl,
  frame::{RgbNchwFrame, RgbNhwcFrame},
};

use image::{ImageReader, RgbImage};
use thiserror::Error;
use tracing::error;
use url::Url;

#[derive(Error, Debug)]
pub enum ImageFileInputError {
  #[error("URI schema mismatch")]
  SchemaMismatch,
  #[error("I/O error: {0}")]
  IoError(std::io::Error),
  #[error("Image loading error: {0}")]
  ImageLoadError(image::ImageError),
}

impl From<std::io::Error> for ImageFileInputError {
  fn from(err: std::io::Error) -> Self {
    ImageFileInputError::IoError(err)
  }
}

impl From<image::ImageError> for ImageFileInputError {
  fn from(err: image::ImageError) -> Self {
    ImageFileInputError::ImageLoadError(err)
  }
}

const READ_IMAGE_FILE_SCHEME: &str = "image";

pub struct ImageFileInput<const W: u32, const H: u32> {
  image: Option<RgbImage>,
}

impl<const W: u32, const H: u32> FromUrl for ImageFileInput<W, H> {
  type Error = ImageFileInputError;

  fn from_url(url: &Url) -> Result<Self, Self::Error> {
    if url.scheme() != READ_IMAGE_FILE_SCHEME {
      error!(
        "URI scheme mismatch: expected '{}', found '{}'",
        READ_IMAGE_FILE_SCHEME,
        url.scheme()
      );
      return Err(ImageFileInputError::SchemaMismatch);
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
