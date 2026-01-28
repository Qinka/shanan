// 该文件是 Shanan （山南西风） 项目的一部分。
// src/bin/simple.rs - 推理测试代码
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
  model::{CocoLabel, DetectResult, Model},
  output::Render,
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

  let input_image = shanan::input::ImageFileInput::from_url(&args.input)?;
  let model = shanan::model::Yolo26Builder::from_url(&args.model)?.build()?;
  let output = shanan::output::SaveImageFileOutput::from_url(&args.output)?;

  info!("开始推理...");
  let now = std::time::Instant::now();
  for frame in input_image.into_nhwc() {
    let result: DetectResult<CocoLabel> = model.infer(&frame)?;
    let elapsed = now.elapsed();
    info!("推理完成，耗时: {:.2?}", elapsed);
    output.render_result(&frame, &result)?;
  }

  Ok(())
}
