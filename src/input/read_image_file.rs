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
use tracing::{error, warn};
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct ReadImageFolderInput<const W: u32, const H: u32> {
  files: Vec<std::path::PathBuf>,
  index: usize,
}

impl<const W: u32, const H: u32> FromUrlWithScheme for ReadImageFolderInput<W, H> {
  const SCHEME: &'static str = "folder";
}

impl<const W: u32, const H: u32> ReadImageFolderInput<W, H> {
  pub fn new(folder: &std::path::Path) -> Self {
    warn!(
      "这个从文件夹读取代码的功能是为了进行性能测试开发的。会将指定的路径下所有的文件当作图片进行读取，同时不会检查和修改图片的尺寸，所以请确保输入路径下的图片尺寸是正确的，否则可能会导致后续处理出问题。同时支持持 NHWC 格式，后续可能会有变化！"
    );
    let mut files = std::fs::read_dir(folder)
      .unwrap()
      .filter_map(|entry| {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_file() { Some(path) } else { None }
      })
      .collect::<Vec<_>>();
    files.sort();
    Self { files, index: 0 }
  }
}

impl<const W: u32, const H: u32> FromUrl for ReadImageFolderInput<W, H> {
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
    let folder = ReadImageFolderInput::new(std::path::Path::new(path));

    Ok(folder)
  }
}

impl<const W: u32, const H: u32> Iterator for ReadImageFolderInput<W, H> {
  type Item = RgbNhwcFrame<W, H>;

  fn next(&mut self) -> Option<Self::Item> {
    if self.index >= self.files.len() {
      return None;
    }
    let path = &self.files[self.index];
    self.index += 1;

    let image = ImageReader::open(path).ok()?.decode().ok()?;
    assert!(
      image.width() == W && image.height() == H,
      "图像尺寸不匹配: expected {}x{}, found {}x{}",
      W,
      H,
      image.width(),
      image.height()
    );

    Some(RgbNhwcFrame::from(image.to_rgb8()))
  }
}
