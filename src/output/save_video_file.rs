// 该文件是 Shanan （山南西风） 项目的一部分。
// src/output/save_video_file.rs - 保存视频文件
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

//! 视频文件输出模块
//!
//! 此模块提供将检测结果保存为 MP4 视频文件的功能，类似于 SaveImageFileOutput。
//!
//! # 使用示例
//!
//! ```no_run
//! use shanan::{FromUrl, output::SaveVideoFileOutput};
//! use url::Url;
//!
//! // 创建视频输出，默认 25 fps
//! let url = Url::parse("video:///path/to/output.mp4").unwrap();
//! let output = SaveVideoFileOutput::from_url(&url).unwrap();
//!
//! // 或者指定 fps
//! let url = Url::parse("video:///path/to/output.mp4?fps=30").unwrap();
//! let output = SaveVideoFileOutput::from_url(&url).unwrap();
//!
//! // 使用 Render trait 保存每一帧
//! for frame in video_input.into_nhwc() {
//!     let result = model.infer(&frame)?;
//!     output.render_result(&frame, &result)?;
//! }
//! // 视频在 output 被销毁时自动完成编码
//! ```
//!
//! # URL 格式
//!
//! - `video:///path/to/output.mp4` - 指定输出视频路径，默认 25 fps
//! - `video:///path/to/output.mp4?fps=30` - 指定输出视频路径和帧率
//!
//! # 依赖
//!
//! 此模块需要系统安装 ffmpeg 命令行工具来编码视频。

use std::cell::RefCell;
use std::path::Path;
use std::process::Command;

use ab_glyph::{FontRef, PxScale};
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut};
use thiserror::Error;
use tracing::{error, info, warn};
use url::Url;

use crate::{
  frame::{RgbNchwFrame, RgbNhwcFrame},
  input::{AsNchwFrame, AsNhwcFrame},
  model::{DetectItem, DetectResult},
  output::Render,
  FromUrl,
};

// COCO 80 类别名称
const COCO_CLASSES: [&str; 80] = [
  "person",
  "bicycle",
  "car",
  "motorcycle",
  "airplane",
  "bus",
  "train",
  "truck",
  "boat",
  "traffic light",
  "fire hydrant",
  "stop sign",
  "parking meter",
  "bench",
  "bird",
  "cat",
  "dog",
  "horse",
  "sheep",
  "cow",
  "elephant",
  "bear",
  "zebra",
  "giraffe",
  "backpack",
  "umbrella",
  "handbag",
  "tie",
  "suitcase",
  "frisbee",
  "skis",
  "snowboard",
  "sports ball",
  "kite",
  "baseball bat",
  "baseball glove",
  "skateboard",
  "surfboard",
  "tennis racket",
  "bottle",
  "wine glass",
  "cup",
  "fork",
  "knife",
  "spoon",
  "bowl",
  "banana",
  "apple",
  "sandwich",
  "orange",
  "broccoli",
  "carrot",
  "hot dog",
  "pizza",
  "donut",
  "cake",
  "chair",
  "couch",
  "potted plant",
  "bed",
  "dining table",
  "toilet",
  "tv",
  "laptop",
  "mouse",
  "remote",
  "keyboard",
  "cell phone",
  "microwave",
  "oven",
  "toaster",
  "sink",
  "refrigerator",
  "book",
  "clock",
  "vase",
  "scissors",
  "teddy bear",
  "hair drier",
  "toothbrush",
];

// 文本渲染常量
const LABEL_FONT_SIZE: f32 = 20.0;
const LABEL_TEXT_HEIGHT: i32 = 24;
const LABEL_CHAR_WIDTH: f32 = 11.0; // 每字符平均宽度（粗略估计）
const LABEL_TEXT_VERTICAL_PADDING: i32 = 2;

