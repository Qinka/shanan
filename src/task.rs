// 该文件是 Shanan （山南西风） 项目的一部分。
// src/output/gstreamer_video_output.rs - GStreamer 视频文件输出
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

use std::{thread, time::Duration};
use tracing::{info, warn};

use crate::{model::Model, output::Render};

pub trait Task<I, M, O>: Sized {
  type Error;
  fn run_task(self, input: I, model: M, output: O) -> Result<(), Self::Error>;
}

pub struct OneShotTask;

impl<
  F,
  D,
  ME: std::error::Error + Sync + Send + 'static,
  RE: std::error::Error + Sync + Send + 'static,
  I: Iterator<Item = F>,
  M: Model<Input = F, Output = D, Error = ME>,
  O: Render<F, D, Error = RE>,
> Task<I, M, O> for OneShotTask
{
  type Error = anyhow::Error;

  fn run_task(self, mut input: I, model: M, output: O) -> Result<(), Self::Error> {
    info!("开始任务...");
    let frame = input.next().ok_or_else(|| anyhow::anyhow!("没有输入帧"))?;
    info!("输入帧获取成功，开始推理...");
    let now = std::time::Instant::now();
    let result = model.infer(&frame)?;
    let elapsed = now.elapsed();
    info!("推理完成，耗时: {:.2?}", elapsed);
    output.render_result(&frame, &result)?;
    info!("渲染完成，耗时: {:.2?}", elapsed);

    Ok(())
  }
}

pub struct RepeatShotTask;

impl<
  F,
  D,
  ME: std::error::Error + Sync + Send + 'static,
  RE: std::error::Error + Sync + Send + 'static,
  I: Iterator<Item = F>,
  M: Model<Input = F, Output = D, Error = ME>,
  O: Render<F, D, Error = RE>,
> Task<I, M, O> for RepeatShotTask
{
  type Error = anyhow::Error;

  fn run_task(self, mut input: I, model: M, output: O) -> Result<(), Self::Error> {
    const REPEAT_TIMES: usize = 1000;

    info!("开始任务...");
    let frame = input.next().ok_or_else(|| anyhow::anyhow!("没有输入帧"))?;
    info!("输入帧获取成功，开始推理...");
    let mut times = Vec::with_capacity(REPEAT_TIMES);
    for i in 0..REPEAT_TIMES {
      let now = std::time::Instant::now();
      let result = model.infer(&frame)?;
      let elapsed = now.elapsed();
      info!("({})推理完成，耗时: {:.2?}", i, elapsed);
      output.render_result(&frame, &result)?;
      info!("({})渲染完成，耗时: {:.2?}", i, elapsed);
      times.push(elapsed);
    }

    warn!(
      "平均推理时间: {:.2?}",
      times.iter().skip(2).sum::<Duration>() / (times.len() - 2) as u32
    );

    Ok(())
  }
}

#[derive(Default, Debug)]
pub struct ContinuousTask {
  frame_number: Option<usize>,
}

impl ContinuousTask {
  pub fn with_frame_number(mut self, frame_number: Option<usize>) -> Self {
    self.frame_number = frame_number;
    self
  }
}

impl<
  F,
  D,
  ME: std::error::Error + Sync + Send + 'static,
  RE: std::error::Error + Sync + Send + 'static,
  I: Iterator<Item = F>,
  M: Model<Input = F, Output = D, Error = ME>,
  O: Render<F, D, Error = RE>,
> Task<I, M, O> for ContinuousTask
{
  type Error = anyhow::Error;

  fn run_task(self, input: I, model: M, output: O) -> Result<(), Self::Error> {
    info!("开始任务...");
    let (tx, rx) = std::sync::mpsc::channel();

    ctrlc::set_handler(move || {
      info!("收到中断信号，准备退出...");
      let _ = tx.send(());
      thread::spawn(|| {
        thread::sleep(Duration::from_secs(30));
        warn!("强制退出程序");
        std::process::exit(1);
      });
    })
    .expect("Error setting Ctrl-C handler");

    let mut frame_index = 0;
    let mut now = std::time::Instant::now();
    for frame in input {
      frame_index = (frame_index + 1) % usize::MAX;
      info!("处理第 {} 帧图像", frame_index);
      let result = model.infer(&frame)?;
      let elapsed_a = now.elapsed();
      output.render_result(&frame, &result)?;
      let elapsed_b = now.elapsed();
      now = std::time::Instant::now();
      info!("推理完成，耗时: {:.2?} / {:.2?}", elapsed_a, elapsed_b);
      if self.frame_number.map(|n| frame_index >= n).unwrap_or(false) {
        info!("达到指定帧数 {}, 退出任务循环", frame_index);
        break;
      }
      if rx.try_recv().is_ok() {
        warn!("中断信号接收，退出任务循环");
        break;
      }
    }

    info!("任务完成，退出");
    Ok(())
  }
}
