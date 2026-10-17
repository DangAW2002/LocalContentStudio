import React from "react";
import { Dialog, Button } from "@studio/design-system";
import { Icon } from "../../components/Icon";
import { studioApi } from "../../api/client";

export interface CreateArtifactDialogProps {
  open: boolean;
  onClose: () => void;
  projectSlug: string;
  existingIds: string[];
  onCreated: (newArtifactSource: string) => void;
}

export type ArtifactKindChoice = "design" | "motion" | "timeline" | "text";

const KIND_OPTIONS: Array<{
  id: ArtifactKindChoice;
  label: string;
  desc: string;
  icon: "design" | "motion" | "video" | "file";
  defaultSource: string;
}> = [
  {
    id: "design",
    label: "Thiết kế",
    desc: "Bản vẽ đồ họa HTML/CSS tĩnh hỗ trợ layers (1080×1350)",
    icon: "design",
    defaultSource: "index.html",
  },
  {
    id: "motion",
    label: "Chuyển động",
    desc: "Hoạt cảnh HyperFrames GSAP mượt mà (1920×1080 60fps)",
    icon: "motion",
    defaultSource: "index.html",
  },
  {
    id: "timeline",
    label: "Dòng thời gian",
    desc: "Dựng video đa track kèm video, audio và phụ đề timeline.json",
    icon: "video",
    defaultSource: "timeline.json",
  },
  {
    id: "text",
    label: "Văn bản",
    desc: "Kịch bản, dàn ý bài viết định dạng Markdown",
    icon: "file",
    defaultSource: "content.md",
  },
];

const TIMELINE_STARTER_JSON = `{
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
`;

const DESIGN_STARTER_HTML = `<!DOCTYPE html>
<html lang="vi">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=1080, height=1350, initial-scale=1.0">
  <title>Design Artboard</title>
  <link rel="stylesheet" href="brand.css">
  <style>
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      width: 1080px; height: 1350px; overflow: hidden;
      background-color: var(--brand-bg, #0b0f14);
      color: var(--brand-text, #ffffff);
      font-family: var(--brand-font-body, system-ui, sans-serif);
      display: flex; flex-direction: column; position: relative;
    }
    .artboard {
      width: 100%; height: 100%; position: relative;
      display: flex; flex-direction: column; justify-content: space-between;
      padding: 80px;
    }
    .main-layer { display: flex; flex-direction: column; gap: 24px; }
    h1 { font-size: 72px; font-weight: 800; color: var(--brand-text, #ffffff); }
    p { font-size: 32px; color: var(--brand-accent, #94a3b8); }
  </style>
</head>
<body>
  <div class="artboard" data-layer="artboard-root">
    <div class="main-layer" data-layer="content">
      <h1 data-layer="heading">Tiêu đề thiết kế mới</h1>
      <p data-layer="subtitle">Nội dung phụ đề được tạo theo chuẩn thương hiệu.</p>
    </div>
  </div>
</body>
</html>
`;

const DEFAULT_BRAND_CSS = `/* Auto-generated brand variables fallback */
:root {
  --brand-primary: #1e88e5;
  --brand-accent: #ffb300;
  --brand-bg: #0b0f14;
  --brand-text: #ffffff;
  --brand-font-heading: 'Inter', system-ui, sans-serif;
  --brand-font-body: 'Inter', system-ui, sans-serif;
}
`;

const MOTION_STARTER_HTML = `<!doctype html>
<html lang="vi">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=1920, height=1080" />
    <title>HyperFrames Composition</title>
    <script src="https://cdn.jsdelivr.net/npm/gsap@3.14.2/dist/gsap.min.js"></script>
    <style>
      body { margin: 0; background: #0b0f14; color: white; font-family: Inter, system-ui, sans-serif; }
      #root { position: relative; width: 100%; height: 100%; overflow: hidden; }
      .clip { position: absolute; inset: 0; display: grid; place-items: center; }
      h1 { margin: 0; font-size: 80px; font-weight: 800; }
      p { margin-top: 16px; font-size: 32px; color: #94a3b8; }
    </style>
  </head>
  <body>
    <div id="root" data-composition-id="main" data-start="0" data-width="1920" data-height="1080" data-duration="5">
      <section id="title-card" class="clip" data-start="0" data-duration="5">
        <div style="text-align: center;">
          <h1 id="title">Chuyển động Studio</h1>
          <p id="desc">Tạo hoạt cảnh với GSAP và HyperFrames</p>
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
`;

const TEXT_STARTER_MD = `---
title: Kịch bản nội dung mới
channel: youtube
tags:
  - studio
  - script
---

# Tiêu đề nội dung

Nội dung chi tiết cho kịch bản hoặc bài đăng.
`;

