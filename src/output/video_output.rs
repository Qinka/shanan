// 该文件是 Shanan （山南西风） 项目的一部分。
// src/output/video_output.rs - 视频输出
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use anyhow::{Context, Result};
use ffmpeg_next as ffmpeg;
use ffmpeg_next::format::{Pixel, output};
use ffmpeg_next::software::scaling::{context::Context as ScalingContext, flag::Flags};
use ffmpeg_next::util::frame::video::Video;
use ffmpeg_next::{Rational, codec};
use image::RgbImage;

use super::{OutputWriter, Visualizer};
use crate::detector::Detection;

/// 视频输出
pub struct VideoOutput {
  /// FFmpeg 输出上下文
  output_context: ffmpeg::format::context::Output,
  /// 视频编码器
  encoder: ffmpeg::encoder::Video,
  /// 缩放上下文（RGB -> YUV）
  scaler: ScalingContext,
  /// 视频宽度
  width: u32,
  /// 视频高度
  height: u32,
  /// 帧率
  fps: f64,
  /// 帧索引
  frame_index: u64,
  /// 可视化工具
  visualizer: Visualizer,
  /// 视频流索引
  stream_index: usize,
  /// 时间基准
  time_base: Rational,
}

impl VideoOutput {
  /// 创建一个新的视频输出
  pub fn new(output_path: &str, width: u32, height: u32, fps: f64) -> Result<Self> {
    ffmpeg::init().context("无法初始化 FFmpeg")?;

    let mut output_context =
      output(&output_path).with_context(|| format!("无法创建输出文件: {}", output_path))?;

    // 查找编码器
    let codec = ffmpeg::encoder::find(codec::Id::H264)
      .or_else(|| ffmpeg::encoder::find(codec::Id::MPEG4))
      .context("找不到视频编码器")?;

    let mut stream = output_context.add_stream(codec)?;
    let stream_index = stream.index();

    let context_encoder = ffmpeg::codec::context::Context::new_with_codec(codec);
    let mut encoder = context_encoder.encoder().video()?;

    encoder.set_width(width);
    encoder.set_height(height);
    encoder.set_format(Pixel::YUV420P);
    encoder.set_frame_rate(Some(Rational::new(fps as i32, 1)));
    encoder.set_time_base(Rational::new(1, fps as i32));

    let encoder = encoder.open()?;
    stream.set_parameters(&encoder);

    let time_base = stream.time_base();

    // 写入文件头
    output_context.write_header()?;

    // 创建缩放上下文（RGB24 -> YUV420P）
    let scaler = ScalingContext::get(
      Pixel::RGB24,
      width,
      height,
      Pixel::YUV420P,
      width,
      height,
      Flags::BILINEAR,
    )?;

    Ok(Self {
      output_context,
      encoder,
      scaler,
      width,
      height,
      fps,
      frame_index: 0,
      visualizer: Visualizer::new(),
      stream_index,
      time_base,
    })
  }

  /// 编码并写入帧
  fn encode_frame(&mut self, frame: Option<&Video>) -> Result<()> {
    if let Some(f) = frame {
      self.encoder.send_frame(f)?;
    } else {
      self.encoder.send_eof()?;
    }

    let mut packet = ffmpeg::Packet::empty();
    while self.encoder.receive_packet(&mut packet).is_ok() {
      packet.set_stream(self.stream_index);
      packet.rescale_ts(Rational::new(1, self.fps as i32), self.time_base);
      packet.write_interleaved(&mut self.output_context)?;
    }

    Ok(())
  }
}

impl OutputWriter for VideoOutput {
  fn write_frame(&mut self, image: &RgbImage, detections: &[Detection]) -> Result<()> {
    // 绘制检测结果
    let mut output_image = image.clone();
    self
      .visualizer
      .draw_detections(&mut output_image, detections);

    // 创建 RGB 帧
    let mut rgb_frame = Video::new(Pixel::RGB24, self.width, self.height);
    let data = output_image.as_raw();
    let stride = rgb_frame.stride(0);
    let width = self.width as usize;
    let height = self.height as usize;

    // 复制数据，处理步长对齐
    let frame_data = rgb_frame.data_mut(0);
    for y in 0..height {
      let src_start = y * width * 3;
      let src_end = src_start + width * 3;
      let dst_start = y * stride;
      frame_data[dst_start..dst_start + width * 3].copy_from_slice(&data[src_start..src_end]);
    }

    // 转换为 YUV
    let mut yuv_frame = Video::empty();
    self.scaler.run(&rgb_frame, &mut yuv_frame)?;

    // 设置 PTS
    yuv_frame.set_pts(Some(self.frame_index as i64));
    self.frame_index += 1;

    // 编码并写入
    self.encode_frame(Some(&yuv_frame))?;

    Ok(())
  }

  fn finish(&mut self) -> Result<()> {
    // 刷新编码器
    self.encode_frame(None)?;

    // 写入文件尾
    self.output_context.write_trailer()?;

    Ok(())
  }
}
