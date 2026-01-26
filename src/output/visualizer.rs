// 该文件是 Shanan （山南西风） 项目的一部分。
// src/output/visualizer.rs - 可视化模块
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use ab_glyph::{FontArc, PxScale};
use image::{Rgb, RgbImage};
use imageproc::drawing::{draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;

use crate::detector::Detection;

/// 可视化工具
pub struct Visualizer {
  /// 字体
  font: FontArc,
  /// 字体大小
  font_scale: PxScale,
  /// 边界框颜色映射
  colors: Vec<Rgb<u8>>,
}

impl Default for Visualizer {
  fn default() -> Self {
    Self::new()
  }
}

impl Visualizer {
  /// 创建一个新的可视化工具
  pub fn new() -> Self {
    // 使用内置的默认字体数据
    let font_data = include_bytes!("../../assets/DejaVuSans.ttf");
    let font = FontArc::try_from_slice(font_data).expect("无法加载字体");

    // 生成 80 种不同的颜色（对应 COCO 数据集的 80 个类别）
    let colors: Vec<Rgb<u8>> = (0..80)
      .map(|i| {
        let hue = (i as f32 / 80.0) * 360.0;
        Self::hsv_to_rgb(hue, 0.8, 0.9)
      })
      .collect();

    Self {
      font,
      font_scale: PxScale::from(16.0),
      colors,
    }
  }

  /// HSV 转 RGB
  fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Rgb<u8> {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
      (c, x, 0.0)
    } else if h < 120.0 {
      (x, c, 0.0)
    } else if h < 180.0 {
      (0.0, c, x)
    } else if h < 240.0 {
      (0.0, x, c)
    } else if h < 300.0 {
      (x, 0.0, c)
    } else {
      (c, 0.0, x)
    };

    Rgb([
      ((r + m) * 255.0) as u8,
      ((g + m) * 255.0) as u8,
      ((b + m) * 255.0) as u8,
    ])
  }

  /// 在图像上绘制检测结果
  pub fn draw_detections(&self, image: &mut RgbImage, detections: &[Detection]) {
    for detection in detections {
      let color = self.colors[detection.class_id % self.colors.len()];

      // 绘制边界框
      let x = detection.x.max(0.0) as i32;
      let y = detection.y.max(0.0) as i32;
      let width = detection.width.min(image.width() as f32 - detection.x) as u32;
      let height = detection.height.min(image.height() as f32 - detection.y) as u32;

      if width > 0 && height > 0 {
        let rect = Rect::at(x, y).of_size(width, height);
        draw_hollow_rect_mut(image, rect, color);

        // 绘制第二个边框以增加可见度
        if x > 0 && y > 0 {
          let inner_rect =
            Rect::at(x + 1, y + 1).of_size(width.saturating_sub(2), height.saturating_sub(2));
          draw_hollow_rect_mut(image, inner_rect, color);
        }
      }

      // 绘制标签
      let label = format!("{}: {:.2}", detection.class_name, detection.confidence);
      let text_y = (y - 20).max(0);

      draw_text_mut(image, color, x, text_y, self.font_scale, &self.font, &label);
    }
  }
}