export const CreateArtifactDialog: React.FC<CreateArtifactDialogProps> = ({
  open,
  onClose,
  projectSlug,
  existingIds,
  onCreated,
}) => {
  const [kind, setKind] = React.useState<ArtifactKindChoice>("timeline");
  const [id, setId] = React.useState("");
  const [title, setTitle] = React.useState("");
  const [creating, setCreating] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  // Auto-generate suggested ID when kind changes or dialog opens
  React.useEffect(() => {
    if (!open) return;
    const prefix = kind === "timeline" ? "timeline" : kind === "design" ? "design" : kind === "motion" ? "motion" : "text";
    let counter = 1;
    while (existingIds.includes(`${prefix}-${String(counter).padStart(2, "0")}`)) {
      counter++;
    }
    const suggestedId = `${prefix}-${String(counter).padStart(2, "0")}`;
    setId(suggestedId);

    const kindLabel = KIND_OPTIONS.find((k) => k.id === kind)?.label || kind;
    setTitle(`${kindLabel} ${counter}`);
    setError(null);
  }, [open, kind, existingIds]);

  const handleCreate = async () => {
    const cleanId = id.trim().toLowerCase();
    const cleanTitle = title.trim();

    if (!cleanId) {
      setError("Mã định danh artifact không được để trống.");
      return;
    }
    if (!/^[a-z0-9_-]+$/.test(cleanId)) {
      setError("Mã định danh chỉ được chứa chữ thường không dấu, số, dấu gạch nối (-) hoặc gạch dưới (_).");
      return;
    }
    if (existingIds.includes(cleanId)) {
      setError(`Mã artifact "${cleanId}" đã tồn tại trong dự án.`);
      return;
    }
    if (!cleanTitle) {
      setError("Tiêu đề artifact không được để trống.");
      return;
    }

    setCreating(true);
    setError(null);

    try {
      const selectedOption = KIND_OPTIONS.find((k) => k.id === kind)!;
      const defaultSource = selectedOption.defaultSource;

      // 1. Build artifact.yaml content
      const artifactYaml = `id: "${cleanId}"
kind: "${kind}"
title: "${cleanTitle}"
brand: null
channels: []
source: "${defaultSource}"
inputs: []
params: {}
`;

      // 2. Write artifact.yaml
      await studioApi.writeFile({
        project: projectSlug,
        path: `artifacts/${cleanId}/artifact.yaml`,
        text: artifactYaml,
      });

      // 3. Write default source starter file
      let starterContent = "";
      if (kind === "timeline") {
        starterContent = TIMELINE_STARTER_JSON;
      } else if (kind === "design") {
        starterContent = DESIGN_STARTER_HTML;
        // Also write brand.css for design
        await studioApi.writeFile({
          project: projectSlug,
          path: `artifacts/${cleanId}/brand.css`,
          text: DEFAULT_BRAND_CSS,
        });
      } else if (kind === "motion") {
        starterContent = MOTION_STARTER_HTML;
      } else {
        starterContent = TEXT_STARTER_MD;
      }

      await studioApi.writeFile({
        project: projectSlug,
        path: `artifacts/${cleanId}/${defaultSource}`,
        text: starterContent,
      });

      onCreated(`projects/${projectSlug}/artifacts/${cleanId}/${defaultSource}`);
      onClose();
    } catch (err: any) {
      setError(err?.message || "Lỗi khi tạo artifact.");
    } finally {
      setCreating(false);
    }
  };

  return (
    <Dialog
      title="Tạo thành phẩm (Artifact) mới"
      open={open}
      onClose={onClose}
      actions={
        <>
          <Button variant="secondary" onClick={onClose} disabled={creating}>
            Hủy
          </Button>
          <Button
            variant="primary"
            busy={creating}
            busyText="Đang tạo..."
            onClick={handleCreate}
          >
            Tạo artifact
          </Button>
        </>
      }
    >
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)", minWidth: 420 }}>
        {error && (
          <div
            style={{
              padding: "8px 12px",
              backgroundColor: "rgba(239, 68, 68, 0.1)",
              border: "1px solid var(--cs-danger)",
              borderRadius: "var(--cs-radius-control)",
              color: "var(--cs-danger)",
              fontSize: "var(--cs-text-xs)",
            }}
          >
            {error}
          </div>
        )}

        {/* Kind choices */}
        <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
          <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-secondary)" }}>
            Chọn loại Artifact
          </label>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
            {KIND_OPTIONS.map((opt) => {
              const isSelected = kind === opt.id;
              return (
                <div
                  key={opt.id}
                  onClick={() => setKind(opt.id)}
                  style={{
                    padding: "10px 12px",
                    borderRadius: "var(--cs-radius-control)",
                    border: `1px solid ${isSelected ? "var(--cs-accent)" : "var(--cs-border-subtle)"}`,
                    backgroundColor: isSelected ? "var(--cs-selection)" : "var(--cs-bg-raised)",
                    cursor: "pointer",
                    display: "flex",
                    flexDirection: "column",
                    gap: 4,
                    transition: "all var(--cs-motion-duration) ease",
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: 6 }}>
                    <Icon name={opt.icon} size={15} color={isSelected ? "var(--cs-accent)" : "var(--cs-fg-muted)"} />
                    <span style={{ fontWeight: 600, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-primary)" }}>
                      {opt.label}
                    </span>
                  </div>
                  <span style={{ fontSize: 11, color: "var(--cs-fg-muted)", lineHeight: 1.3 }}>
                    {opt.desc}
                  </span>
                </div>
              );
            })}
          </div>
        </div>

        {/* Artifact ID */}
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-secondary)" }}>
            Mã định danh (ID thư mục)
          </label>
          <input
            type="text"
            value={id}
            onChange={(e) => setId(e.target.value)}
            placeholder="vd: timeline-01, thumb-video..."
            style={{
              padding: "6px 10px",
              backgroundColor: "var(--cs-bg-app)",
              border: "1px solid var(--cs-border-subtle)",
              borderRadius: "var(--cs-radius-control)",
              color: "var(--cs-fg-primary)",
              fontFamily: "var(--cs-font-mono)",
              fontSize: "var(--cs-text-xs)",
              outline: "none",
            }}
          />
        </div>

        {/* Artifact Title */}
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-secondary)" }}>
            Tiêu đề hiển thị
          </label>
          <input
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="vd: Dòng thời gian chính, Video Intro..."
            style={{
              padding: "6px 10px",
              backgroundColor: "var(--cs-bg-app)",
              border: "1px solid var(--cs-border-subtle)",
              borderRadius: "var(--cs-radius-control)",
              color: "var(--cs-fg-primary)",
              fontSize: "var(--cs-text-xs)",
              outline: "none",
            }}
          />
        </div>
      </div>
    </Dialog>
  );
};
