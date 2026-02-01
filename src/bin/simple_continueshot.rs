// 该文件是 Shanan （山南西风） 项目的一部分。
// src/bin/simple_camera.rs - 简单的图像推理代码
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use anyhow::Result;
use clap::Parser;
use url::Url;

use shanan::{
  FromUrl,
  model::{CocoLabel, DetectionNhwc},
  task::{ContinuousTask, Task},
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

  #[arg(long, value_name = "FRAME_NUMBER")]
  pub frame_number: Option<usize>,
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

  ContinuousTask::default()
    .with_frame_number(args.frame_number)
    .run_task(input_image.into_nhwc(), model, output)?;

  Ok(())
}
