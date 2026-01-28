// 该文件是 Shanan （山南西风） 项目的一部分。
// src/lib.rs - 库主文件
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

pub mod frame;
pub mod input;
pub mod model;
pub mod output;

pub trait FromUrl {
  type Error;
  fn from_url(url: &url::Url) -> Result<Self, Self::Error>
  where
    Self: Sized;
}
