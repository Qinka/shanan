// 该文件是 Shanan （山南西风） 项目的一部分。
// src/input.rs - 视频/图像输入
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use thiserror::Error;

pub trait AsNchwFrame<const W: u32, const H: u32> {
  fn as_nchw(&self) -> &[u8];
}

pub trait AsNhwcFrame<const W: u32, const H: u32> {
  fn as_nhwc(&self) -> &[u8];
}

#[cfg(feature = "read_image_file")]
mod read_image_file;
use crate::{
  FromUrl,
  frame::{RgbNchwFrame, RgbNhwcFrame},
};

#[cfg(feature = "read_image_file")]
pub use self::read_image_file::{ImageFileInput, ImageFileInputError};

#[cfg(feature = "gstreamer_input")]
mod gstreamer_input;
#[cfg(feature = "gstreamer_input")]
pub use self::gstreamer_input::{
  GStreamerInput, GStreamerInputError, GStreamerInputPipelineBuilder,
};

#[derive(Error, Debug)]
pub enum InputError {
  #[cfg(feature = "read_image_file")]
  #[error("Image file input error: {0}")]
  ImageFileInputError(#[from] ImageFileInputError),
  #[cfg(feature = "gstreamer_input")]
  #[error("GStreamer input error: {0}")]
  GStreamerInputError(#[from] GStreamerInputError),
  #[error("URI scheme mismatch")]
  SchemeMismatch,
}

pub enum InputWrapper<const W: u32, const H: u32> {
  #[cfg(feature = "gstreamer_input")]
  GStreamerInput(GStreamerInput<W, H>),
  #[cfg(feature = "read_image_file")]
  ReadImageFile(ImageFileInput<W, H>),
}

impl<const W: u32, const H: u32> FromUrl for InputWrapper<W, H> {
  type Error = InputError;

  fn from_url(url: &url::Url) -> Result<Self, Self::Error> {
    #[cfg(feature = "gstreamer_input")]
    {
      use crate::FromUrlWithScheme;

      if url.scheme() == GStreamerInputPipelineBuilder::<W, H>::SCHEME {
        let input = GStreamerInputPipelineBuilder::from_url(url)?.build()?;
        return Ok(InputWrapper::GStreamerInput(input));
      }
    }
    #[cfg(feature = "read_image_file")]
    {
      use crate::FromUrlWithScheme;

      if url.scheme() == ImageFileInput::<W, H>::SCHEME {
        let input = ImageFileInput::from_url(url)?;
        return Ok(InputWrapper::ReadImageFile(input));
      }
    }
    Err(InputError::SchemeMismatch)
  }
}

impl<const W: u32, const H: u32> InputWrapper<W, H> {
  pub fn into_nhwc(self) -> InputWrapperNhwcIter<W, H> {
    match self {
      #[cfg(feature = "gstreamer_input")]
      InputWrapper::GStreamerInput(input) => {
        InputWrapperNhwcIter::GStreamerInput(input.into_nhwc())
      }
      #[cfg(feature = "read_image_file")]
      InputWrapper::ReadImageFile(input) => InputWrapperNhwcIter::ReadImageFile(input.into_nhwc()),
    }
  }

  pub fn into_nchw(self) -> InputWrapperNchwIter<W, H> {
    match self {
      #[cfg(feature = "gstreamer_input")]
      InputWrapper::GStreamerInput(input) => {
        InputWrapperNchwIter::GStreamerInput(input.into_nchw())
      }
      #[cfg(feature = "read_image_file")]
      InputWrapper::ReadImageFile(input) => InputWrapperNchwIter::ReadImageFile(input.into_nchw()),
    }
  }
}

pub enum InputWrapperNhwcIter<const W: u32, const H: u32> {
  #[cfg(feature = "gstreamer_input")]
  GStreamerInput(self::gstreamer_input::GStreamerInputNhwc<W, H>),
  #[cfg(feature = "read_image_file")]
  ReadImageFile(self::read_image_file::ImageFileInputNhwc<W, H>),
}

impl<const W: u32, const H: u32> Iterator for InputWrapperNhwcIter<W, H> {
  type Item = RgbNhwcFrame<W, H>;

  fn next(&mut self) -> Option<Self::Item> {
    match self {
      #[cfg(feature = "gstreamer_input")]
      InputWrapperNhwcIter::GStreamerInput(input) => input.next(),
      #[cfg(feature = "read_image_file")]
      InputWrapperNhwcIter::ReadImageFile(input) => input.next(),
    }
  }
}

pub enum InputWrapperNchwIter<const W: u32, const H: u32> {
  #[cfg(feature = "gstreamer_input")]
  GStreamerInput(self::gstreamer_input::GStreamerInputNchw<W, H>),
  #[cfg(feature = "read_image_file")]
  ReadImageFile(self::read_image_file::ImageFileInputNchw<W, H>),
}

impl<const W: u32, const H: u32> Iterator for InputWrapperNchwIter<W, H> {
  type Item = RgbNchwFrame<W, H>;

  fn next(&mut self) -> Option<Self::Item> {
    match self {
      #[cfg(feature = "gstreamer_input")]
      InputWrapperNchwIter::GStreamerInput(input) => input.next(),
      #[cfg(feature = "read_image_file")]
      InputWrapperNchwIter::ReadImageFile(input) => input.next(),
    }
  }
}
