// 该文件是 Shanan （山南西风） 项目的一部分。
// src/model.rs - 模型
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use shanan_macro::toml_label;

#[toml_label(file = "labels/coco.toml")]
pub enum CocoLabel {}

pub trait Model {
  type Input;
  type Output;
  type Error;

  fn infer(&self, input: &Self::Input) -> Result<Self::Output, Self::Error>;
  fn postprocess(output: rknpu::Output) -> Self::Output;
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

pub trait WithLabel: Sized + std::fmt::Debug {
  fn to_label_str(&self) -> String;
  fn from_label_id(id: u32) -> Self;
}

mod yolo26;
pub use self::yolo26::{Yolo26, Yolo26Builder};
