// 该文件是 Shanan （山南西风） 项目的一部分。
// src/bin/benchmark_repeatshot.rs - 用于测试代码的基准测试任务
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
  task::BenchmarkTask,
};
use shanan_trait::Task;
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
  /// 基准测试重复次数
  #[arg(long, value_name = "TIMES", default_value_t = 1000)]
  pub times: u32,
}

#[cfg(not(feature = "cubecl-wgpu"))]
type Runtime = shanan_cv::cubecl::cpu::CpuRuntime;
#[cfg(feature = "cubecl-wgpu")]
type Runtime = shanan_cv::cubecl::wgpu::WgpuRuntime;

fn main() -> Result<()> {
  tracing_subscriber::fmt::init();

  let args = Args::parse();

  info!("模型文件路径: {}", args.model);
  info!("输入来源: {}", args.input);
  info!("输出路径: {}", args.output);

  let input_image = shanan::input::InputWrapper::from_url(&args.input)?;
  let model: DetectionNhwc<640, 640> = shanan::model::Detection::from_url(&args.model)?;
  let postprocess: shanan::model::DetectionPostprocess<640, 640, CocoLabel, Runtime> =
    shanan::model::DetectionPostprocess::from_url(&args.model)?;
  let output = shanan::output::OutputWrapper::from_url(&args.output)?;

  BenchmarkTask::default().with_times(args.times).run_task(
    input_image.into_nhwc(),
    model,
    postprocess,
    output,
  )?;

  Ok(())
}
