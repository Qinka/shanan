// 该文件是 Shanan （山南西风） 项目的一部分。
// src/detector/yolo.rs - YOLO 目标检测器
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use anyhow::{Context, Result};
use image::RgbImage;
use rknpu::{RknnContext, RknnInput, RknnOutput, TensorFormat, TensorType};

/// COCO 数据集类别名称
pub const COCO_CLASSES: [&str; 80] = [
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

/// 检测结果
#[derive(Clone, Debug)]
pub struct Detection {
  /// 边界框左上角 x 坐标
  pub x: f32,
  /// 边界框左上角 y 坐标
  pub y: f32,
  /// 边界框宽度
  pub width: f32,
  /// 边界框高度
  pub height: f32,
  /// 置信度
  pub confidence: f32,
  /// 类别索引
  pub class_id: usize,
  /// 类别名称
  pub class_name: String,
}

/// YOLO 目标检测器
pub struct YoloDetector {
  /// RKNN 上下文
  context: RknnContext,
  /// 模型输入宽度
  input_width: u32,
  /// 模型输入高度
  input_height: u32,
  /// 置信度阈值
  confidence_threshold: f32,
  /// NMS IOU 阈值
  nms_threshold: f32,
  /// 类别数量
  num_classes: usize,
}

impl YoloDetector {
  /// 创建一个新的 YOLO 检测器
  pub fn new(model_path: &str, confidence_threshold: f32, nms_threshold: f32) -> Result<Self> {
    let context = RknnContext::from_model_path(model_path)
      .with_context(|| format!("无法加载模型: {}", model_path))?;

    // 获取输入尺寸
    let input_attr = &context.input_attrs[0];
    let input_height = input_attr.dims[1];
    let input_width = input_attr.dims[2];

    Ok(Self {
      context,
      input_width,
      input_height,
      confidence_threshold,
      nms_threshold,
      num_classes: 80, // COCO 数据集有 80 个类别
    })
  }

  /// 预处理图像
  fn preprocess(&self, image: &RgbImage) -> Vec<u8> {
    // 调整图像大小到模型输入尺寸
    let resized = image::imageops::resize(
      image,
      self.input_width,
      self.input_height,
      image::imageops::FilterType::Triangle,
    );

    // 返回原始像素数据（NHWC 格式，已经是 RGB）
    resized.into_raw()
  }

  /// 运行推理
  pub fn detect(&self, image: &RgbImage) -> Result<Vec<Detection>> {
    let original_width = image.width() as f32;
    let original_height = image.height() as f32;

    // 预处理
    let input_data = self.preprocess(image);

    // 创建输入
    let input = RknnInput {
      index: 0,
      buf: input_data,
      size: (self.input_width * self.input_height * 3) as u32,
      pass_through: false,
      dtype: TensorType::Uint8,
      fmt: TensorFormat::NHWC,
    };

    // 运行推理
    let outputs = self.context.run(&[input])?;

    // 后处理
    let detections = self.postprocess(&outputs, original_width, original_height)?;

    Ok(detections)
  }

  /// 后处理输出
  fn postprocess(
    &self,
    outputs: &[RknnOutput],
    original_width: f32,
    original_height: f32,
  ) -> Result<Vec<Detection>> {
    let mut detections = Vec::new();

    // YOLO 输出格式: [batch, grid_h, grid_w, (5 + num_classes)]
    // 其中 5 = x, y, w, h, objectness
    let scales = [(80, 8), (40, 16), (20, 32)]; // (grid_size, stride)

    for (output_idx, output) in outputs.iter().enumerate() {
      if output_idx >= scales.len() {
        break;
      }

      let (grid_size, stride) = scales[output_idx];
      let output_data = &output.buf;

      for row in 0..grid_size {
        for col in 0..grid_size {
          let base_idx = (row * grid_size + col) * (5 + self.num_classes);

          if base_idx + 5 + self.num_classes > output_data.len() {
            continue;
          }

          let objectness = output_data[base_idx + 4];

          if objectness < self.confidence_threshold {
            continue;
          }

          // 找到最高类别分数
          let mut max_class_score = 0.0f32;
          let mut max_class_id = 0usize;

          for class_id in 0..self.num_classes {
            let score = output_data[base_idx + 5 + class_id];
            if score > max_class_score {
              max_class_score = score;
              max_class_id = class_id;
            }
          }

          let confidence = objectness * max_class_score;
          if confidence < self.confidence_threshold {
            continue;
          }

          // 解码边界框
          let cx = (col as f32 + output_data[base_idx]) * stride as f32;
          let cy = (row as f32 + output_data[base_idx + 1]) * stride as f32;
          let w = output_data[base_idx + 2] * self.input_width as f32;
          let h = output_data[base_idx + 3] * self.input_height as f32;

          // 转换为左上角坐标和宽高
          let x = cx - w / 2.0;
          let y = cy - h / 2.0;

          // 缩放到原始图像尺寸
          let scale_x = original_width / self.input_width as f32;
          let scale_y = original_height / self.input_height as f32;

          detections.push(Detection {
            x: x * scale_x,
            y: y * scale_y,
            width: w * scale_x,
            height: h * scale_y,
            confidence,
            class_id: max_class_id,
            class_name: COCO_CLASSES
              .get(max_class_id)
              .unwrap_or(&"unknown")
              .to_string(),
          });
        }
      }
    }

    // 应用 NMS
    let detections = self.nms(detections);

    Ok(detections)
  }

  /// 非极大值抑制
  fn nms(&self, mut detections: Vec<Detection>) -> Vec<Detection> {
    // 按置信度降序排序
    detections.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

    let mut result = Vec::new();

    while !detections.is_empty() {
      let best = detections.remove(0);
      result.push(best.clone());

      detections.retain(|det| {
        if det.class_id != best.class_id {
          return true;
        }
        self.iou(&best, det) < self.nms_threshold
      });
    }

    result
  }

  /// 计算两个边界框的 IoU
  fn iou(&self, a: &Detection, b: &Detection) -> f32 {
    let x1 = a.x.max(b.x);
    let y1 = a.y.max(b.y);
    let x2 = (a.x + a.width).min(b.x + b.width);
    let y2 = (a.y + a.height).min(b.y + b.height);

    let intersection = (x2 - x1).max(0.0) * (y2 - y1).max(0.0);
    let area_a = a.width * a.height;
    let area_b = b.width * b.height;
    let union = area_a + area_b - intersection;

    if union > 0.0 {
      intersection / union
    } else {
      0.0
    }
  }
}
