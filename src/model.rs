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

use shanan_macro::toml_label;
use thiserror::Error;
use url::Url;

#[toml_label(file = "labels/coco.toml")]
pub enum CocoLabel {}

pub trait Model {
  type Input;
  type Output;
  type Error;

  fn infer(&self, input: &Self::Input) -> Result<Self::Output, Self::Error>;
  fn postprocess(&self, output: rknpu::Output) -> Self::Output;
}

#[derive(Debug, Clone)]
pub struct DetectItem<T> {
  pub kind: T,
  pub score: f32,
  pub bbox: [f32; 4], // [x_min, y_min, x_max, y_max]
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

pub trait WithLabel: Sized + std::fmt::Debug {
  const LABEL_NUM: u32;
  fn to_label_str(&self) -> String;
  fn to_label_id(&self) -> u32;
  fn from_label_id(id: u32) -> Self;
}

#[cfg(feature = "model_yolo26")]
use crate::FromUrlWithScheme;
use crate::{FromUrl, input::AsNhwcFrame};

#[cfg(feature = "model_yolo26")]
mod yolo26;
#[cfg(feature = "model_yolo26")]
pub use self::yolo26::{Yolo26, Yolo26Builder, Yolo26Nhwc};

pub type DetectionNhwc<const W: u32, const H: u32, T> =
  Detection<W, H, crate::frame::RgbNhwcFrame<H, W>, T>;

#[derive(Error, Debug)]
pub enum DetectionError {
  #[cfg(feature = "model_yolo26")]
  #[error("Yolo26 错误: {0}")]
  Yolo26Error(#[from] yolo26::Yolo26Error),
}

pub enum Detection<const W: u32, const H: u32, F, T> {
  #[cfg(feature = "model_yolo26")]
  Yolo26(Yolo26<W, H, F, T>),
}

impl<const W: u32, const H: u32, F, T> FromUrl for Detection<W, H, F, T> {
  type Error = DetectionError;

  fn from_url(url: &Url) -> Result<Self, Self::Error> {
    match url.scheme() {
      #[cfg(feature = "model_yolo26")]
      Yolo26Builder::SCHEME => {
        let model = Yolo26Builder::from_url(url)?.build()?;
        Ok(Detection::Yolo26(model))
      }
      _ => Err(DetectionError::Yolo26Error(
        yolo26::Yolo26Error::ModelPathError(format!("Unsupported model scheme: {}", url.scheme())),
      )),
    }
  }
}

impl<const W: u32, const H: u32, Frame: AsNhwcFrame<H, W>, T: WithLabel> Model
  for Detection<W, H, Frame, T>
{
  type Input = Frame;
  type Output = DetectResult<T>;
  type Error = DetectionError;

  fn infer(&self, input: &Self::Input) -> Result<Self::Output, Self::Error> {
    match self {
      #[cfg(feature = "model_yolo26")]
      Detection::Yolo26(model) => model.infer(input).map_err(DetectionError::from),
    }
  }

  fn postprocess(&self, output: rknpu::Output) -> Self::Output {
    match self {
      #[cfg(feature = "model_yolo26")]
      Detection::Yolo26(model) => model.postprocess(output),
    }
  }
}