// 在图像上绘制一个矩形边框，bbox 为归一化坐标 [x_min, y_min, x_max, y_max]
fn draw_bbox_with_label(
  image: &mut RgbImage,
  bbox: &[f32; 4],
  class_id: u32,
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

  // 获取类别名称
  let class_name = if (class_id as usize) < COCO_CLASSES.len() {
    COCO_CLASSES[class_id as usize]
  } else {
    "unknown"
  };

  // 创建标签文本
  let label = format!("{} {:.2}", class_name, score);

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

#[derive(Error, Debug)]
pub enum SaveVideoFileError {
  #[error("I/O 错误: {0}")]
  IoError(#[from] std::io::Error),
  #[error("图像错误: {0}")]
  ImageError(#[from] image::ImageError),
  #[error("URI 方案不匹配: {0}")]
  SchemeMismatch(String),
  #[error("视频编码错误: {0}")]
  EncodingError(String),
  #[error("字体加载错误")]
  FontLoadError,
  #[error("参数验证错误: {0}")]
  ValidationError(String),
}

const SAVE_VIDEO_FILE_SCHEME: &str = "video";
const DEFAULT_FPS: u32 = 25;
const MIN_FPS: u32 = 1;
const MAX_FPS: u32 = 120;

/// 视频帧缓存
struct FrameBuffer {
  temp_dir: String,
  frames: Vec<String>,
  width: Option<u32>,
  height: Option<u32>,
}

impl FrameBuffer {
  fn new(output_path: &str) -> Result<Self, SaveVideoFileError> {
    // 在输出文件旁边创建临时目录
    let temp_dir = format!("{}.frames", output_path);
    std::fs::create_dir_all(&temp_dir)?;

    Ok(FrameBuffer {
      temp_dir,
      frames: Vec::new(),
      width: None,
      height: None,
    })
  }

  fn add_frame(&mut self, image: &RgbImage) -> Result<(), SaveVideoFileError> {
    let width = image.width();
    let height = image.height();

    // 初始化宽高
    if self.width.is_none() {
      self.width = Some(width);
      self.height = Some(height);
    }

    // 保存帧为临时文件
    let frame_index = self.frames.len();
    let frame_path = format!("{}/frame_{:06}.png", self.temp_dir, frame_index);
    image.save(&frame_path)?;
    self.frames.push(frame_path);

    Ok(())
  }

  fn encode_to_video(&self, output_path: &str, fps: u32) -> Result<(), SaveVideoFileError> {
    if self.frames.is_empty() {
      info!("没有帧可以编码为视频，跳过编码过程");
      return Ok(());
    }

    info!(
      "开始编码视频: {} 帧 @ {} fps -> {}",
      self.frames.len(),
      fps,
      output_path
    );

    // 创建输出目录
    if let Some(parent) = Path::new(output_path).parent() {
      if !parent.as_os_str().is_empty() {
        std::fs::create_dir_all(parent)?;
      }
    }

    // 使用 ffmpeg 将帧序列编码为 MP4
    let ffmpeg_result = Command::new("ffmpeg")
      .arg("-y") // 覆盖已存在的文件
      .arg("-loglevel")
      .arg("error") // 减少日志输出
      .arg("-framerate")
      .arg(fps.to_string())
      .arg("-i")
      .arg(format!("{}/frame_%06d.png", self.temp_dir))
      .arg("-c:v")
      .arg("libx264") // 使用 H.264 编码
      .arg("-pix_fmt")
      .arg("yuv420p") // 兼容性格式
      .arg("-preset")
      .arg("fast") // 编码速度预设
      .arg("-crf")
      .arg("23") // 质量参数（0-51，越小质量越好）
      .arg(output_path)
      .output();

    match ffmpeg_result {
      Ok(output) => {
        if output.status.success() {
          info!("视频编码成功: {}", output_path);
          Ok(())
        } else {
          let stderr = String::from_utf8_lossy(&output.stderr);
          error!("ffmpeg 错误: {}", stderr);
          Err(SaveVideoFileError::EncodingError(format!(
            "ffmpeg 失败: {}",
            stderr
          )))
        }
      }
      Err(e) => {
        error!("无法执行 ffmpeg: {}", e);
        Err(SaveVideoFileError::EncodingError(format!(
          "无法执行 ffmpeg (请确保已安装): {}",
          e
        )))
      }
    }
  }

  fn cleanup(&self) {
    // 清理临时文件
    if let Err(e) = std::fs::remove_dir_all(&self.temp_dir) {
      warn!("清理临时目录失败: {}", e);
    }
  }
}

pub struct SaveVideoFileOutput {
  path: String,
  fps: u32,
  buffer: RefCell<Option<FrameBuffer>>,
}

impl FromUrl for SaveVideoFileOutput {
  type Error = SaveVideoFileError;

  fn from_url(uri: &Url) -> Result<Self, Self::Error> {
    if uri.scheme() != SAVE_VIDEO_FILE_SCHEME {
      return Err(SaveVideoFileError::SchemeMismatch(format!(
        "期望保存方式 '{}', 实际保存方式 '{}'",
        SAVE_VIDEO_FILE_SCHEME,
        uri.scheme()
      )));
    }

    // 从 URL 查询参数中获取 FPS（如果有的话）
    let fps = uri
      .query_pairs()
      .find(|(k, _)| k == "fps")
      .and_then(|(_, v)| v.parse::<u32>().ok())
      .unwrap_or(DEFAULT_FPS);

    // 验证 FPS 范围
    if fps < MIN_FPS || fps > MAX_FPS {
      return Err(SaveVideoFileError::ValidationError(format!(
        "FPS {} 超出有效范围 [{}, {}]",
        fps, MIN_FPS, MAX_FPS
      )));
    }

    Ok(SaveVideoFileOutput {
      path: uri.path().to_string(),
      fps,
      buffer: RefCell::new(None),
    })
  }
}

impl SaveVideoFileOutput {
  fn ensure_buffer_initialized(&self) -> Result<(), SaveVideoFileError> {
    let mut buffer_opt = self.buffer.borrow_mut();
    if buffer_opt.is_none() {
      let buffer = FrameBuffer::new(&self.path)?;
      *buffer_opt = Some(buffer);
      info!("初始化视频帧缓冲区: {}", self.path);
    }
    Ok(())
  }

  fn render_detect_result(
    &self,
    mut image: RgbImage,
    result: &DetectResult,
  ) -> Result<(), SaveVideoFileError> {
    // 加载嵌入的字体
    let font_data = include_bytes!("../../assets/font.ttf");
    let font = FontRef::try_from_slice(font_data).map_err(|_| SaveVideoFileError::FontLoadError)?;

    // 绘制检测框和标签
    for DetectItem {
      class_id,
      score,
      bbox,
    } in result.items.iter()
    {
      draw_bbox_with_label(
        &mut image,
        bbox,
        *class_id,
        *score,
        [0, 255, 0], // 绿色边框（视频中更清晰）
        &font,
      );
    }

    // 确保缓冲区已初始化
    self.ensure_buffer_initialized()?;

    // 添加帧到缓冲区
    let mut buffer_opt = self.buffer.borrow_mut();
    if let Some(buffer) = buffer_opt.as_mut() {
      buffer.add_frame(&image)?;
    }

    Ok(())
  }
}

impl Render<RgbNchwFrame, DetectResult> for SaveVideoFileOutput {
  type Error = SaveVideoFileError;

  fn render_result(&self, frame: &RgbNchwFrame, result: &DetectResult) -> Result<(), Self::Error> {
    let width = frame.width() as u32;
    let height = frame.height() as u32;
    let data = frame.as_nchw();

    // 将 NCHW 转为 RGB 图像
    let image: RgbImage = ImageBuffer::from_fn(width, height, |x, y| {
      let x = x as usize;
      let y = y as usize;
      let idx = y * (width as usize) + x;
      let r = data[idx];
      let g = data[(height as usize * width as usize) + idx];
      let b = data[(2 * height as usize * width as usize) + idx];
      Rgb([r, g, b])
    });

    self.render_detect_result(image, result)
  }
}

impl Render<RgbNhwcFrame, DetectResult> for SaveVideoFileOutput {
  type Error = SaveVideoFileError;

  fn render_result(&self, frame: &RgbNhwcFrame, result: &DetectResult) -> Result<(), Self::Error> {
    let width = frame.width() as u32;
    let height = frame.height() as u32;
    let data = frame.as_nhwc();

    // 将 NHWC 转为 RGB 图像
    let image: RgbImage = ImageBuffer::from_fn(width, height, |x, y| {
      let x = x as usize;
      let y = y as usize;
      let idx = (y * (width as usize) + x) * 3;
      let r = data[idx];
      let g = data[idx + 1];
      let b = data[idx + 2];
      Rgb([r, g, b])
    });

    self.render_detect_result(image, result)
  }
}

impl Drop for SaveVideoFileOutput {
  fn drop(&mut self) {
    // 在对象销毁时完成视频编码
    if let Some(buffer) = self.buffer.borrow_mut().take() {
      // 确保清理总是执行
      let cleanup_guard = CleanupGuard(&buffer);

      if let Err(e) = buffer.encode_to_video(&self.path, self.fps) {
        error!("编码视频时出错: {}", e);
      } else {
        info!("视频已保存到文件: {}", self.path);
      }

      // cleanup_guard 在此处自动执行 cleanup
    }
  }
}

/// RAII 守卫确保清理总是执行
struct CleanupGuard<'a>(&'a FrameBuffer);

impl<'a> Drop for CleanupGuard<'a> {
  fn drop(&mut self) {
    self.0.cleanup();
  }
}
