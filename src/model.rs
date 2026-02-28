// 该文件是 Shanan （山南西风） 项目的一部分。
// src/model.rs - 模型
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

use shanan_cv::cubecl::Runtime;
use shanan_macro::toml_label;
use shanan_trait::{Model, Postprocess, WithLabel};
use thiserror::Error;
use url::Url;

#[toml_label(file = "labels/coco.toml")]
pub enum CocoLabel {}

#[derive(Debug, Clone, Copy)]
pub struct BBox {
  pub x_min: f32,
  pub y_min: f32,
  pub x_max: f32,
  pub y_max: f32,
}

#[derive(Debug, Clone)]
pub struct DetectItem<T> {
  pub kind: T,
  pub score: f32,
  pub bbox: BBox,
}

#[derive(Debug, Clone)]
pub struct DetectResult<T> {
  pub items: Box<[DetectItem<T>]>,
}

impl<T> DetectResult<T> {
  pub fn is_empty(&self) -> bool {
    self.items.is_empty()
  }
}

use crate::{FromUrl, input::AsNhwcFrame};
#[cfg(feature = "model_yolo26")]
use crate::{FromUrlWithScheme, model::yolo26::Yolo26Postprocess};

#[cfg(feature = "model_yolo26")]
mod yolo26;
#[cfg(feature = "model_yolo26")]
pub use self::yolo26::{Yolo26, Yolo26Builder, Yolo26Nhwc};

pub type DetectionNhwc<const W: u32, const H: u32> =
  Detection<W, H, crate::frame::RgbNhwcFrame<H, W>>;

#[derive(Error, Debug)]
pub enum DetectionError {
  #[cfg(feature = "model_yolo26")]
  #[error("Yolo26 错误: {0}")]
  Yolo26Error(#[from] yolo26::Yolo26Error),
}

pub enum Detection<const W: u32, const H: u32, F> {
  #[cfg(feature = "model_yolo26")]
  Yolo26(Yolo26<W, H, F>),
}
pub enum DetectionOutput {
  RknnOutput(rknpu::Output),
}

impl<const W: u32, const H: u32, F> FromUrl for Detection<W, H, F> {
  type Error = DetectionError;

  fn from_url(url: &Url) -> Result<Self, Self::Error> {
    match url.scheme() {
      #[cfg(feature = "model_yolo26")]
      Yolo26Builder::SCHEME => {
        let model = Yolo26Builder::from_url(url)?.build_model()?;
        Ok(Detection::Yolo26(model))
      }
      _ => Err(DetectionError::Yolo26Error(
        yolo26::Yolo26Error::ModelPathError(format!("Unsupported model scheme: {}", url.scheme())),
      )),
    }
  }
}

impl<const W: u32, const H: u32, Frame: AsNhwcFrame<H, W>> Model for Detection<W, H, Frame> {
  type Input = Frame;
  type Output = DetectionOutput;
  type Error = DetectionError;

  fn infer(&self, input: &Self::Input) -> Result<Self::Output, Self::Error> {
    match self {
      #[cfg(feature = "model_yolo26")]
      Detection::Yolo26(model) => Ok(DetectionOutput::RknnOutput(
        model.infer(input).map_err(DetectionError::from)?,
      )),
    }
  }
}

pub enum DetectionPostprocess<const W: u32, const H: u32, T, R: Runtime> {
  #[cfg(feature = "model_yolo26")]
  Yolo26(Yolo26Postprocess<W, H, T, R>),
}

impl<const W: u32, const H: u32, T: WithLabel, R: Runtime> Postprocess
  for DetectionPostprocess<W, H, T, R>
{
  type Input = DetectionOutput;
  type Output = DetectResult<T>;
  type Error = DetectionError;

  fn process(&self, output: Self::Input) -> Result<Self::Output, Self::Error> {
    match self {
      #[cfg(feature = "model_yolo26")]
      DetectionPostprocess::Yolo26(post) => match output {
        DetectionOutput::RknnOutput(rknn_output) => {
          post.process(rknn_output).map_err(DetectionError::from)
        }
      },
    }
  }
}

impl<const W: u32, const H: u32, T: WithLabel, R: Runtime> FromUrl
  for DetectionPostprocess<W, H, T, R>
{
  type Error = DetectionError;

  fn from_url(url: &Url) -> Result<Self, Self::Error> {
    match url.scheme() {
      #[cfg(feature = "model_yolo26")]
      Yolo26Builder::SCHEME => {
        let post = Yolo26Builder::from_url(url)?.build_postprocess()?;
        Ok(DetectionPostprocess::Yolo26(post))
      }
      _ => Err(DetectionError::Yolo26Error(
        yolo26::Yolo26Error::ModelPathError(format!("Unsupported model scheme: {}", url.scheme())),
      )),
    }
  }
}
