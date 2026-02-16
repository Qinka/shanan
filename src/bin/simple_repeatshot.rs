// 该文件是 Shanan （山南西风） 项目的一部分。
// src/bin/simple_image.rs - 简单的图像推理代码
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

use anyhow::Result;
use clap::Parser;
use url::Url;

use shanan::{
  FromUrl,
  model::{CocoLabel, DetectionNhwc},
  task::{RepeatShotTask, Task},
};
use tracing::info;

/// Shanan 项目参数配置
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
  /// RKNN 模型文件路径
  #[arg(long, value_name = "MODEL")]
  pub model: Url,
  /// 输入来源
  #[arg(long, value_name = "SOURCE")]
  pub input: Url,
  /// 输出路径
  #[arg(long, value_name = "OUTPUT")]
  pub output: Url,
}

fn main() -> Result<()> {
  tracing_subscriber::fmt::init();

  let args = Args::parse();

  info!("模型文件路径: {}", args.model);
  info!("输入来源: {}", args.input);
  info!("输出路径: {}", args.output);

  let input_image = shanan::input::InputWrapper::from_url(&args.input)?;
  let model: DetectionNhwc<640, 640, CocoLabel> = shanan::model::Detection::from_url(&args.model)?;
  let output = shanan::output::OutputWrapper::from_url(&args.output)?;

  RepeatShotTask.run_task(input_image.into_nhwc(), model, output)?;

  Ok(())
}
