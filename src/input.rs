// 该文件是 Shanan （山南西风） 项目的一部分。
// src/input.rs - 视频/图像输入
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

pub trait AsNchwFrame {
  fn as_nchw(&self) -> &[u8];
}

pub trait AsNhwcFrame {
  fn as_nhwc(&self) -> &[u8];
}

// #[cfg(feature = "read_image_file")]
mod read_image_file;
pub use self::read_image_file::{ImageFileInput, ImageFileInputError};

// #[cfg(feature = "gstreamer_input")]
mod gstreamer_input;
// #[cfg(feature = "gstreamer_input")]
pub use self::gstreamer_input::{
  GStreamerInput, GStreamerInputError, GStreamerInputPipelineBuilder,
};
