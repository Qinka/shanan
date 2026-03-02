// 该文件是 Shanan （山南西风） 项目的一部分。
// src/utils/benchmark.rs - 基准测试工具函数
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

use std::{
  ops::{Add, Div},
  time::Duration,
};

#[derive(Debug, Clone)]
pub struct DetectionTimeRecord {
  pub data_load: Duration,
  pub inference: Duration,
  pub postprocess: Duration,
  pub render: Duration,
}

impl Add for DetectionTimeRecord {
  type Output = Self;

  fn add(self, other: Self) -> Self {
    Self {
      data_load: self.data_load + other.data_load,
      inference: self.inference + other.inference,
      postprocess: self.postprocess + other.postprocess,
      render: self.render + other.render,
    }
  }
}

impl Div<u32> for DetectionTimeRecord {
  type Output = Self;

  fn div(self, rhs: u32) -> Self {
    Self {
      data_load: self.data_load / rhs,
      inference: self.inference / rhs,
      postprocess: self.postprocess / rhs,
      render: self.render / rhs,
    }
  }
}

pub struct DetectionBenchmarker {
  start: std::time::SystemTime,
  records: Vec<DetectionTimeRecord>,
  data_load: Option<Duration>,
  inference: Option<Duration>,
  postprocess: Option<Duration>,
  render: Option<Duration>,
}

impl Default for DetectionBenchmarker {
  fn default() -> Self {
    Self {
      start: std::time::SystemTime::now(),
      records: Vec::new(),
      data_load: None,
      inference: None,
      postprocess: None,
      render: None,
    }
  }
}

impl DetectionBenchmarker {
  pub fn step(&mut self) {
    self.start = std::time::SystemTime::now();
    self.data_load = None;
    self.inference = None;
    self.postprocess = None;
    self.render = None;
  }

  pub fn data_load(&mut self) {
    self.data_load = Some(self.start.elapsed().unwrap());
  }

  pub fn inference(&mut self) {
    self.inference = Some(self.start.elapsed().unwrap());
  }

  pub fn postprocess(&mut self) {
    self.postprocess = Some(self.start.elapsed().unwrap());
  }

  pub fn render(&mut self) {
    self.render = Some(self.start.elapsed().unwrap());
  }

  pub fn finish(&mut self) {
    tracing::info!("记录本次检测时间...");
    if let (Some(data_load), Some(inference), Some(postprocess), Some(render)) = (
      self.data_load.take(),
      self.inference.take(),
      self.postprocess.take(),
      self.render.take(),
    ) {
      self.records.push(DetectionTimeRecord {
        data_load,
        inference,
        postprocess,
        render,
      });
    }
  }

  pub fn report(&self) -> DetectionTimeRecord {
    let total_records = self.records.len() as u32;
    let records_average = {
      let avg = self
        .records
        .clone()
        .into_iter()
        .reduce(DetectionTimeRecord::add)
        .unwrap()
        / total_records;
      DetectionTimeRecord {
        data_load: avg.data_load,
        inference: avg.inference - avg.data_load,
        postprocess: avg.postprocess - avg.inference,
        render: avg.render - avg.postprocess,
      }
    };

    println!(
      "Average Data Load Time: {:.2}ms",
      records_average.data_load.as_secs_f64() * 1000.0
    );
    println!(
      "Average Inference Time: {:.2}ms",
      records_average.inference.as_secs_f64() * 1000.0
    );
    println!(
      "Average Postprocess Time: {:.2}ms",
      records_average.postprocess.as_secs_f64() * 1000.0
    );
    println!(
      "Average Render Time: {:.2}ms",
      records_average.render.as_secs_f64() * 1000.0
    );

    records_average
  }
}
