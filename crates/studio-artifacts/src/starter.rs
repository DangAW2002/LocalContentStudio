pub const DESIGN_STARTER_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=1080, height=1350, initial-scale=1.0">
  <title>Design Artboard</title>
  <link rel="stylesheet" href="brand.css">
  <style>
    * {
      box-sizing: border-box;
      margin: 0;
      padding: 0;
    }
    body {
      width: 1080px;
      height: 1350px;
      overflow: hidden;
      background-color: var(--brand-bg, #0b0f14);
      color: var(--brand-text, #ffffff);
      font-family: var(--brand-font-body, system-ui, -apple-system, sans-serif);
      display: flex;
      flex-direction: column;
      position: relative;
    }
    .artboard {
      width: 100%;
      height: 100%;
      position: relative;
      display: flex;
      flex-direction: column;
      justify-content: space-between;
      padding: 80px;
    }
    [data-layer] {
      position: relative;
    }
    .header-layer {
      display: flex;
      align-items: center;
      gap: 20px;
    }
    .brand-tag {
      font-size: 24px;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 2px;
      color: var(--brand-primary, #6366f1);
    }
    .main-layer {
      display: flex;
      flex-direction: column;
      gap: 24px;
    }
    h1 {
      font-size: 72px;
      font-weight: 800;
      line-height: 1.1;
      font-family: var(--brand-font-heading, inherit);
      color: var(--brand-text, #ffffff);
    }
    p {
      font-size: 32px;
      line-height: 1.4;
      color: var(--brand-accent, #94a3b8);
    }
    .footer-layer {
      display: flex;
      justify-content: space-between;
      align-items: center;
      border-top: 2px solid rgba(255, 255, 255, 0.1);
      padding-top: 30px;
      font-size: 24px;
      color: var(--brand-accent, #64748b);
    }
  </style>
</head>
<body>
  <div class="artboard" data-layer="artboard-root">
    <div class="header-layer" data-layer="header">
      <span class="brand-tag" data-layer="brand-tag">STUDIO</span>
    </div>
    <div class="main-layer" data-layer="content">
      <h1 data-layer="heading">Tiêu đề thiết kế nổi bật</h1>
      <p data-layer="subtitle">Phụ đề mô tả ngắn gọn nội dung thiết kế theo chuẩn brand.</p>
    </div>
    <div class="footer-layer" data-layer="footer">
      <span data-layer="channel-spec">1080 × 1350 · 4:5</span>
      <span data-layer="action-hint">Local Content Studio</span>
    </div>
  </div>
</body>
</html>
"#;

pub const DEFAULT_BRAND_CSS: &str = r#"/* Auto-generated brand variables fallback */
:root {
  --brand-primary: #6366f1;
  --brand-accent: #94a3b8;
  --brand-bg: #0b0f14;
  --brand-text: #ffffff;
  --brand-font-heading: 'Inter', system-ui, sans-serif;
  --brand-font-body: 'Inter', system-ui, sans-serif;
}
"#;

pub const MOTION_STARTER_HTML: &str = r##"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=1920, height=1080" />
    <title>HyperFrames Composition</title>
    <script src="https://cdn.jsdelivr.net/npm/gsap@3.14.2/dist/gsap.min.js"></script>
    <style>
      body {
        margin: 0;
        background: #0b0f14;
        color: white;
        font-family: Inter, system-ui, sans-serif;
      }
      #root {
        position: relative;
        width: 100%;
        height: 100%;
        overflow: hidden;
      }
      .clip {
        position: absolute;
        inset: 0;
        display: grid;
        place-items: center;
      }
      h1 {
        margin: 0;
        font-size: 80px;
        font-weight: 800;
        letter-spacing: -1px;
      }
      p {
        margin-top: 16px;
        font-size: 32px;
        color: #94a3b8;
      }
    </style>
  </head>
  <body>
    <div
      id="root"
      data-composition-id="main"
      data-start="0"
      data-width="1920"
      data-height="1080"
      data-duration="5"
    >
      <section id="title-card" class="clip" data-start="0" data-duration="5">
        <div style="text-align: center;">
          <h1 id="title">HyperFrames Motion</h1>
          <p id="desc">Animated with GSAP & HyperFrames</p>
        </div>
      </section>
    </div>
    <script>
      window.__timelines = window.__timelines || {};
      const tl = gsap.timeline({ paused: true });
      tl.from("#title", { y: 48, opacity: 0, duration: 0.8, ease: "power3.out" }, 0.2);
      tl.from("#desc", { y: 24, opacity: 0, duration: 0.6, ease: "power3.out" }, 0.5);
      window.__timelines["main"] = tl;
    </script>
  </body>
</html>
"##;

pub const TIMELINE_STARTER: &str = r#"{
  "version": 1,
  "width": 1920,
  "height": 1080,
  "fps": 30.0,
  "tracks": [
    {
      "id": "video-1",
      "kind": "video",
      "clips": [
        {
          "id": "clip-video-1",
          "source": "color:#1e88e5",
          "in_point": 0.0,
          "out_point": 5.0,
          "start": 0.0,
          "duration": 5.0,
          "transition": "none"
        }
      ]
    },
    {
      "id": "audio-1",
      "kind": "audio",
      "clips": []
    },
    {
      "id": "captions-1",
      "kind": "captions",
      "clips": [
        {
          "id": "cap-1",
          "source": "Xin chào! Chào mừng bạn đến với Local Content Studio.",
          "start": 0.5,
          "duration": 2.0
        },
        {
          "id": "cap-2",
          "source": "Dựng video dễ dàng với AI và timeline trực quan.",
          "start": 2.5,
          "duration": 2.0
        }
      ]
    }
  ]
}
"#;

pub const TIMELINE_STARTER_JSON: &str = TIMELINE_STARTER;

pub const TEXT_STARTER_MD: &str = r#"---
title: Tiêu đề nội dung
channel: youtube
tags:
  - studio
  - content
---

# Tiêu đề nội dung

Nội dung chi tiết cho kịch bản hoặc bài đăng.
"#;

pub const GENERATION_STARTER_JSON: &str = r#"{
  "prompt": "",
  "model": "flux-schnell",
  "seed": 42,
  "params": {}
}
"#;

pub const AUDIO_STARTER_MD: &str = r#"---
voice: default
speed: 1.0
---

Xin chào, đây là kịch bản âm thanh mẫu.
"#;

pub const REFERENCE_STARTER_MD: &str = r#"# Hồ sơ tham chiếu

Mô tả đặc điểm nhận dạng nhân vật, phong cách hình ảnh hoặc phong cách giọng đọc.

## Đặc điểm nhận dạng
- Nhân vật / Phong cách: Mô tả chi tiết diện mạo hoặc phong cách
- Chi tiết trang phục / bối cảnh: Trang phục cố định, ánh sáng, góc chụp
"#;
