// 该文件是 Shanan （山南西风） 项目的一部分。
// src/input.rs - 视频/图像输入
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
