// 该文件是 Shanan （山南西风） 项目的一部分。
// src/output.rs - 输出定义
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP


pub trait Render<Frame, Output>: Sized {
  type Error;
  fn render_result(&self, frame: &Frame, result: &Output) -> Result<(), Self::Error>;
}


// #[cfg(feature = "save_image_file")]
pub mod draw;

mod save_image_file;
pub use self::save_image_file::{SaveImageFileError, SaveImageFileOutput};

// #[cfg(feature = "gstreamer_output")]
mod gstreamer_video_output;
// #[cfg(feature = "gstreamer_output")]
pub use self::gstreamer_video_output::{GStreamerVideoOutput, GStreamerVideoOutputError};

// #[cfg(feature = "gstreamer_output")]
mod gstreamer_rtsp_output;
// #[cfg(feature = "gstreamer_output")]
pub use self::gstreamer_rtsp_output::{GStreamerRtspOutput, GStreamerRtspOutputError};
