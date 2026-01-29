// 该文件是 Shanan （山南西风） 项目的一部分。
// src/bin/simple_camera.rs - 简单的图像推理代码
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use std::{thread::sleep, time::Duration};

use anyhow::Result;
use clap::Parser;
use url::Url;

use shanan::{
  FromUrl,
  model::{CocoLabel, Model},
  output::Render,
};
use tracing::{info, warn};

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

  #[arg(long, value_name = "FRAME_NUMBER", default_value_t = 0)]
  pub frame_number: usize,
}

fn main() -> Result<()> {
  tracing_subscriber::fmt::init();

  let args = Args::parse();

  info!("模型文件路径: {}", args.model);
  info!("输入来源: {}", args.input);
  info!("输出路径: {}", args.output);

  let input_image =
    shanan::input::GStreamerInputPipelineBuilder::<640, 640>::from_url(&args.input)?.build()?;
  let model = shanan::model::Yolo26Builder::from_url(&args.model)?.build()?;
  let output = shanan::output::GStreamerVideoOutput::from_url(&args.output)?;

  info!("开始处理图像流...");
  for (index, frame) in input_image.into_nhwc().enumerate() {
    info!("处理第 {} 帧图像", index + 1);

    if (args.frame_number > 0) && (index >= args.frame_number) {
      warn!("已达到指定帧数 {}, 停止推理", args.frame_number);
      break;
    }

    info!("开始推理...");
    let now = std::time::Instant::now();
    let result: shanan::model::DetectResult<CocoLabel> = model.infer(&frame)?;
    let elapsed = now.elapsed();
    info!("推理完成，耗时: {:.2?}", elapsed);
    output.render_result(&frame, &result)?;
    sleep(Duration::from_millis(100));
  }

  Ok(())
}
