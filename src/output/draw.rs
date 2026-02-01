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
const LABEL_COLOR: [u8; 3] = [0, 0, 255]; // 蓝色

pub struct Draw<'a> {
  font_size: f32,
  label_text_height: i32,
  label_char_width: f32,
  label_text_vertical_padding: i32,
  font: FontRef<'a>,
  label_color: [u8; 3],
}

impl<'a> Default for Draw<'a> {
  fn default() -> Self {
    let font_data = include_bytes!("../../assets/font.ttf"); // default font
    let font = FontRef::try_from_slice(font_data).expect("无法加载嵌入的字体文件");

    Self {
      font_size: LABEL_FONT_SIZE,
      label_text_height: LABEL_TEXT_HEIGHT,
      label_char_width: LABEL_CHAR_WIDTH,
      label_text_vertical_padding: LABEL_TEXT_VERTICAL_PADDING,
      label_color: LABEL_COLOR,
      font,
    }
  }
}

impl<'a> Draw<'a> {
  // 在图像上绘制一个矩形边框，bbox 为归一化坐标 [x_min, y_min, x_max, y_max]
  fn draw_bbox_with_label<T: WithLabel>(
    &self,
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
    let scale = PxScale::from(self.font_size);
    let text_color = Rgb([255u8, 255u8, 255u8]); // 白色文本

    // 估算文本大小（粗略估计）
    let text_width = (label.len() as f32 * self.label_char_width) as i32;
    let text_height = self.label_text_height;

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
        label_y + self.label_text_vertical_padding,
        scale,
        font,
        &label,
      );
    }
  }
}

pub trait DrawDetecctionOnImage<T: WithLabel> {
  fn draw_detections_on_image(&self, image: &mut RgbImage, result: &DetectResult<T>);
}

pub trait ToRgbImage {
  fn to_rgb_image(&self) -> RgbImage;
}

pub trait FromRgbImage {
  fn from_rgb_image(image: &RgbImage) -> Self;
}

pub trait DrawDetectionOnFrame<FromFrame, ToFrame, T: WithLabel> {
  fn draw_detection(&self, frame: &FromFrame, result: &DetectResult<T>) -> ToFrame;
}

impl<FromFrame: ToRgbImage, ToFrame: FromRgbImage, T: WithLabel, D: DrawDetecctionOnImage<T>>
  DrawDetectionOnFrame<FromFrame, ToFrame, T> for D
{
  fn draw_detection(&self, frame: &FromFrame, result: &DetectResult<T>) -> ToFrame {
    let mut image = frame.to_rgb_image();
    self.draw_detections_on_image(&mut image, result);
    ToFrame::from_rgb_image(&image)
  }
}

impl<const W: u32, const H: u32> ToRgbImage for RgbNchwFrame<W, H> {
  fn to_rgb_image(&self) -> RgbImage {
    let width = self.width() as u32;
    let height = self.height() as u32;
    let data = self.as_nchw();

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
}

impl<const W: u32, const H: u32> ToRgbImage for RgbNhwcFrame<W, H> {
  fn to_rgb_image(&self) -> RgbImage {
    let width = self.width() as u32;
    let height = self.height() as u32;
    let data = self.as_nhwc();

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
}

impl<const W: u32, const H: u32> FromRgbImage for RgbNhwcFrame<W, H> {
  fn from_rgb_image(image: &RgbImage) -> Self {
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

    RgbNhwcFrame::from(data)
  }
}

impl<const W: u32, const H: u32> FromRgbImage for RgbNchwFrame<W, H> {
  fn from_rgb_image(image: &RgbImage) -> Self {
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

    RgbNchwFrame::from(data)
  }
}

impl<T: WithLabel> DrawDetecctionOnImage<T> for Draw<'_> {
  fn draw_detections_on_image(&self, image: &mut RgbImage, result: &DetectResult<T>) {
    // 绘制检测框和标签
    for DetectItem { kind, score, bbox } in result.items.iter() {
      self.draw_bbox_with_label(
        image,
        bbox,
        kind,
        *score,
        self.label_color, // 蓝色边框
        &self.font,
      );
    }
  }
}

impl FromRgbImage for RgbImage {
  fn from_rgb_image(image: &RgbImage) -> Self {
    image.clone()
  }
}

pub struct Record {
  pub label_with_name: bool,
}

impl Record {
  pub fn record<T: WithLabel>(
    &self,
    result: &DetectResult<T>,
    path: &std::path::Path,
  ) -> Result<(), std::io::Error> {
    let mut records = Vec::new();
    for item in result.items.iter() {
      let name = if self.label_with_name {
        item.kind.to_label_str()
      } else {
        format!("{}", item.kind.to_label_id())
      };
      let record = format!(
        "{}, {:.4}, {:.4}, {:.4}, {:.4}, {:.4}",
        name, item.score, item.bbox[0], item.bbox[1], item.bbox[2], item.bbox[3]
      );
      records.push(record);
    }
    std::fs::write(path.with_extension("txt"), records.join("\n"))?;
    Ok(())
  }
}
