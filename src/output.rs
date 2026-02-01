// 该文件是 Shanan （山南西风） 项目的一部分。
// src/output.rs - 输出定义
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

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
