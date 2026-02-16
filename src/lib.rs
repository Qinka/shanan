// 该文件是 Shanan （山南西风） 项目的一部分。
// src/lib.rs - 库主文件
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

pub mod frame;
pub mod input;
pub mod model;
pub mod output;
pub mod task;
pub mod utils;

pub trait FromUrl {
  type Error;
  fn from_url(url: &url::Url) -> Result<Self, Self::Error>
  where
    Self: Sized;
}

pub trait FromUrlWithScheme: FromUrl {
  const SCHEME: &'static str;
}
