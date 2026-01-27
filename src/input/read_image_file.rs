// 该文件是 Shanan （山南西风） 项目的一部分。
// src/input/image_file.rs - 图像文件输入
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use crate::{frame::RgbNchwFrame, input::InputSource};

use image::{DynamicImage, ImageReader};
use thiserror::Error;
use tracing::error;
use url::Url;

pub struct ImageFileInput {
  image: Option<RgbNchwFrame>,
}

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

impl From<DynamicImage> for RgbNchwFrame {
  fn from(image: DynamicImage) -> Self {
    let rgb_image = image.to_rgb8();

    let mut frame = {
      let (width, height) = rgb_image.dimensions();
      RgbNchwFrame::with_shape(height as usize, width as usize)
    };

    let channels = frame.channels() as u32;
    let height = frame.height() as u32;
    let width = frame.width() as u32;
    let slice = frame.as_mut();

    for c in 0..channels {
      for h in 0..height {
        for w in 0..width {
          let pixel = rgb_image.get_pixel(w, h);
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

const READ_IMAGE_FILE_SCHEME: &str = "image";

impl InputSource for ImageFileInput {
  type Error = ImageFileInputError;

  fn from_uri(uri: &Url) -> Result<Self, Self::Error> {
    if uri.scheme() != READ_IMAGE_FILE_SCHEME {
      error!(
        "URI scheme mismatch: expected '{}', found '{}'",
        READ_IMAGE_FILE_SCHEME,
        uri.scheme()
      );
      return Err(ImageFileInputError::SchemaMismatch);
    }

    let path = uri.path();
    let image = ImageReader::open(path)?.decode()?;

    Ok(ImageFileInput {
      image: Some(image.into()),
    })
  }
}

impl Iterator for ImageFileInput {
  type Item = RgbNchwFrame;

  fn next(&mut self) -> Option<Self::Item> {
    self.image.take()
  }
}
