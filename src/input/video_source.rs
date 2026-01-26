// 该文件是 Shanan （山南西风） 项目的一部分。
// src/input/video_source.rs - 视频输入源
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use anyhow::{Context, Result};
use ffmpeg_next as ffmpeg;
use ffmpeg_next::format::{Pixel, input};
use ffmpeg_next::media::Type;
use ffmpeg_next::software::scaling::{context::Context as ScalingContext, flag::Flags};
use ffmpeg_next::util::frame::video::Video;
use image::RgbImage;

use super::{Frame, InputSource, InputSourceType};

/// 视频输入源
pub struct VideoSource {
  /// FFmpeg 输入上下文
  input_context: ffmpeg::format::context::Input,
  /// 视频流索引
  video_stream_index: usize,
  /// 视频解码器
  decoder: ffmpeg::decoder::Video,
  /// 缩放上下文
  scaler: ScalingContext,
  /// 帧索引
  frame_index: u64,
  /// 视频宽度
  width: u32,
  /// 视频高度
  height: u32,
  /// 帧率
  fps: f64,
  /// 时间基准
  time_base: f64,
  /// 是否结束
  finished: bool,
}

impl VideoSource {
  /// 创建一个新的视频输入源
  pub fn new(path: &str) -> Result<Self> {
    ffmpeg::init().context("无法初始化 FFmpeg")?;

    let input_context = input(&path).with_context(|| format!("无法打开视频文件: {}", path))?;

    let video_stream = input_context
      .streams()
      .best(Type::Video)
      .context("找不到视频流")?;

    let video_stream_index = video_stream.index();
    let context_decoder =
      ffmpeg::codec::context::Context::from_parameters(video_stream.parameters())?;
    let decoder = context_decoder.decoder().video()?;

    let width = decoder.width();
    let height = decoder.height();

    let fps = video_stream.avg_frame_rate();
    let fps = fps.numerator() as f64 / fps.denominator() as f64;

    let time_base = video_stream.time_base();
    let time_base = time_base.numerator() as f64 / time_base.denominator() as f64;

    let scaler = ScalingContext::get(
      decoder.format(),
      width,
      height,
      Pixel::RGB24,
      width,
      height,
      Flags::BILINEAR,
    )?;

    Ok(Self {
      input_context,
      video_stream_index,
      decoder,
      scaler,
      frame_index: 0,
      width,
      height,
      fps,
      time_base,
      finished: false,
    })
  }

  /// 解码下一帧
  fn decode_next_frame(&mut self) -> Result<Option<Video>> {
    loop {
      // 首先尝试从解码器获取已解码的帧
      let mut decoded = Video::empty();
      if self.decoder.receive_frame(&mut decoded).is_ok() {
        return Ok(Some(decoded));
      }

      // 读取下一个数据包
      let mut packet_iter = self.input_context.packets();
      loop {
        match packet_iter.next() {
          Some((stream, packet)) => {
            if stream.index() == self.video_stream_index {
              self.decoder.send_packet(&packet)?;
              break;
            }
          }
          None => {
            // 发送 EOF
            self.decoder.send_eof()?;
            // 尝试获取剩余帧
            if self.decoder.receive_frame(&mut decoded).is_ok() {
              return Ok(Some(decoded));
            }
            return Ok(None);
          }
        }
      }
    }
  }
}

impl Iterator for VideoSource {
  type Item = Result<Frame>;

  fn next(&mut self) -> Option<Self::Item> {
    if self.finished {
      return None;
    }

    match self.decode_next_frame() {
      Ok(Some(decoded)) => {
        let mut rgb_frame = Video::empty();
        if let Err(e) = self.scaler.run(&decoded, &mut rgb_frame) {
          return Some(Err(e.into()));
        }

        let data = rgb_frame.data(0);
        let stride = rgb_frame.stride(0);
        let width = self.width as usize;
        let height = self.height as usize;

        // 处理步长对齐的数据
        let mut image_data = Vec::with_capacity(width * height * 3);
        for y in 0..height {
          let row_start = y * stride;
          let row_end = row_start + width * 3;
          image_data.extend_from_slice(&data[row_start..row_end]);
        }

        let image = match RgbImage::from_raw(self.width, self.height, image_data) {
          Some(img) => img,
          None => {
            return Some(Err(anyhow::anyhow!("无法创建 RGB 图像")));
          }
        };

        let timestamp_ms = decoded
          .timestamp()
          .map_or(0, |ts| (ts as f64 * self.time_base * 1000.0) as u64);

        let frame = Frame {
          image,
          index: self.frame_index,
          timestamp_ms,
        };

        self.frame_index += 1;
        Some(Ok(frame))
      }
      Ok(None) => {
        self.finished = true;
        None
      }
      Err(e) => {
        self.finished = true;
        Some(Err(e))
      }
    }
  }
}

impl InputSource for VideoSource {
  fn source_type(&self) -> InputSourceType {
    InputSourceType::Video
  }

  fn width(&self) -> u32 {
    self.width
  }

  fn height(&self) -> u32 {
    self.height
  }

  fn fps(&self) -> Option<f64> {
    Some(self.fps)
  }
}
