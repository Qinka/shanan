// 该文件是 Shanan （山南西风） 项目的一部分。
// src/model/yolo26.rs - 模型定义
//
// 本文件根据 Apache 许可证第 2.0 版（以下简称“许可证”）授权使用；
// 除非遵守该许可证条款，否则您不得使用本文件。
// 您可通过以下网址获取许可证副本：
// http://www.apache.org/licenses/LICENSE-2.0
// 除非适用法律要求或书面同意，根据本许可协议分发的软件均按“原样”提供，
// 不附带任何形式的明示或暗示的保证或条件。
// 有关许可权限与限制的具体条款，请参阅本许可协议。
//
// Copyright (C) 2026 Johann Li <me@qinka.pro>, Wareless Group

// use image::Frame;
use rknpu::{Context, InitFlags, TensorType};
use shanan_cv::cubecl::Runtime;
use shanan_cv::cubecl::client::ComputeClient;
use shanan_cv::data::DataBuffer;
use shanan_trait::Postprocess;
use thiserror::Error;
use tracing::{debug, error, info};
use url::Url;

use crate::{
  FromUrl,
  FromUrlWithScheme,
  input::AsNhwcFrame,
  model::{BBox, DetectItem, DetectResult, Model, WithLabel},
  // utils::sigmoid,
};

const YOLO26_NUM_INPUTS: u32 = 1;
const YOLO26_NUM_OUTPUTS: u32 = 3;
const YOLO26_OBJECT_THRESH: f32 = 0.5;

#[cfg(not(feature = "cubecl-wgpu"))]
const SCV_P_DIM: u32 = 1;
#[cfg(feature = "cubecl-wgpu")]
const SCV_P_DIM: u32 = 256;

pub type Yolo26Nhwc<const W: u32, const H: u32> = Yolo26<W, H, crate::frame::RgbNhwcFrame<H, W>>;

pub struct Yolo26<const W: u32, const H: u32, Frame> {
  context: Context,
  _phantom: std::marker::PhantomData<Frame>,
}

pub struct Yolo26Postprocess<const W: u32, const H: u32, T, R: Runtime> {
  object_thresh: f32,
  postprocess: shanan_cv::postprocess::detection::Yolo26Bc<R, f32, u32>,
  cl_client: ComputeClient<R>,
  _phantom: std::marker::PhantomData<T>,
}

