Shanan 山南西风
============

#### 介绍
山南西风 - 基于 RKNPU 的视觉推理框架

## 功能特性

- 支持从图片、视频或 V4L2 摄像头读取数据
- 使用 YOLO 目标检测算法检测图像中的对象
- 在检测结果上绘制边界框和标签
- 将结果保存到图片或视频文件

## 依赖项

- RKNN 模型文件 (*.rknn)
- FFmpeg 开发库
- V4L2 设备（如需使用摄像头）

## 安装依赖

```bash
# Ubuntu/Debian
sudo apt-get install libavcodec-dev libavformat-dev libavutil-dev libswscale-dev libavfilter-dev libavdevice-dev
```

## 编译

```bash
cargo build --release
```

## 使用方法

```bash
# 处理图片
shanan --model model.rknn --input input.jpg --output output.jpg

# 处理视频
shanan --model model.rknn --input input.mp4 --output output.mp4

# 处理摄像头（限制处理 100 帧）
shanan --model model.rknn --input /dev/video0 --output output.mp4 --max-frames 100

# 自定义检测参数
shanan --model model.rknn --input input.jpg --output output.jpg --confidence 0.6 --nms-threshold 0.5
```

## 命令行参数

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `--model` | RKNN 模型文件路径 | 必需 |
| `--input` | 输入来源（图片/视频/V4L2 设备） | 必需 |
| `--output` | 输出文件路径 | 必需 |
| `--confidence` | 置信度阈值 (0.0-1.0) | 0.5 |
| `--nms-threshold` | NMS IOU 阈值 (0.0-1.0) | 0.45 |
| `--max-frames` | 最大处理帧数（0=无限制） | 0 |

## 支持的输入格式

- **图片**: JPG, JPEG, PNG, BMP, GIF, WebP
- **视频**: MP4, AVI, MKV 等 FFmpeg 支持的格式
- **摄像头**: /dev/video0 或 v4l2:///dev/video0

## 许可证

本程序遵循 GNU Affero 通用公共许可证（AGPL）许可协议。

