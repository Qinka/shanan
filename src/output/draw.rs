// 该文件是 Shanan （山南西风） 项目的一部分。
// src/output/draw.rs - 目标检测结果可视化
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use ab_glyph::{FontRef, PxScale};
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut};

use crate::{
  frame::{RgbNchwFrame, RgbNhwcFrame},
  input::{AsNchwFrame, AsNhwcFrame},
  model::{DetectItem, DetectResult, WithLabel},
};

// 文本渲染常量
const LABEL_FONT_SIZE: f32 = 20.0;
const LABEL_TEXT_HEIGHT: i32 = 24;
const LABEL_CHAR_WIDTH: f32 = 11.0; // 每字符平均宽度（粗略估计）
const LABEL_TEXT_VERTICAL_PADDING: i32 = 2;

// 在图像上绘制一个矩形边框，bbox 为归一化坐标 [x_min, y_min, x_max, y_max]
fn draw_bbox_with_label<T: WithLabel>(
  image: &mut RgbImage,
  bbox: &[f32; 4],
  kind: &T,
  score: f32,
  color: [u8; 3],
  font: &FontRef,
) {
  let (w, h) = (image.width() as f32, image.height() as f32);

  let mut x_min = (bbox[0] * w).floor() as i32;
  let mut y_min = (bbox[1] * h).floor() as i32;
  let mut x_max = (bbox[2] * w).ceil() as i32;
  let mut y_max = (bbox[3] * h).ceil() as i32;

  // Clamp to image bounds
  x_min = x_min.clamp(0, w as i32 - 1);
  y_min = y_min.clamp(0, h as i32 - 1);
  x_max = x_max.clamp(0, w as i32 - 1);
  y_max = y_max.clamp(0, h as i32 - 1);

  if x_min >= x_max || y_min >= y_max {
    return;
  }

  // 绘制边框（加粗为2像素）
  for thickness in 0..2 {
    let x_min_t = (x_min + thickness).min(w as i32 - 1);
    let y_min_t = (y_min + thickness).min(h as i32 - 1);
    let x_max_t = (x_max - thickness).max(0);
    let y_max_t = (y_max - thickness).max(0);

    // Top and bottom edges
    for x in x_min_t..=x_max_t {
      if y_min_t >= 0 && (y_min_t as u32) < image.height() && (x as u32) < image.width() {
        let top = image.get_pixel_mut(x as u32, y_min_t as u32);
        *top = Rgb(color);
      }
      if y_max_t >= 0 && (y_max_t as u32) < image.height() && (x as u32) < image.width() {
        let bottom = image.get_pixel_mut(x as u32, y_max_t as u32);
        *bottom = Rgb(color);
      }
    }

    // Left and right edges
    for y in y_min_t..=y_max_t {
      if x_min_t >= 0 && (x_min_t as u32) < image.width() && (y as u32) < image.height() {
        let left = image.get_pixel_mut(x_min_t as u32, y as u32);
        *left = Rgb(color);
      }
      if x_max_t >= 0 && (x_max_t as u32) < image.width() && (y as u32) < image.height() {
        let right = image.get_pixel_mut(x_max_t as u32, y as u32);
        *right = Rgb(color);
      }
    }
  }

  // 创建标签文本
  let label = format!("{} {:.2}", kind.to_label_str(), score);

  // 文本参数
  let scale = PxScale::from(LABEL_FONT_SIZE);
  let text_color = Rgb([255u8, 255u8, 255u8]); // 白色文本

  // 估算文本大小（粗略估计）
  let text_width = (label.len() as f32 * LABEL_CHAR_WIDTH) as i32;
  let text_height = LABEL_TEXT_HEIGHT;

  // 确定标签背景位置（在边框上方）
  let label_x = x_min.max(0);
  let label_y = (y_min - text_height).max(0);

  // 确保标签不超出图像边界
  let max_width = (w as i32 - label_x).max(0);
  let label_width = text_width.min(max_width) as u32;
  let label_height = text_height as u32;

  // 仅在标签有空间时绘制
  if label_width > 0 && label_height > 0 {
    // 绘制标签背景
    let rect = imageproc::rect::Rect::at(label_x, label_y).of_size(label_width, label_height);
    draw_filled_rect_mut(image, rect, Rgb(color));

    // 绘制文本
    draw_text_mut(
      image,
      text_color,
      label_x,
      label_y + LABEL_TEXT_VERTICAL_PADDING,
      scale,
      font,
      &label,
    );
  }
}

