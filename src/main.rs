// 该文件是 Shanan （山南西风） 项目的一部分。
// src/main.rs - 项目主程序
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

mod args;
mod detector;
mod input;
mod output;

use anyhow::Result;
use clap::Parser;

use detector::YoloDetector;
use input::create_input_source;
use output::create_output_writer;

fn main() -> Result<()> {
  let args = args::Args::parse();

  println!("Shanan 视觉推理框架");
  println!("==================");
  println!("模型文件路径: {}", args.model);
  println!("输入来源: {}", args.input);
  println!("输出文件: {}", args.output);
  println!("置信度阈值: {}", args.confidence);
  println!("NMS 阈值: {}", args.nms_threshold);
  println!();

  // 创建 YOLO 检测器
  println!("正在加载模型...");
  let detector = YoloDetector::new(&args.model, args.confidence, args.nms_threshold)?;
  println!("模型加载完成");

  // 创建输入源
  println!("正在打开输入源...");
  let mut input_source = create_input_source(&args.input)?;
  println!(
    "输入源已打开: {}x{} {}",
    input_source.width(),
    input_source.height(),
    match input_source.source_type() {
      input::InputSourceType::Image => "图片",
      input::InputSourceType::Video => "视频",
      input::InputSourceType::V4l2 => "V4L2 摄像头",
    }
  );

  // 创建输出写入器
  println!("正在创建输出...");
  let mut output_writer = create_output_writer(
    &args.output,
    input_source.width(),
    input_source.height(),
    input_source.fps(),
  )?;
  println!("输出已创建");

  // 处理帧
  println!();
  println!("开始处理...");
  let mut frame_count = 0u64;
  let mut total_detections = 0usize;

  while let Some(frame_result) = input_source.next() {
    let frame = frame_result?;

    // 检查是否达到最大帧数
    if args.max_frames > 0 && frame_count >= args.max_frames {
      println!("已达到最大帧数限制: {}", args.max_frames);
      break;
    }

    // 运行检测
    let detections = detector.detect(&frame.image)?;
    total_detections += detections.len();

    // 输出检测结果
    if !detections.is_empty() {
      println!(
        "帧 {} (时间: {}ms): 检测到 {} 个对象",
        frame.index,
        frame.timestamp_ms,
        detections.len()
      );
      for det in &detections {
        println!(
          "  - {}: {:.2}% at ({:.0}, {:.0}, {:.0}x{:.0})",
          det.class_name,
          det.confidence * 100.0,
          det.x,
          det.y,
          det.width,
          det.height
        );
      }
    }

    // 写入输出
    output_writer.write_frame(&frame.image, &detections)?;
    frame_count += 1;
  }

  // 完成输出
  output_writer.finish()?;

  println!();
  println!("处理完成!");
  println!("总帧数: {}", frame_count);
  println!("总检测数: {}", total_detections);
  println!("输出文件: {}", args.output);

  Ok(())
}
