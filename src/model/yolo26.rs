// 该文件是 Shanan （山南西风） 项目的一部分。
// src/model/yolo26.rs - 模型定义
//
// 本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。
// 本程序的发布旨在提供实用价值，但不作任何形式的担保，
// 包括但不限于对适销性或特定用途适用性的默示担保。
// 更多详情请参阅 GNU 通用公共许可证。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, ETVP

use rknpu::{Context, InitFlags, TensorType};
use thiserror::Error;
use tracing::{debug, error, info};
use url::Url;

use crate::{
  FromUrl, frame::{RgbNchwFrame, RgbNhwcFrame}, input::{AsNchwFrame, AsNhwcFrame}, model::{DetectItem, DetectResult, Model}
};

const YOLO26_NUM_INPUTS: u32 = 1;
const YOLO26_NUM_OUTPUTS: u32 = 6;
const YOLO26_CLASS_NUM: usize = 80;
const YOLO26_INPUT_W: f32 = 640.0;
const YOLO26_INPUT_H: f32 = 640.0;
const YOLO26_HEAD_SIZES: [(usize, usize); 3] = [(80, 80), (40, 40), (20, 20)];
const YOLO26_STRIDES: [f32; 3] = [8.0, 16.0, 32.0];
const YOLO26_OBJECT_THRESH: f32 = 0.5;

pub struct Yolo26<Frame> {
  context: Context,
  _phantom: std::marker::PhantomData<Frame>,
}

#[derive(Error, Debug)]
pub enum Yolo26Error {
  #[error("模型加载错误: {0}")]
  ModelLoadError(std::io::Error),
  #[error("模型无效: {0}, 错误: {1}")]
  ModelInvalid(String, rknpu::Error),
  #[error("RKNN 错误: {0}")]
  RknnError(rknpu::Error),
  #[error("模型路径错误: {0}")]
  ModelPathError(String),
}

impl From<std::io::Error> for Yolo26Error {
  fn from(err: std::io::Error) -> Self {
    Yolo26Error::ModelLoadError(err)
  }
}

impl From<rknpu::Error> for Yolo26Error {
  fn from(err: rknpu::Error) -> Self {
    Yolo26Error::RknnError(err)
  }
}

impl Yolo26Error {
  pub fn invalid(msg: &str, e: rknpu::Error) -> Self {
    Yolo26Error::ModelInvalid(msg.to_string(), e)
  }
}

pub struct Yolo26Builder {
  model_path: String,
  flags: InitFlags,
}

const YOLO26_SCHEME: &str = "yolo26";

impl FromUrl for Yolo26Builder {
  type Error = Yolo26Error;

  fn from_url(url: &Url) -> Result<Self, Self::Error> {
    if url.scheme() != YOLO26_SCHEME {
      return Err(Yolo26Error::ModelPathError(format!(
        "模型路径必须使用 {} 方案",
        YOLO26_SCHEME
      )));
    }

    Ok(Yolo26Builder {
      model_path: url.path().to_string(),
      flags: InitFlags::default(),
    })
  }
}

impl Yolo26Builder {

  pub fn flags(mut self, flags: InitFlags) -> Self {
    self.flags = flags;
    self
  }

  pub fn build<Frame>(self) -> Result<Yolo26<Frame>, Yolo26Error> {
    info!("加载模型文件: {}", self.model_path);
    let mode_data = std::fs::read(&self.model_path)?;
    debug!(
      "模型文件大小: {:.2} MB",
      mode_data.len() as f64 / (1024.0 * 1024.0)
    );

    info!("创建 RKNN 推理上下文");
    let context = Context::new(&mode_data, self.flags)?;
    info!("模型加载完成");

    match context.sdk_version() {
      Ok(version) => {
        if let Ok(api_ver) = version.api_version() {
          debug!("模型 API 版本: {}", api_ver);
        }
        if let Ok(drv_ver) = version.driver_version() {
          debug!("模型驱动版本: {}", drv_ver);
        }
      }
      Err(e) => {
        error!(" 查询 SDK 版本失败: {}", e);
        return Err(Yolo26Error::invalid("无法查询 SDK 版本", e));
      }
    }

    let num_inputs = context
      .num_inputs()
      .map_err(|e| Yolo26Error::invalid("无法获取输入数量", e))?;
    let num_outputs = context
      .num_outputs()
      .map_err(|e| Yolo26Error::invalid("无法获取输出数量", e))?;

    if num_inputs != YOLO26_NUM_INPUTS {
      error!(
        "预期模型输入数量为 {}, 实际为 {}",
        YOLO26_NUM_INPUTS, num_inputs
      );
      return Err(Yolo26Error::invalid(
        &format!(
          "预期模型输入数量为 {}, 实际为 {}",
          YOLO26_NUM_INPUTS, num_inputs
        ),
        rknpu::Error::InvalidModel,
      ));
    }

    if num_outputs != YOLO26_NUM_OUTPUTS {
      error!(
        "预期模型输出数量为 {}, 实际为 {}",
        YOLO26_NUM_OUTPUTS, num_outputs
      );
      return Err(Yolo26Error::invalid(
        &format!(
          "预期模型输出数量为 {}, 实际为 {}",
          YOLO26_NUM_OUTPUTS, num_outputs
        ),
        rknpu::Error::InvalidModel,
      ));
    }

    debug!("模型输入数量: {}", num_inputs);
    debug!("模型输出数量: {}", num_outputs);

    let _phantom = std::marker::PhantomData::<Frame>;
    Ok(Yolo26 { context, _phantom })
  }
}

