// 该文件是 Shanan （山南西风） 项目的一部分。
// src/output/directory_record.rs - 目录记录输出
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

use chrono::{Datelike, Utc};
use image::RgbImage;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use thiserror::Error;

use crate::output::draw::ToRgbImage;
use crate::{
  FromUrl, FromUrlWithScheme,
  frame::{RgbNchwFrame, RgbNhwcFrame},
  model::{DetectResult, WithLabel},
  output::{
    Render,
    draw::{Draw, DrawDetectionOnFrame, Record},
  },
};

#[derive(Error, Debug)]
pub enum DirectoryRecordOutputError {
  #[error("URI 方案不匹配")]
  SchemeMismatch,
  #[error("图像错误: {0}")]
  ImageError(#[from] image::ImageError),
  #[error("I/O 错误: {0}")]
  IoError(#[from] std::io::Error),
}

pub enum DrawWrapper<'a> {
  Draw(Box<Draw<'a>>),
  Record(Record),
}

impl DrawWrapper<'_> {
  pub fn save_result<F, T>(
    &self,
    path: &PathBuf,
    frame: &F,
    result: &DetectResult<T>,
  ) -> Result<(), DirectoryRecordOutputError>
  where
    F: ToRgbImage,
    T: WithLabel,
  {
    match self {
      DrawWrapper::Draw(draw) => {
        let image: RgbImage = draw.draw_detection(frame, result);
        image.save(path)?;
      }
      DrawWrapper::Record(record) => {
        let image = frame.to_rgb_image();
        image.save(path)?;
        record.record(result, path)?;
      }
    };

    Ok(())
  }
  pub fn with(kind: &str) -> Self {
    match kind {
      "record-name" => DrawWrapper::Record(Record {
        label_with_name: true,
      }),
      "record-id" => DrawWrapper::Record(Record {
        label_with_name: false,
      }),
      _ => DrawWrapper::Draw(Box::new(Draw::default())),
    }
  }
}

pub struct DirectoryRecordOutput<'a, const W: u32, const H: u32> {
  directory: PathBuf,
  draw: DrawWrapper<'a>,
  frame_counters: Arc<Mutex<u16>>,
  always: bool,
}

impl<'a, const W: u32, const H: u32> FromUrlWithScheme for DirectoryRecordOutput<'a, W, H> {
  const SCHEME: &'static str = "folder";
}

impl<'a, const W: u32, const H: u32> FromUrl for DirectoryRecordOutput<'a, W, H> {
  type Error = DirectoryRecordOutputError;

  fn from_url(uri: &url::Url) -> Result<Self, Self::Error> {
    if uri.scheme() != Self::SCHEME {
      return Err(DirectoryRecordOutputError::SchemeMismatch);
    }

    let kind = {
      let mut kind = "draw";
      for (k, v) in uri.query_pairs() {
        if k == "record" {
          if v == "id" {
            kind = "record-id";
          } else {
            kind = "record-name";
          }
          break;
        }
      }
      kind
    };

    let always = uri.query_pairs().any(|(k, _)| k == "always");

    Ok(DirectoryRecordOutput {
      directory: PathBuf::from(uri.path()),
      draw: DrawWrapper::with(kind),
      frame_counters: Arc::new(Mutex::new(0)),
      always,
    })
  }
}

impl<'a, const W: u32, const H: u32> DirectoryRecordOutput<'a, W, H> {
  fn frame_id(&self) -> u16 {
    let mut counter = self.frame_counters.lock().unwrap();
    let id = counter.overflowing_add(1).0;
    *counter = id;
    id
  }

  fn frame_path(&self) -> PathBuf {
    let now = Utc::now();
    let directory = self
      .directory
      .join(now.year().to_string())
      .join(format!("{:02}", now.month()))
      .join(format!("{:02}", now.day()));
    if !directory.exists() {
      std::fs::create_dir_all(&directory).unwrap();
    }

    let filename = directory.join(format!(
      "{}-{:04X}.png",
      now.format("%H-%M-%S"),
      self.frame_id()
    ));

    directory.join(filename)
  }
}

impl<'a, const W: u32, const H: u32, T: WithLabel> Render<RgbNhwcFrame<W, H>, DetectResult<T>>
  for DirectoryRecordOutput<'a, W, H>
{
  type Error = DirectoryRecordOutputError;

  fn render_result(
    &self,
    frame: &RgbNhwcFrame<W, H>,
    result: &DetectResult<T>,
  ) -> Result<(), Self::Error> {
    let path = self.frame_path();
    if self.always || !result.is_empty() {
      self.draw.save_result(&path, frame, result)?;
    }
    Ok(())
  }
}

impl<'a, const W: u32, const H: u32, T: WithLabel> Render<RgbNchwFrame<W, H>, DetectResult<T>>
  for DirectoryRecordOutput<'a, W, H>
{
  type Error = DirectoryRecordOutputError;

  fn render_result(
    &self,
    frame: &RgbNchwFrame<W, H>,
    result: &DetectResult<T>,
  ) -> Result<(), Self::Error> {
    let path = self.frame_path();
    if self.always || !result.is_empty() {
      self.draw.save_result(&path, frame, result)?;
    }
    Ok(())
  }
}