/// 在 RgbImage 上绘制目标检测结果
pub fn draw_detections<T: WithLabel>(image: &mut RgbImage, result: &DetectResult<T>) {
  // 加载嵌入的字体（使用 lazy_static 或每次加载）
  // 注意：为了保持最小改动，这里保持与原代码相同的方式
  // 在生产环境中，建议使用 lazy_static 或 once_cell 来缓存字体
  let font_data = include_bytes!("../../assets/font.ttf");
  let font = FontRef::try_from_slice(font_data).expect("无法加载嵌入的字体文件");

  // 绘制检测框和标签
  for DetectItem { kind, score, bbox } in result.items.iter() {
    draw_bbox_with_label(
      image,
      bbox,
      kind,
      *score,
      [0, 0, 255], // 蓝色边框
      &font,
    );
  }
}

/// 从 NchwFrame 创建 RgbImage
pub fn nchw_to_image<const W: u32, const H: u32>(frame: &RgbNchwFrame<W, H>) -> RgbImage {
  let width = frame.width() as u32;
  let height = frame.height() as u32;
  let data = frame.as_nchw();

  // 将 NCHW 转为 RGB 图像
  ImageBuffer::from_fn(width, height, |x, y| {
    let x = x as usize;
    let y = y as usize;
    let idx = y * (width as usize) + x;
    let r = data[idx];
    let g = data[(height as usize * width as usize) + idx];
    let b = data[(2 * height as usize * width as usize) + idx];
    Rgb([r, g, b])
  })
}

/// 从 NhwcFrame 创建 RgbImage
pub fn nhwc_to_image<const W: u32, const H: u32>(frame: &RgbNhwcFrame<W, H>) -> RgbImage {
  let width = frame.width() as u32;
  let height = frame.height() as u32;
  let data = frame.as_nhwc();

  // 将 NHWC 转为 RGB 图像
  ImageBuffer::from_fn(width, height, |x, y| {
    let x = x as usize;
    let y = y as usize;
    let idx = (y * (width as usize) + x) * 3;
    let r = data[idx];
    let g = data[idx + 1];
    let b = data[idx + 2];
    Rgb([r, g, b])
  })
}

/// 将 RgbImage 转换为 NHWC 格式的数据
pub fn image_to_nhwc(image: &RgbImage) -> Vec<u8> {
  let (width, height) = (image.width(), image.height());
  let mut data = vec![0u8; (width * height * 3) as usize];

  for y in 0..height {
    for x in 0..width {
      let pixel = image.get_pixel(x, y);
      let idx = ((y * width + x) * 3) as usize;
      data[idx] = pixel[0];
      data[idx + 1] = pixel[1];
      data[idx + 2] = pixel[2];
    }
  }

  data
}

/// 将 RgbImage 转换为 NCHW 格式的数据
pub fn image_to_nchw(image: &RgbImage) -> Vec<u8> {
  let (width, height) = (image.width(), image.height());
  let plane_size = (width * height) as usize;
  let mut data = vec![0u8; plane_size * 3];

  for y in 0..height {
    for x in 0..width {
      let pixel = image.get_pixel(x, y);
      let idx = (y * width + x) as usize;
      data[idx] = pixel[0];
      data[plane_size + idx] = pixel[1];
      data[2 * plane_size + idx] = pixel[2];
    }
  }

  data
}