/// 根据张量大小匹配回归和分类输出
/// 返回 (reg, cls) 元组，如果大小不匹配则返回 None
fn match_reg_cls_tensors<'a>(
  tensor1: &'a [f32],
  tensor2: &'a [f32],
  reg_expected: usize,
  cls_expected: usize,
  head_idx: usize,
  output_idx1: usize,
  output_idx2: usize,
) -> Option<(&'a [f32], &'a [f32])> {
  if tensor1.len() == reg_expected && tensor2.len() == cls_expected {
    debug!(
      "检测头 {}: 输出顺序正常 - 索引 {} 是回归，索引 {} 是分类",
      head_idx, output_idx1, output_idx2
    );
    Some((tensor1, tensor2))
  } else if tensor1.len() == cls_expected && tensor2.len() == reg_expected {
    debug!(
      "检测头 {}: 输出顺序交换 - 索引 {} 是分类，索引 {} 是回归",
      head_idx, output_idx1, output_idx2
    );
    Some((tensor2, tensor1))
  } else {
    error!(
      "检测头 {}: 输出大小不匹配 - 张量1: {}, 张量2: {}, 期望回归: {}, 期望分类: {}",
      head_idx,
      tensor1.len(),
      tensor2.len(),
      reg_expected,
      cls_expected
    );
    None
  }
}

impl<Frame: AsNhwcFrame> Model for Yolo26<Frame> {
  // type Input = RgbNchwFrame; // 输入为 NCHW 格式的字节数组
  type Input = Frame;
  type Output = DetectResult; // 输出为浮点数组
  type Error = Yolo26Error;

  fn infer(&self, input: &Self::Input) -> Result<Self::Output, Self::Error> {
    // 设置输入
    debug!("设置模型输入");
    self.context.set_input(
      0,
      input.as_nhwc(),
      rknpu::TensorFormat::NHWC,
      TensorType::UInt8,
    )?;

    // 执行推理
    debug!("执行模型推理");
    self.context.run()?;

    // 获取输出
    debug!("获取模型输出");

    let output = self.context.get_outputs()?;
    debug!("模型推理结果：{:?}", output);

    Ok(Self::postprocess(output))
  }

  fn postprocess(output: rknpu::Output) -> Self::Output {
    // 调试性输出结果
    debug!("后处理模型输出");
    let mut items = Vec::new();

    for (head_idx, (&(map_h, map_w), stride)) in
      YOLO26_HEAD_SIZES.iter().zip(YOLO26_STRIDES).enumerate()
    {
      let spatial = map_h * map_w;
      let reg_expected = 4 * spatial;
      let cls_expected = YOLO26_CLASS_NUM * spatial;

      // 获取该检测头的两个输出张量
      // 由于RKNN输出顺序可能不同，需要根据张量大小来判断哪个是回归，哪个是分类
      let output_idx1 = head_idx * 2;
      let output_idx2 = head_idx * 2 + 1;

      let tensor1 = match output.get_f32(output_idx1) {
        Ok(data) => data,
        Err(e) => {
          error!("获取第 {} 个输出失败: {}", output_idx1, e);
          continue;
        }
      };

      let tensor2 = match output.get_f32(output_idx2) {
        Ok(data) => data,
        Err(e) => {
          error!("获取第 {} 个输出失败: {}", output_idx2, e);
          continue;
        }
      };

      debug!(
        "检测头 {}: 张量1大小={}, 张量2大小={}, 空间大小={}x{}={}, 期望回归={}, 期望分类={}",
        head_idx,
        tensor1.len(),
        tensor2.len(),
        map_h,
        map_w,
        spatial,
        reg_expected,
        cls_expected
      );

      // 根据张量大小判断哪个是回归输出，哪个是分类输出
      let (reg, cls) = match match_reg_cls_tensors(
        tensor1,
        tensor2,
        reg_expected,
        cls_expected,
        head_idx,
        output_idx1,
        output_idx2,
      ) {
        Some(tensors) => tensors,
        None => continue,
      };

      for h in 0..map_h {
        for w in 0..map_w {
          let idx = h * map_w + w;

          let (score, class_id) = {
            let mut max_logit = f32::MIN;
            let mut cls_idx = 0usize;
            for c in 0..YOLO26_CLASS_NUM {
              let logit = cls[c * spatial + idx];
              if logit > max_logit {
                max_logit = logit;
                cls_idx = c;
              }
            }
            (sigmoid(max_logit), cls_idx as u32)
          };

          if score <= YOLO26_OBJECT_THRESH {
            continue;
          }

          let cx = reg[idx];
          let cy = reg[spatial + idx];
          let cw = reg[2 * spatial + idx];
          let ch = reg[3 * spatial + idx];

          let grid_x = (w as f32) + 0.5;
          let grid_y = (h as f32) + 0.5;

          let xmin = ((grid_x - cx) * stride).clamp(0.0, YOLO26_INPUT_W);
          let ymin = ((grid_y - cy) * stride).clamp(0.0, YOLO26_INPUT_H);
          let xmax = ((grid_x + cw) * stride).clamp(0.0, YOLO26_INPUT_W);
          let ymax = ((grid_y + ch) * stride).clamp(0.0, YOLO26_INPUT_H);

          if xmin >= 0.0 && ymin >= 0.0 && xmax <= YOLO26_INPUT_W && ymax <= YOLO26_INPUT_H {
            items.push(DetectItem {
              class_id,
              score,
              bbox: [
                xmin / YOLO26_INPUT_W,
                ymin / YOLO26_INPUT_H,
                xmax / YOLO26_INPUT_W,
                ymax / YOLO26_INPUT_H,
              ],
            });
          }
        }
      }
    }

    debug!("检测到 {} 个物体", items.len());
    debug!("检测结果: {:?}", items);

    DetectResult {
      items: items.into_boxed_slice(),
    }
  }
}

fn sigmoid(x: f32) -> f32 {
  1.0 / (1.0 + (-x).exp())
}
