// 该文件是 Shanan （山南西风） 项目的一部分。
// src/args.rs - 项目参数配置
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use clap::Parser;

/// Shanan 项目参数配置
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
  /// RKNN 模型文件路径
  #[arg(long, value_name = "FILE")]
  pub model: String,

  /// 输入来源（图片文件、视频文件或 V4L2 设备路径）
  /// 支持格式:
  /// - 图片: *.jpg, *.jpeg, *.png, *.bmp, *.gif, *.webp
  /// - 视频: *.mp4, *.avi, *.mkv 等
  /// - V4L2: /dev/video0 或 v4l2:///dev/video0
  #[arg(long, value_name = "SOURCE")]
  pub input: String,

  /// 输出文件路径
  /// 支持格式:
  /// - 图片: *.jpg, *.jpeg, *.png, *.bmp
  /// - 视频: *.mp4, *.avi, *.mkv 等
  #[arg(long, value_name = "OUTPUT")]
  pub output: String,

  /// 置信度阈值 (0.0 - 1.0)
  #[arg(long, default_value = "0.5", value_name = "THRESHOLD")]
  pub confidence: f32,

  /// NMS IOU 阈值 (0.0 - 1.0)
  #[arg(long, default_value = "0.45", value_name = "THRESHOLD")]
  pub nms_threshold: f32,

  /// 最大处理帧数（仅对视频/摄像头有效，0 表示无限制）
  #[arg(long, default_value = "0", value_name = "COUNT")]
  pub max_frames: u64,
}
