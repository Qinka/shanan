// 该文件是 Shanan （山南西风） 项目的一部分。
// src/model/model.rs - 模型定义
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

pub trait Model {
  type Input;
  type Output;
  type Error;

  fn infer(&self, input: &Self::Input) -> Result<Self::Output, Self::Error>;
  fn postprocess(output: rknpu::Output) -> Self::Output;
}

#[derive(Debug, Clone)]
pub struct DetectItem {
  pub class_id: u32,
  pub score: f32,
  pub bbox: [f32; 4], // [x_min, y_min, x_max, y_max]
}

#[derive(Debug, Clone)]
pub struct DetectResult {
  pub items: Box<[DetectItem]>,
}
