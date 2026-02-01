// 该文件是 Shanan （山南西风） 项目的一部分。
// src/output/gstreamer_video_output.rs - GStreamer 视频文件输出
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

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
