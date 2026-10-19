import React from "react";
import { Button, Badge, Tabs } from "@studio/design-system";
import { Icon } from "../../components/Icon";
import type { ScopeItem } from "../../api/types";
import {
  type ChannelSpec,
  type FormatSpec,
  DEFAULT_CHANNEL_SPEC,
  computeAspectRatio,
  parseChannelYaml,
  stringifyChannelYaml,
} from "../../utils/channelYaml";
import { SafeZonePreview } from "./SafeZonePreview";
import { studioApi } from "../../api/client";

export interface ChannelDetailProps {
  selectedItem: ScopeItem;
  rawText: string | null;
  onSave?: (yamlText: string) => Promise<void>;
  onRevertOverride?: () => Promise<void>;
}

export const ChannelDetail: React.FC<ChannelDetailProps> = ({
  selectedItem,
  rawText,
  onSave,
  onRevertOverride,
}) => {
  const [tab, setTab] = React.useState<"form" | "yaml">("form");
  const [spec, setSpec] = React.useState<ChannelSpec>(() => {
    return rawText ? parseChannelYaml(rawText, selectedItem.id) : { ...DEFAULT_CHANNEL_SPEC, id: selectedItem.id };
  });
  const [yamlText, setYamlText] = React.useState<string>(rawText || "");
  const [saving, setSaving] = React.useState(false);
  const [saveSuccess, setSaveSuccess] = React.useState(false);
  const [reverting, setReverting] = React.useState(false);
  const [errorMsg, setErrorMsg] = React.useState<string | null>(null);
  const [validationErrors, setValidationErrors] = React.useState<Array<{ line: number; column: number; message: string }>>([]);

  React.useEffect(() => {
    if (rawText) {
      setSpec(parseChannelYaml(rawText, selectedItem.id));
      setYamlText(rawText);
    } else {
      const initial = { ...DEFAULT_CHANNEL_SPEC, id: selectedItem.id };
      setSpec(initial);
      setYamlText(stringifyChannelYaml(initial));
    }
    setErrorMsg(null);
    setValidationErrors([]);
  }, [rawText, selectedItem.id]);

  // Synchronize when switching to YAML tab
  const handleTabChange = (nextTab: string) => {
    if (nextTab === "yaml" && tab === "form") {
      setYamlText(stringifyChannelYaml(spec));
    } else if (nextTab === "form" && tab === "yaml") {
      try {
        setSpec(parseChannelYaml(yamlText, selectedItem.id));
      } catch {
        // keep current spec if yaml parsing fails
      }
    }
    setTab(nextTab as "form" | "yaml");
  };

  const updateFormat = (index: number, patch: Partial<FormatSpec>) => {
    const nextFormats = [...spec.formats];
    const target = { ...nextFormats[index], ...patch };
    if (patch.width !== undefined || patch.height !== undefined) {
      target.aspect = computeAspectRatio(target.width, target.height);
    }
    nextFormats[index] = target;
    setSpec({ ...spec, formats: nextFormats });
  };

  const updateSafeZone = (index: number, patch: Partial<FormatSpec["safe_zone"]>) => {
    const nextFormats = [...spec.formats];
    const target = { ...nextFormats[index] };
    target.safe_zone = { ...target.safe_zone, ...patch };
    nextFormats[index] = target;
    setSpec({ ...spec, formats: nextFormats });
  };

  const addFormat = () => {
    const newFmt: FormatSpec = {
      name: `format-${spec.formats.length + 1}`,
      width: 1080,
      height: 1920,
      aspect: "9:16",
      fps: 30,
      max_duration_s: 60,
      safe_zone: { top: 0.05, right: 0.05, bottom: 0.15, left: 0.05 },
    };
    setSpec({ ...spec, formats: [...spec.formats, newFmt] });
  };

  const removeFormat = (index: number) => {
    if (spec.formats.length <= 1) return;
    const next = spec.formats.filter((_, i) => i !== index);
    setSpec({ ...spec, formats: next });
  };

  const handleValidateYaml = async () => {
    try {
      const res = await studioApi.scopeValidate("channels", yamlText);
      if (res.valid) {
        setValidationErrors([]);
        setErrorMsg(null);
        setSaveSuccess(true);
        setTimeout(() => setSaveSuccess(false), 2000);
      } else {
        setValidationErrors(res.errors);
      }
    } catch (err: any) {
      setErrorMsg(err?.message || "Lỗi kiểm tra cú pháp.");
    }
  };

  const handleSave = async () => {
    const textToSave = tab === "form" ? stringifyChannelYaml(spec) : yamlText;
    setSaving(true);
    setErrorMsg(null);
    setValidationErrors([]);
    try {
      // Validate schema first
      const val = await studioApi.scopeValidate("channels", textToSave);
      if (!val.valid) {
        setValidationErrors(val.errors);
        setErrorMsg("Cấu hình chưa đúng quy chuẩn schema channel@1.");
        setSaving(false);
        return;
      }

      if (onSave) {
        await onSave(textToSave);
      } else {
        // Direct save via scope_override if inherited
        if (selectedItem.layer !== "workspace") {
          await studioApi.scopeOverride("channels", selectedItem.id);
        }
        await studioApi.writeFile({
          path: `channels/${selectedItem.id}.yaml`,
          text: textToSave,
        });
      }

      setSaveSuccess(true);
      setTimeout(() => setSaveSuccess(false), 3000);
    } catch (err: any) {
      setErrorMsg(err?.message || "Lỗi khi lưu tệp cấu hình kênh.");
    } finally {
      setSaving(false);
    }
  };

  const handleRevert = async () => {
    if (!window.confirm(`Bạn có chắc muốn bỏ bản ghi đè của kênh '${selectedItem.name}' để quay về bản kế thừa?`)) {
      return;
    }
    setReverting(true);
    setErrorMsg(null);
    try {
      if (onRevertOverride) {
        await onRevertOverride();
      } else {
        await studioApi.scopeRemoveOverride("channels", selectedItem.id);
      }
    } catch (err: any) {
      setErrorMsg(err?.message || "Lỗi khi bỏ bản ghi đè.");
    } finally {
      setReverting(false);
    }
  };

  const isInherited = selectedItem.layer !== "workspace";
  const isOverriding = Boolean(selectedItem.isOverride);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
      {/* Top Action Bar */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          backgroundColor: "var(--cs-bg-raised)",
          padding: "var(--cs-space-3) var(--cs-space-4)",
          borderRadius: "var(--cs-radius-control)",
          border: "1px solid var(--cs-border-subtle)",
        }}
      >
        <Tabs
          size="sm"
          items={[
            { id: "form", label: "Biểu mẫu trực quan" },
            { id: "yaml", label: "Mã nguồn YAML" },
          ]}
          activeId={tab}
          onChange={handleTabChange}
        />

        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {isOverriding && (
            <Button
              size="sm"
              variant="secondary"
              busy={reverting}
              busyText="Đang khôi phục..."
              onClick={handleRevert}
              leftIcon={<Icon name="undo" size={13} />}
              title="Xoá bản ghi đè ở workspace và quay lại bản gốc từ App/Org"
            >
              Bỏ bản ghi đè
            </Button>
          )}

          {tab === "yaml" && (
            <Button
              size="sm"
              variant="secondary"
              onClick={handleValidateYaml}
              leftIcon={<Icon name="check" size={13} />}
            >
              Kiểm tra cú pháp
            </Button>
          )}

          {saveSuccess && <Badge variant="success">✓ Đã lưu thành công</Badge>}

          <Button
            size="sm"
            variant="primary"
            busy={saving}
            busyText="Đang lưu..."
            onClick={handleSave}
            leftIcon={<Icon name="check" size={13} />}
            title={isInherited ? "Kênh kế thừa chỉ đọc — lưu sẽ tạo bản ghi đè trong workspace này" : undefined}
          >
            {isInherited ? "Lưu bản ghi đè" : "Lưu kênh"}
          </Button>
        </div>
      </div>

      {errorMsg && (
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
          {errorMsg}
        </div>
      )}

      {validationErrors.length > 0 && (
        <div
          style={{
            padding: "8px 12px",
            backgroundColor: "rgba(239, 68, 68, 0.08)",
            border: "1px solid var(--cs-danger)",
            borderRadius: "var(--cs-radius-control)",
            color: "var(--cs-danger)",
            fontSize: "var(--cs-text-xs)",
            display: "flex",
            flexDirection: "column",
            gap: 4,
          }}
        >
          <span style={{ fontWeight: 700 }}>Phát hiện lỗi cấu hình:</span>
          {validationErrors.map((err, idx) => (
            <div key={idx} style={{ fontFamily: "var(--cs-font-mono)" }}>
              • Dòng {err.line}, Cột {err.column}: {err.message}
            </div>
          ))}
        </div>
      )}

      {tab === "form" ? (
        <>
          {/* General Information */}
          <div style={cardSectionStyle}>
            <div style={sectionTitleStyle}>1. THÔNG TIN KÊNH</div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
              <label style={fieldLabelStyle}>
                <span>Tên hiển thị kênh</span>
                <input
                  type="text"
                  value={spec.title}
                  onChange={(e) => setSpec({ ...spec, title: e.target.value })}
                  style={inputStyle}
                />
              </label>

              <label style={fieldLabelStyle}>
                <span>Nền tảng (Platform)</span>
                <input
                  type="text"
                  value={spec.platform}
                  onChange={(e) => setSpec({ ...spec, platform: e.target.value })}
                  style={inputStyle}
                  placeholder="tiktok, youtube, facebook..."
                />
              </label>
            </div>

            <label style={{ ...fieldLabelStyle, marginTop: 8 }}>
              <span>Ghi chú biên tập (Notes)</span>
              <input
                type="text"
                value={spec.notes}
                onChange={(e) => setSpec({ ...spec, notes: e.target.value })}
                style={inputStyle}
                placeholder="Yêu cầu riêng của nền tảng..."
              />
            </label>
          </div>

          {/* Formats & Safe Zones Form */}
          <div style={cardSectionStyle}>
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <div style={sectionTitleStyle}>2. QUY CHUẨN ĐỊNH DẠNG KHUNG HÌNH & VÙNG AN TOÀN</div>
              <Button size="sm" variant="secondary" onClick={addFormat} leftIcon={<Icon name="plus" size={12} />}>
                Thêm format
              </Button>
            </div>
            <p style={{ margin: "0 0 12px 0", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
              Tỉ lệ khung hình được tự động tính toán từ chiều rộng × chiều cao. Vùng an toàn hiển thị xem thử trực quan theo thời gian thực.
            </p>

            <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
              {spec.formats.map((fmt, idx) => (
                <div
                  key={idx}
                  style={{
                    display: "flex",
                    gap: 20,
                    padding: 16,
                    backgroundColor: "var(--cs-bg-raised)",
                    borderRadius: "var(--cs-radius-control)",
                    border: "1px solid var(--cs-border-subtle)",
                    flexWrap: "wrap",
                  }}
                >
                  {/* Left Form Controls for Format */}
                  <div style={{ flex: 1, minWidth: 280, display: "flex", flexDirection: "column", gap: 10 }}>
                    <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                      <span style={{ fontWeight: 700, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-primary)" }}>
                        Định dạng #{idx + 1}
                      </span>
                      {spec.formats.length > 1 && (
                        <button
                          type="button"
                          onClick={() => removeFormat(idx)}
                          style={{
                            border: "none",
                            background: "transparent",
                            color: "var(--cs-danger)",
                            cursor: "pointer",
                            fontSize: "var(--cs-text-xs)",
                          }}
                        >
                          Xoá
                        </button>
                      )}
                    </div>

                    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
                      <label style={fieldLabelStyle}>
                        <span>Tên format</span>
                        <input
                          type="text"
                          value={fmt.name}
                          onChange={(e) => updateFormat(idx, { name: e.target.value })}
                          style={inputStyle}
                        />
                      </label>

                      <label style={fieldLabelStyle}>
                        <span>Tỉ lệ (Tự tính)</span>
                        <input
                          type="text"
                          value={fmt.aspect}
                          onChange={(e) => updateFormat(idx, { aspect: e.target.value })}
                          style={{ ...inputStyle, backgroundColor: "var(--cs-bg-panel)" }}
                        />
                      </label>
                    </div>

                    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
                      <label style={fieldLabelStyle}>
                        <span>Chiều rộng (px)</span>
                        <input
                          type="number"
                          value={fmt.width}
                          onChange={(e) => updateFormat(idx, { width: parseInt(e.target.value, 10) || 0 })}
                          style={inputStyle}
                        />
                      </label>

                      <label style={fieldLabelStyle}>
                        <span>Chiều cao (px)</span>
                        <input
                          type="number"
                          value={fmt.height}
                          onChange={(e) => updateFormat(idx, { height: parseInt(e.target.value, 10) || 0 })}
                          style={inputStyle}
                        />
                      </label>
                    </div>

                    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 8 }}>
                      <label style={fieldLabelStyle}>
                        <span>FPS</span>
                        <input
                          type="number"
                          value={fmt.fps ?? ""}
                          onChange={(e) => updateFormat(idx, { fps: parseFloat(e.target.value) || undefined })}
                          style={inputStyle}
                          placeholder="30"
                        />
                      </label>

                      <label style={fieldLabelStyle}>
                        <span>Tối thiểu (s)</span>
                        <input
                          type="number"
                          value={fmt.min_duration_s ?? ""}
                          onChange={(e) => updateFormat(idx, { min_duration_s: parseFloat(e.target.value) || undefined })}
                          style={inputStyle}
                          placeholder="3"
                        />
                      </label>

                      <label style={fieldLabelStyle}>
                        <span>Tối đa (s)</span>
                        <input
                          type="number"
                          value={fmt.max_duration_s ?? ""}
                          onChange={(e) => updateFormat(idx, { max_duration_s: parseFloat(e.target.value) || undefined })}
                          style={inputStyle}
                          placeholder="60"
                        />
                      </label>
                    </div>

                    {/* Safe Zone Margins */}
                    <div style={{ marginTop: 6 }}>
                      <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-secondary)" }}>
                        Lề vùng an toàn (0.00 – 1.00)
                      </span>
                      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr 1fr", gap: 6, marginTop: 4 }}>
                        <label style={fieldLabelStyle}>
                          <span>Trên</span>
                          <input
                            type="number"
                            step="0.01"
                            min="0"
                            max="1"
                            value={fmt.safe_zone.top}
                            onChange={(e) => updateSafeZone(idx, { top: parseFloat(e.target.value) || 0 })}
                            style={inputStyle}
                          />
                        </label>
                        <label style={fieldLabelStyle}>
                          <span>Dưới</span>
                          <input
                            type="number"
                            step="0.01"
                            min="0"
                            max="1"
                            value={fmt.safe_zone.bottom}
                            onChange={(e) => updateSafeZone(idx, { bottom: parseFloat(e.target.value) || 0 })}
                            style={inputStyle}
                          />
                        </label>
                        <label style={fieldLabelStyle}>
                          <span>Trái</span>
                          <input
                            type="number"
                            step="0.01"
                            min="0"
                            max="1"
                            value={fmt.safe_zone.left}
                            onChange={(e) => updateSafeZone(idx, { left: parseFloat(e.target.value) || 0 })}
                            style={inputStyle}
                          />
                        </label>
                        <label style={fieldLabelStyle}>
                          <span>Phải</span>
                          <input
                            type="number"
                            step="0.01"
                            min="0"
                            max="1"
                            value={fmt.safe_zone.right}
                            onChange={(e) => updateSafeZone(idx, { right: parseFloat(e.target.value) || 0 })}
                            style={inputStyle}
                          />
                        </label>
                      </div>
                    </div>
                  </div>

                  {/* Right Live Safe Zone Preview */}
                  <div
                    style={{
                      display: "flex",
                      flexDirection: "column",
                      alignItems: "center",
                      justifyContent: "center",
                      backgroundColor: "var(--cs-bg-panel)",
                      padding: 12,
                      borderRadius: "var(--cs-radius-control)",
                      border: "1px solid var(--cs-border-subtle)",
                      minWidth: 200,
                    }}
                  >
                    <SafeZonePreview format={fmt} />
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* Text Limits & File Constraints */}
          <div style={cardSectionStyle}>
            <div style={sectionTitleStyle}>3. GIỚI HẠN VĂN BẢN & ĐÓNG GÓI TỆP</div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr 1fr", gap: 10 }}>
              <label style={fieldLabelStyle}>
                <span>Tiêu đề tối đa (ký tự)</span>
                <input
                  type="number"
                  value={spec.text.title_max ?? ""}
                  onChange={(e) =>
                    setSpec({
                      ...spec,
                      text: { ...spec.text, title_max: parseInt(e.target.value, 10) || undefined },
                    })
                  }
                  style={inputStyle}
                  placeholder="100"
                />
              </label>

              <label style={fieldLabelStyle}>
                <span>Mô tả tối đa (ký tự)</span>
                <input
                  type="number"
                  value={spec.text.description_max ?? ""}
                  onChange={(e) =>
                    setSpec({
                      ...spec,
                      text: { ...spec.text, description_max: parseInt(e.target.value, 10) || undefined },
                    })
                  }
                  style={inputStyle}
                  placeholder="2000"
                />
              </label>

              <label style={fieldLabelStyle}>
                <span>Phụ đề/Post (ký tự)</span>
                <input
                  type="number"
                  value={spec.text.caption_max ?? ""}
                  onChange={(e) =>
                    setSpec({
                      ...spec,
                      text: { ...spec.text, caption_max: parseInt(e.target.value, 10) || undefined },
                    })
                  }
                  style={inputStyle}
                  placeholder="150"
                />
              </label>

              <label style={fieldLabelStyle}>
                <span>Hashtag tối đa</span>
                <input
                  type="number"
                  value={spec.text.hashtags_max ?? ""}
                  onChange={(e) =>
                    setSpec({
                      ...spec,
                      text: { ...spec.text, hashtags_max: parseInt(e.target.value, 10) || undefined },
                    })
                  }
                  style={inputStyle}
                  placeholder="10"
                />
              </label>
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "2fr 1fr 1fr", gap: 10, marginTop: 10 }}>
              <label style={fieldLabelStyle}>
                <span>Định dạng tệp cho phép (phân cách bằng dấu phẩy)</span>
                <input
                  type="text"
                  value={spec.file.containers.join(", ")}
                  onChange={(e) =>
                    setSpec({
                      ...spec,
                      file: {
                        ...spec.file,
                        containers: e.target.value
                          .split(",")
                          .map((s) => s.trim())
                          .filter(Boolean),
                      },
                    })
                  }
                  style={inputStyle}
                  placeholder="mp4, mov"
                />
              </label>

              <label style={fieldLabelStyle}>
                <span>Kích thước tối đa (MB)</span>
                <input
                  type="number"
                  value={spec.file.max_size_mb ?? ""}
                  onChange={(e) =>
                    setSpec({
                      ...spec,
                      file: { ...spec.file, max_size_mb: parseInt(e.target.value, 10) || undefined },
                    })
                  }
                  style={inputStyle}
                  placeholder="500"
                />
              </label>

              <label style={fieldLabelStyle}>
                <span>Độ lớn âm thanh LUFS</span>
                <input
                  type="number"
                  step="0.5"
                  value={spec.loudness_lufs ?? ""}
                  onChange={(e) =>
                    setSpec({
                      ...spec,
                      loudness_lufs: parseFloat(e.target.value) || undefined,
                    })
                  }
                  style={inputStyle}
                  placeholder="-14.0"
                />
              </label>
            </div>

            <div style={{ marginTop: 10, display: "flex", alignItems: "center", gap: 8 }}>
              <input
                type="checkbox"
                id="captions_required"
                checked={spec.captions_required}
                onChange={(e) => setSpec({ ...spec, captions_required: e.target.checked })}
                style={{ cursor: "pointer" }}
              />
              <label htmlFor="captions_required" style={{ fontSize: "var(--cs-text-xs)", cursor: "pointer" }}>
                Bắt buộc có phụ đề (Captions / Subtitles) khi xuất bản
              </label>
            </div>
          </div>
        </>
      ) : (
        /* Direct YAML Editor */
        <div style={cardSectionStyle}>
          <div style={sectionTitleStyle}>CHỈNH SỬA YAML TRỰC TIẾP (CHANNEL@1)</div>
          <p style={{ margin: "0 0 8px 0", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
            Bạn có thể sửa trực tiếp nội dung cấu hình. Khi bấm Lưu hoặc Kiểm tra cú pháp, hệ thống sẽ đối chiếu với schema channel@1.
          </p>
          <textarea
            value={yamlText}
            onChange={(e) => setYamlText(e.target.value)}
            rows={18}
            style={{
              width: "100%",
              padding: "var(--cs-space-3)",
              backgroundColor: "var(--cs-bg-raised)",
              border: "1px solid var(--cs-border-subtle)",
              borderRadius: "var(--cs-radius-control)",
              fontFamily: "var(--cs-font-mono)",
              fontSize: 12,
              lineHeight: 1.5,
              resize: "vertical",
              color: "var(--cs-fg-primary)",
              boxSizing: "border-box",
            }}
          />
        </div>
      )}
    </div>
  );
};

const cardSectionStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 8,
  padding: "var(--cs-space-4)",
  backgroundColor: "var(--cs-bg-panel)",
  border: "1px solid var(--cs-border-subtle)",
  borderRadius: "var(--cs-radius-control)",
};

const sectionTitleStyle: React.CSSProperties = {
  fontSize: "var(--cs-text-xs)",
  fontWeight: 700,
  color: "var(--cs-fg-secondary)",
  letterSpacing: 0.5,
};

const fieldLabelStyle: React.CSSProperties = {
  display: "flex",
  flexDirection: "column",
  gap: 4,
  fontSize: "var(--cs-text-xs)",
  fontWeight: 500,
  color: "var(--cs-fg-secondary)",
};

const inputStyle: React.CSSProperties = {
  padding: "6px 8px",
  backgroundColor: "var(--cs-bg-raised)",
  border: "1px solid var(--cs-border-subtle)",
  borderRadius: "var(--cs-radius-control)",
  color: "var(--cs-fg-primary)",
  fontSize: "var(--cs-text-xs)",
  outline: "none",
};
