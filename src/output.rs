// 该文件是 Shanan （山南西风） 项目的一部分。
// src/output.rs - 输出定义
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

use crate::FromUrl;
#[cfg(feature = "save_image_file")]
use crate::FromUrlWithScheme;
use crate::frame::{RgbNchwFrame, RgbNhwcFrame};
use crate::model::{DetectResult, WithLabel};
use thiserror::Error;
use url::Url;

pub trait Render<Frame, Output>: Sized {
  type Error;
  fn render_result(&self, frame: &Frame, result: &Output) -> Result<(), Self::Error>;
}

pub mod draw;

#[cfg(feature = "save_image_file")]
mod save_image_file;
#[cfg(feature = "save_image_file")]
pub use self::save_image_file::{SaveImageFileError, SaveImageFileOutput};

#[cfg(feature = "gstreamer_output")]
mod gstreamer_video_output;
#[cfg(feature = "gstreamer_output")]
pub use self::gstreamer_video_output::{GStreamerVideoOutput, GStreamerVideoOutputError};

#[cfg(feature = "gstreamer_output")]
mod gstreamer_rtsp_output;
#[cfg(feature = "gstreamer_output")]
pub use self::gstreamer_rtsp_output::{GStreamerRtspOutput, GStreamerRtspOutputError};

#[cfg(feature = "directory_record")]
mod directory_record;
#[cfg(feature = "directory_record")]
pub use self::directory_record::{DirectoryRecordOutput, DirectoryRecordOutputError};

#[derive(Error, Debug)]
pub enum OutputError {
  #[cfg(feature = "save_image_file")]
  #[error("保存图像文件错误: {0}")]
  SaveImageFileError(#[from] SaveImageFileError),
  #[cfg(feature = "gstreamer_output")]
  #[error("GStreamer 视频输出错误: {0}")]
  GStreamerVideoOutputError(#[from] GStreamerVideoOutputError),
  #[cfg(feature = "gstreamer_output")]
  #[error("GStreamer RTSP 输出错误: {0}")]
  GStreamerRtspOutputError(#[from] GStreamerRtspOutputError),
  #[cfg(feature = "directory_record")]
  #[error("目录记录输出错误: {0}")]
  DirectoryRecordOutputError(#[from] DirectoryRecordOutputError),
  #[error("URI 方案不匹配")]
  SchemeMismatch,
}

pub enum OutputWrapper<'a, const W: u32, const H: u32> {
  #[cfg(feature = "save_image_file")]
  SaveImageFileOutput(SaveImageFileOutput<'a, W, H>),
  #[cfg(feature = "gstreamer_output")]
  GStreamerVideoOutput(GStreamerVideoOutput<'a, W, H>),
  #[cfg(feature = "gstreamer_output")]
  GStreamerRtspOutput(GStreamerRtspOutput<'a, W, H>),
  #[cfg(feature = "directory_record")]
  DirectoryRecordOutput(DirectoryRecordOutput<'a, W, H>),
}

impl<'a, const W: u32, const H: u32> FromUrl for OutputWrapper<'a, W, H> {
  type Error = OutputError;

  fn from_url(url: &Url) -> Result<Self, Self::Error> {
    match url.scheme() {
      #[cfg(feature = "save_image_file")]
      SaveImageFileOutput::<'a, W, H>::SCHEME => {
        let output = SaveImageFileOutput::from_url(url)?;
        Ok(OutputWrapper::SaveImageFileOutput(output))
      }
      #[cfg(feature = "gstreamer_output")]
      GStreamerVideoOutput::<'a, W, H>::SCHEME => {
        let output = GStreamerVideoOutput::from_url(url)?;
        Ok(OutputWrapper::GStreamerVideoOutput(output))
      }
      #[cfg(feature = "gstreamer_output")]
      GStreamerRtspOutput::<'a, W, H>::SCHEME => {
        let output = GStreamerRtspOutput::from_url(url)?;
        Ok(OutputWrapper::GStreamerRtspOutput(output))
      }
      #[cfg(feature = "directory_record")]
      DirectoryRecordOutput::<'a, W, H>::SCHEME => {
        let output = DirectoryRecordOutput::from_url(url)?;
        Ok(OutputWrapper::DirectoryRecordOutput(output))
      }
      _ => Err(OutputError::SchemeMismatch),
    }
  }
}

impl<'a, const W: u32, const H: u32, T: WithLabel> Render<RgbNchwFrame<W, H>, DetectResult<T>>
  for OutputWrapper<'a, W, H>
{
  type Error = OutputError;

  fn render_result(
    &self,
    frame: &RgbNchwFrame<W, H>,
    result: &DetectResult<T>,
  ) -> Result<(), Self::Error> {
    match self {
      #[cfg(feature = "save_image_file")]
      OutputWrapper::SaveImageFileOutput(output) => output
        .render_result(frame, result)
        .map_err(OutputError::from),
      #[cfg(feature = "gstreamer_output")]
      OutputWrapper::GStreamerVideoOutput(output) => output
        .render_result(frame, result)
        .map_err(OutputError::from),
      #[cfg(feature = "gstreamer_output")]
      OutputWrapper::GStreamerRtspOutput(output) => output
        .render_result(frame, result)
        .map_err(OutputError::from),
      #[cfg(feature = "directory_record")]
      OutputWrapper::DirectoryRecordOutput(output) => output
        .render_result(frame, result)
        .map_err(OutputError::from),
    }
  }
}

impl<'a, const W: u32, const H: u32, T: WithLabel> Render<RgbNhwcFrame<W, H>, DetectResult<T>>
  for OutputWrapper<'a, W, H>
{
  type Error = OutputError;

  fn render_result(
    &self,
    frame: &RgbNhwcFrame<W, H>,
    result: &DetectResult<T>,
  ) -> Result<(), Self::Error> {
    match self {
      #[cfg(feature = "save_image_file")]
      OutputWrapper::SaveImageFileOutput(output) => output
        .render_result(frame, result)
        .map_err(OutputError::from),
      #[cfg(feature = "gstreamer_output")]
      OutputWrapper::GStreamerVideoOutput(output) => output
        .render_result(frame, result)
        .map_err(OutputError::from),
      #[cfg(feature = "gstreamer_output")]
      OutputWrapper::GStreamerRtspOutput(output) => output
        .render_result(frame, result)
        .map_err(OutputError::from),
      #[cfg(feature = "directory_record")]
      OutputWrapper::DirectoryRecordOutput(output) => output
        .render_result(frame, result)
        .map_err(OutputError::from),
    }
  }
}