#[derive(Error, Debug)]
pub enum Yolo26Error {
  #[error("模型加载错误: {0}")]
  ModelLoadError(#[from] std::io::Error),
  #[error("模型无效: {0}, 错误: {1}")]
  ModelInvalid(String, rknpu::Error),
  #[error("RKNN 错误: {0}")]
  RknnError(#[from] rknpu::Error),
  #[error("模型路径错误: {0}")]
  ModelPathError(String),
  #[error("shanan-cv 后处理错误: {0}")]
  PostprocessError(#[from] shanan_cv::postprocess::detection::Yolo26BcError),
  #[error("shanan-cv databuffer 错误: {0}")]
  DataBufferError(#[from] shanan_cv::data::DataBufferError),
}

impl Yolo26Error {
  pub fn invalid(msg: &str, e: rknpu::Error) -> Self {
    Yolo26Error::ModelInvalid(msg.to_string(), e)
  }
}

pub struct Yolo26Builder {
  model_path: String,
  flags: InitFlags,
  object_thresh: f32,
  pdim: u32,
}

impl FromUrlWithScheme for Yolo26Builder {
  const SCHEME: &'static str = "yolo26";
}

impl FromUrl for Yolo26Builder {
  type Error = Yolo26Error;

  fn from_url(url: &Url) -> Result<Self, Self::Error> {
    if url.scheme() != Self::SCHEME {
      return Err(Yolo26Error::ModelPathError(format!(
        "模型路径必须使用 {} 方案",
        Self::SCHEME
      )));
    }

    let object_thresh = url
      .query_pairs()
      .find_map(|(k, v)| {
        if k == "object_thresh" {
          v.parse::<f32>().ok()
        } else {
          None
        }
      })
      .unwrap_or(YOLO26_OBJECT_THRESH);

    let pdim = url
      .query_pairs()
      .find_map(|(k, v)| {
        if k == "pdim" {
          v.parse::<u32>().ok()
        } else {
          None
        }
      })
      .unwrap_or(SCV_P_DIM);

    Ok(Yolo26Builder {
      model_path: url.path().to_string(),
      flags: InitFlags::default(),
      object_thresh,
      pdim,
    })
  }
}

impl Yolo26Builder {
  pub fn flags(mut self, flags: InitFlags) -> Self {
    self.flags = flags;
    self
  }

  pub fn build_postprocess<const W: u32, const H: u32, T: WithLabel, R: Runtime>(
    &self,
  ) -> Result<Yolo26Postprocess<W, H, T, R>, Yolo26Error> {
    Yolo26Postprocess::new(self.object_thresh, self.pdim)
  }

  pub fn build_model<const W: u32, const H: u32, Frame>(
    self,
  ) -> Result<Yolo26<W, H, Frame>, Yolo26Error> {
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

    let _phantom = std::marker::PhantomData;
    Ok(Yolo26 { context, _phantom })
  }
}

impl<const W: u32, const H: u32, Frame: AsNhwcFrame<H, W>> Model for Yolo26<W, H, Frame> {
  // type Input = RgbNchwFrame; // 输入为 NCHW 格式的字节数组
  type Input = Frame;
  type Output = rknpu::Output; // 输出为浮点数组
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

    Ok(output)
  }
}

impl<const W: u32, const H: u32, T: WithLabel, R: Runtime> Yolo26Postprocess<W, H, T, R> {
  pub fn new(object_thresh: f32, p: u32) -> Result<Self, Yolo26Error> {
    let ppconfig = shanan_cv::postprocess::detection::Yolo26BcConfig::default()
      .with_shape(W, H)
      .with_dim(p);

    let postprocess = ppconfig.build()?;

    let cl_client = R::client(&R::Device::default());

    Ok(Self {
      object_thresh,
      postprocess,
      cl_client,
      _phantom: std::marker::PhantomData,
    })
  }
}

impl<const W: u32, const H: u32, T: WithLabel, R: Runtime> Postprocess
  for Yolo26Postprocess<W, H, T, R>
{
  type Input = rknpu::Output; // 输出为浮点数组
  type Output = DetectResult<T>;
  type Error = Yolo26Error;

  fn process(&self, output: Self::Input) -> Result<Self::Output, Self::Error> {
    // 调试性输出结果
    debug!("后处理模型输出");
    let mut items = Vec::new();

    for i in 0..output.len() {
      let pred = match output.get_f32(i) {
        Ok(data) => data,
        Err(e) => {
          error!("获取输出张量失败: {}", e);
          return Err(Yolo26Error::RknnError(e));
        }
      };

      let head_len = pred.len() / (4 + T::LABEL_NUM as usize);

      // 调用 shanan-cv 的后处理函数
      let pred: DataBuffer<R, _> = DataBuffer::from_slice(
        pred,
        &[1, 4 + T::LABEL_NUM as usize, head_len], // 可以变化?
        &self.cl_client,
      )?;

      let (score, index, bbox) = self.postprocess.execute(&self.cl_client, &pred)?;

      let bbox = bbox.into_vec(&self.cl_client)?;
      let index = index.into_vec(&self.cl_client)?;
      let score = score.into_vec(&self.cl_client)?;

      for s in 0..head_len {
        let score_value = score[s];
        if score_value >= self.object_thresh {
          let class_id = index[s];
          let x_min = bbox[s];
          let y_min = bbox[head_len + s];
          let x_max = bbox[2 * head_len + s];
          let y_max = bbox[3 * head_len + s];

          items.push(DetectItem {
            kind: T::from_label_id(class_id),
            score: score_value,
            bbox: BBox {
              x_min,
              y_min,
              x_max,
              y_max,
            },
          });
        }
      }
    }

    Ok(DetectResult {
      items: items.into_boxed_slice(),
    })
  }
}
