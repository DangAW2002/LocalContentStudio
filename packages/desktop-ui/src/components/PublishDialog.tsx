import React from "react";
import { Icon } from "./Icon";
import {
  Button,
  Badge,
  Callout,
  Select,
  Input,
  Spinner,
} from "@studio/design-system";
import { studioApi, parseYouTubeError } from "../api/client";
import type {
  ArtifactSummary,
  PublishFinding,
  PublishMetadata,
  PublishPrivacy,
  PublishRequest,
  PublishTarget,
  YouTubeChannelAccount,
  YouTubeErrorDetail,
} from "../api/types";
import { YouTubeGuideModal } from "./YouTubeGuideModal";

export interface PublishDialogProps {
  open: boolean;
  onClose: () => void;
  projectSlug: string;
  artifact?: ArtifactSummary | null;
  initialRequest?: PublishRequest | null;
  onPublished?: (req: PublishRequest) => void;
}

const TARGET_OPTIONS = [
  { value: "youtube", label: "YouTube (Tải lên trực tiếp)" },
  { value: "facebook", label: "Facebook (Xuất gói tệp)" },
  { value: "tiktok", label: "TikTok (Xuất gói tệp)" },
  { value: "instagram", label: "Instagram (Xuất gói tệp)" },
  { value: "export", label: "Xuất bản gói tệp (Thư mục cục bộ)" },
];

const PRIVACY_OPTIONS = [
  { value: "private", label: "Riêng tư (Chỉ mình bạn xem)" },
  { value: "unlisted", label: "Không công khai (Người có liên kết)" },
  { value: "public", label: "Công khai (Mọi người xem)" },
];

const TARGET_VI: Record<PublishTarget, string> = {
  youtube: "YouTube",
  facebook: "Facebook",
  tiktok: "TikTok",
  instagram: "Instagram",
  export: "Xuất bản gói tệp",
};

const PRIVACY_VI: Record<PublishPrivacy, string> = {
  private: "Riêng tư",
  unlisted: "Không công khai",
  public: "Công khai",
};

const CHANNEL_LIMITS: Record<
  PublishTarget,
  { titleMax: number; descMax: number; hashtagsMax: number }
> = {
  youtube: { titleMax: 100, descMax: 5000, hashtagsMax: 15 },
  facebook: { titleMax: 255, descMax: 5000, hashtagsMax: 30 },
  tiktok: { titleMax: 2200, descMax: 2200, hashtagsMax: 30 },
  instagram: { titleMax: 2200, descMax: 2200, hashtagsMax: 30 },
  export: { titleMax: 255, descMax: 5000, hashtagsMax: 30 },
};

export const PublishDialog: React.FC<PublishDialogProps> = ({
  open,
  onClose,
  projectSlug,
  artifact,
  initialRequest,
  onPublished,
}) => {
  const [step, setStep] = React.useState<"edit" | "confirm" | "publishing" | "result">("edit");
  const [target, setTarget] = React.useState<PublishTarget>("youtube");
  const [channel, setChannel] = React.useState<string>("youtube");
  const [title, setTitle] = React.useState("");
  const [description, setDescription] = React.useState("");
  const [tagsInput, setTagsInput] = React.useState("");
  const [privacy, setPrivacy] = React.useState<PublishPrivacy>("private");
  const [thumbnail, setThumbnail] = React.useState("");
  const [schedule, setSchedule] = React.useState("");

  const [currentRequest, setCurrentRequest] = React.useState<PublishRequest | null>(null);
  const [findings, setFindings] = React.useState<PublishFinding[]>([]);
  const [files, setFiles] = React.useState<string[]>([]);
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);
  const [structuredError, setStructuredError] = React.useState<YouTubeErrorDetail | null>(null);
  const [oauthSuccess, setOauthSuccess] = React.useState<string | null>(null);
  const [youtubeChannels, setYoutubeChannels] = React.useState<YouTubeChannelAccount[]>([]);
  const [selectedChannelId, setSelectedChannelId] = React.useState<string>("");
  const [guideModalOpen, setGuideModalOpen] = React.useState(false);

  // Progress state during publishing
  const [publishProgress, setPublishProgress] = React.useState<number | null>(null);
  const [publishMessage, setPublishMessage] = React.useState<string>("");
  const [publishResultUrl, setPublishResultUrl] = React.useState<string | null>(null);

  // Fetch YouTube channels when dialog is opened
  React.useEffect(() => {
    if (open) {
      studioApi
        .youtubeChannelsList()
        .then((list) => {
          setYoutubeChannels(list);
          if (list.length > 0 && !selectedChannelId) {
            setSelectedChannelId(list[0].id);
          }
        })
        .catch(() => {});
    }
  }, [open, selectedChannelId]);

  // Initialize form from initialRequest or artifact
  React.useEffect(() => {
    if (!open) {
      setStep("edit");
      setError(null);
      setStructuredError(null);
      setOauthSuccess(null);
      setPublishProgress(null);
      setPublishResultUrl(null);
      return;
    }

    if (initialRequest) {
      setCurrentRequest(initialRequest);
      setTarget(initialRequest.target);
      setChannel(initialRequest.channel || initialRequest.target);
      setTitle(initialRequest.metadata.title || "");
      setDescription(initialRequest.metadata.description || "");
      setTagsInput((initialRequest.metadata.tags || []).join(", "));
      setPrivacy(initialRequest.metadata.privacy || "private");
      setThumbnail(initialRequest.metadata.thumbnail || "");
      setSchedule(initialRequest.metadata.schedule || "");
      setFiles(initialRequest.files || []);
      if (initialRequest.status === "awaiting_confirmation") {
        setStep("confirm");
      } else if (initialRequest.status === "published") {
        setStep("result");
        setPublishResultUrl(initialRequest.resultUrl || "exported");
      } else {
        setStep("edit");
      }
    } else if (artifact) {
      setCurrentRequest(null);
      const defaultChannel =
        artifact.channels && artifact.channels.length > 0 ? artifact.channels[0] : "youtube";
      setChannel(defaultChannel);
      const defaultTarget: PublishTarget = defaultChannel.includes("youtube")
        ? "youtube"
        : defaultChannel.includes("tiktok")
        ? "tiktok"
        : defaultChannel.includes("facebook")
        ? "facebook"
        : defaultChannel.includes("instagram")
        ? "instagram"
        : "export";
      setTarget(defaultTarget);
      setTitle(artifact.title || artifact.id || "");
      setDescription("");
      setTagsInput("");
      setPrivacy("private");
      setThumbnail("");
      setSchedule("");
      setFiles([]);
      setFindings([]);
      setStep("edit");
    }
  }, [open, initialRequest, artifact]);

  const prevActiveElementRef = React.useRef<HTMLElement | null>(null);

  React.useEffect(() => {
    if (open) {
      if (document.activeElement instanceof HTMLElement) {
        prevActiveElementRef.current = document.activeElement;
      }
      const handleKeyDown = (e: KeyboardEvent) => {
        if (e.key === "Escape") {
          handleCancel();
        }
      };
      window.addEventListener("keydown", handleKeyDown);
      return () => window.removeEventListener("keydown", handleKeyDown);
    } else if (prevActiveElementRef.current) {
      prevActiveElementRef.current.focus?.();
      prevActiveElementRef.current = null;
    }
  }, [open]);

  // Subscribe to job events to track publish progress
  React.useEffect(() => {
    let unlisten: (() => void) | undefined;
    if (open && (step === "publishing" || step === "confirm")) {
      studioApi
        .onJobEvent((payload) => {
          if (payload.kind === "publish") {
            if (payload.progress !== undefined && payload.progress !== null) {
              setPublishProgress(Math.round(payload.progress * 100));
            }
            if (payload.message) {
              setPublishMessage(payload.message);
            }
            if (payload.state === "succeeded") {
              setPublishProgress(100);
              setStep("result");
              if (currentRequest?.id) {
                studioApi.publishGet(currentRequest.id).then((updated) => {
                  setCurrentRequest(updated);
                  setPublishResultUrl(updated.resultUrl || "exported");
                  if (onPublished) onPublished(updated);
                });
              }
            } else if (payload.state === "failed") {
              setError(payload.message || "Tác vụ đăng tải gặp sự cố.");
              setStep("confirm");
            }
          }
        })
        .then((fn) => {
          unlisten = fn;
        })
        .catch(() => {});
    }

    return () => {
      if (unlisten) unlisten();
    };
  }, [open, step, currentRequest, onPublished]);

  if (!open) return null;

  // Limits calculation
  const limits = CHANNEL_LIMITS[target] || CHANNEL_LIMITS.youtube;
  const currentTags = tagsInput
    .split(/[,#\s]+/)
    .map((t) => t.trim().replace(/^#/, ""))
    .filter((t) => t.length > 0);

  const titleOverflow = title.length > limits.titleMax;
  const descOverflow = description.length > limits.descMax;
  const tagsOverflow = currentTags.length > limits.hashtagsMax;

  // Handle prepare & advance to Step 2 (Confirmation)
  const handleProceedToConfirm = async () => {
    setBusy(true);
    setError(null);
    try {
      const artId = artifact?.id || currentRequest?.artifact || "main";
      const metadata: PublishMetadata = {
        title: title.trim(),
        description: description.trim(),
        tags: currentTags,
        privacy,
        thumbnail: thumbnail.trim() ? thumbnail.trim() : undefined,
        schedule: schedule.trim() ? schedule.trim() : undefined,
      };

      if (currentRequest?.id) {
        // Update existing request
        const updated = await studioApi.publishUpdateMetadata(currentRequest.id, metadata);
        setCurrentRequest(updated);
        setFiles(updated.files || []);
      } else {
        // Prepare new draft
        const outcome = await studioApi.publishPrepare({
          project: projectSlug,
          artifact: artId,
          channel: channel || target,
          target,
          metadata,
        });
        setCurrentRequest(outcome.request);
        setFindings(outcome.findings);
        setFiles(outcome.files);
      }
      setStep("confirm");
    } catch (err: any) {
      setError(err?.message || "Không thể chuẩn bị bản đăng. Vui lòng kiểm tra render.");
    } finally {
      setBusy(false);
    }
  };

  // Handle YouTube OAuth Connect
  const handleConnectYouTube = async () => {
    setBusy(true);
    setError(null);
    setStructuredError(null);
    setOauthSuccess(null);
    try {
      const chan = await studioApi.youtubeConnect();
      setOauthSuccess(`Đã kết nối tài khoản YouTube thành công: ${chan.title} (${chan.handle ? "@" + chan.handle : chan.id})`);
      const updatedList = await studioApi.youtubeChannelsList();
      setYoutubeChannels(updatedList);
      setSelectedChannelId(chan.id);
    } catch (err: any) {
      const parsed = parseYouTubeError(err);
      if (parsed) {
        setStructuredError(parsed);
      } else {
        setError(err?.message || "Không thể kết nối YouTube.");
      }
    } finally {
      setBusy(false);
    }
  };

  // Handle Confirm Publish (Step 2 -> Step 3)
  const handleConfirmPublish = async () => {
    if (!currentRequest?.id) return;
    setBusy(true);
    setError(null);
    setStructuredError(null);
    setStep("publishing");
    setPublishProgress(10);
    setPublishMessage("Đang khởi tạo tác vụ đăng tải...");

    try {
      const req = await studioApi.publishConfirm(
        currentRequest.id,
        target === "youtube" ? (selectedChannelId || undefined) : undefined
      );
      setCurrentRequest(req);
      if (req.status === "published") {
        setPublishProgress(100);
        setPublishResultUrl(req.resultUrl || "exported");
        setStep("result");
        if (onPublished) onPublished(req);
      }
    } catch (err: any) {
      const parsed = parseYouTubeError(err);
      if (parsed) {
        setStructuredError(parsed);
      } else {
        setError(err?.message || "Đăng tải thất bại. Bản đăng được giữ nguyên để bạn thử lại thủ công.");
      }
      setStep("confirm");
    } finally {
      setBusy(false);
    }
  };

  // Handle Cancel Publish
  const handleCancel = async () => {
    if (currentRequest?.id && currentRequest.status === "awaiting_confirmation") {
      try {
        await studioApi.publishCancel(currentRequest.id);
      } catch {
        // ignore
      }
    }
    onClose();
  };

  return (
    <div
      role="presentation"
      onClick={handleCancel}
      style={{
        position: "fixed",
        inset: 0,
        backgroundColor: "var(--cs-overlay)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 1000,
      }}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="publish-dialog-title"
        onClick={(e) => e.stopPropagation()}
        style={{
          width: "100%",
          maxWidth: "600px",
          backgroundColor: "var(--cs-bg-panel)",
          boxShadow: "var(--cs-shadow-overlay)",
          border: "1px solid var(--cs-border-strong)",
          overflow: "hidden",
          display: "flex",
          flexDirection: "column",
          maxHeight: "92vh",
        }}
      >
        {/* Header */}
        <div
          style={{
            padding: "var(--cs-space-3) var(--cs-space-4)",
            borderBottom: "1px solid var(--cs-border-subtle)",
            backgroundColor: "var(--cs-bg-raised)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <Icon name="publishing" size={18} color="var(--cs-accent)" />
            <h2
              id="publish-dialog-title"
              style={{
                margin: 0,
                fontSize: "var(--cs-text-md)",
                fontWeight: 600,
                color: "var(--cs-fg-primary)",
              }}
            >
              {step === "confirm"
                ? "Xác nhận xuất bản nội dung"
                : step === "publishing"
                ? "Đang xuất bản nội dung..."
                : step === "result"
                ? "Xuất bản thành công"
                : "Xuất bản & Đăng tải"}
            </h2>
          </div>
          <Button size="sm" variant="quiet" onClick={handleCancel} aria-label="Đóng">
            ✕
          </Button>
        </div>

        {/* Body Content */}
        <div style={{ padding: "var(--cs-space-4)", overflowY: "auto", flex: 1 }}>
          {structuredError && (
            <Callout
              variant="danger"
              title={structuredError.messageVi}
              style={{ marginBottom: "var(--cs-space-3)" }}
            >
              <p style={{ margin: "0 0 8px 0" }}>{structuredError.fixVi}</p>
              <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={() => {
                    if (step === "confirm") {
                      handleConfirmPublish();
                    } else {
                      handleConnectYouTube();
                    }
                  }}
                >
                  Thử lại
                </Button>
                <Button size="sm" variant="secondary" onClick={() => setGuideModalOpen(true)}>
                  Mở hướng dẫn
                </Button>
              </div>
            </Callout>
          )}

          {error && (
            <Callout variant="danger" style={{ marginBottom: "var(--cs-space-3)" }}>
              {error}
            </Callout>
          )}

          {oauthSuccess && (
            <Callout variant="success" style={{ marginBottom: "var(--cs-space-3)" }}>
              {oauthSuccess}
            </Callout>
          )}

          {/* STEP 1: Edit & Metadata */}
          {step === "edit" && (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
              {/* Target Selector */}
              <div>
                <Select
                  label="Nền tảng xuất bản"
                  options={TARGET_OPTIONS}
                  value={target}
                  onChange={(e) => {
                    const nextTarget = e.target.value as PublishTarget;
                    setTarget(nextTarget);
                    setChannel(nextTarget);
                  }}
                />
              </div>

              {/* YouTube Channel Selector */}
              {target === "youtube" && (
                <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                  {youtubeChannels.length > 0 ? (
                    <Select
                      label="Kênh YouTube đăng bài"
                      options={youtubeChannels.map((c) => ({
                        value: c.id,
                        label: `${c.title} (${c.handle ? "@" + c.handle : c.id})`,
                      }))}
                      value={selectedChannelId || (youtubeChannels[0]?.id ?? "")}
                      onChange={(e) => setSelectedChannelId(e.target.value)}
                    />
                  ) : (
                    <Callout variant="warning">
                      Chưa có kênh YouTube nào được kết nối. Hãy bấm &quot;Kết nối YouTube&quot; ở bên dưới để liên kết kênh của bạn.
                    </Callout>
                  )}
                </div>
              )}

              {/* Notice for non-YouTube targets */}
              {target !== "youtube" && (
                <Callout variant="info">
                  Chưa hỗ trợ API trực tiếp cho {TARGET_VI[target]}. Hệ thống sẽ tạo gói xuất bản
                  thư mục (video/ảnh, thumbnail, metadata.json và caption.txt) để bạn tải lên thủ công.
                </Callout>
              )}

              {/* Title with live counter */}
              <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <label
                    style={{
                      fontSize: "var(--cs-text-xs)",
                      fontWeight: 600,
                      color: "var(--cs-fg-secondary)",
                    }}
                  >
                    Tiêu đề bài đăng
                  </label>
                  <span
                    style={{
                      fontSize: "var(--cs-text-xs)",
                      fontFamily: "var(--cs-font-mono)",
                      color: titleOverflow ? "var(--cs-danger)" : "var(--cs-fg-muted)",
                      fontWeight: titleOverflow ? 700 : 400,
                    }}
                  >
                    {title.length} / {limits.titleMax} ký tự
                  </span>
                </div>
                <Input
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder="Nhập tiêu đề xuất bản..."
                />
              </div>

              {/* Description with live counter */}
              <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <label
                    style={{
                      fontSize: "var(--cs-text-xs)",
                      fontWeight: 600,
                      color: "var(--cs-fg-secondary)",
                    }}
                  >
                    Mô tả / Nội dung
                  </label>
                  <span
                    style={{
                      fontSize: "var(--cs-text-xs)",
                      fontFamily: "var(--cs-font-mono)",
                      color: descOverflow ? "var(--cs-danger)" : "var(--cs-fg-muted)",
                      fontWeight: descOverflow ? 700 : 400,
                    }}
                  >
                    {description.length} / {limits.descMax} ký tự
                  </span>
                </div>
                <textarea
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  rows={4}
                  placeholder="Nhập mô tả bài đăng..."
                  style={{
                    padding: "var(--cs-space-2) var(--cs-space-3)",
                    borderRadius: "var(--cs-radius-control)",
                    border: "1px solid var(--cs-border-subtle)",
                    backgroundColor: "var(--cs-bg-panel)",
                    color: "var(--cs-fg-primary)",
                    fontFamily: "var(--cs-font-ui)",
                    fontSize: "var(--cs-text-sm)",
                    outline: "none",
                    resize: "vertical",
                  }}
                />
              </div>

              {/* Tags with live counter */}
              <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <label
                    style={{
                      fontSize: "var(--cs-text-xs)",
                      fontWeight: 600,
                      color: "var(--cs-fg-secondary)",
                    }}
                  >
                    Thẻ / Hashtags (cách nhau bởi dấu phẩy)
                  </label>
                  <span
                    style={{
                      fontSize: "var(--cs-text-xs)",
                      fontFamily: "var(--cs-font-mono)",
                      color: tagsOverflow ? "var(--cs-danger)" : "var(--cs-fg-muted)",
                      fontWeight: tagsOverflow ? 700 : 400,
                    }}
                  >
                    {currentTags.length} / {limits.hashtagsMax} thẻ
                  </span>
                </div>
                <Input
                  value={tagsInput}
                  onChange={(e) => setTagsInput(e.target.value)}
                  placeholder="ví dụ: review, công nghệ, hướng dẫn"
                />
              </div>

              {/* Privacy Select */}
              <div>
                <Select
                  label="Chế độ hiển thị (Quyền riêng tư)"
                  options={PRIVACY_OPTIONS}
                  value={privacy}
                  onChange={(e) => setPrivacy(e.target.value as PublishPrivacy)}
                />
              </div>

              {/* Privacy Warning for YouTube public / unlisted */}
              {target === "youtube" && (privacy === "public" || privacy === "unlisted") && (
                <Callout variant="warning">
                  Project Google chưa qua kiểm duyệt API có thể bị YouTube giữ video ở chế độ Riêng tư.
                </Callout>
              )}

              {/* Thumbnail Input */}
              <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                <label
                  style={{
                    fontSize: "var(--cs-text-xs)",
                    fontWeight: 600,
                    color: "var(--cs-fg-secondary)",
                  }}
                >
                  Ảnh thu nhỏ (Thumbnail, tuỳ chọn)
                </label>
                <Input
                  value={thumbnail}
                  onChange={(e) => setThumbnail(e.target.value)}
                  placeholder="Đường dẫn tệp ảnh thu nhỏ (PNG, JPG)..."
                />
              </div>

              {/* Findings list if present */}
              {findings.length > 0 && (
                <div
                  style={{
                    display: "flex",
                    flexDirection: "column",
                    gap: 6,
                    padding: "var(--cs-space-3)",
                    backgroundColor: "var(--cs-bg-raised)",
                    border: "1px solid var(--cs-border-subtle)",
                    borderRadius: "var(--cs-radius-control)",
                  }}
                >
                  <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 700, color: "var(--cs-fg-secondary)" }}>
                    Kết quả kiểm tra tiêu chuẩn ({findings.length})
                  </span>
                  {findings.map((f, i) => (
                    <div
                      key={i}
                      style={{
                        display: "flex",
                        alignItems: "flex-start",
                        gap: 6,
                        fontSize: "var(--cs-text-xs)",
                      }}
                    >
                      <Badge
                        variant={
                          f.severity === "error"
                            ? "danger"
                            : f.severity === "warn"
                            ? "warning"
                            : "info"
                        }
                      >
                        {f.severity === "error"
                          ? "Lỗi"
                          : f.severity === "warn"
                          ? "Cảnh báo"
                          : "Lưu ý"}
                      </Badge>
                      <div style={{ flex: 1 }}>
                        <span style={{ color: "var(--cs-fg-primary)" }}>{f.message_vi}</span>
                        {f.fix_hint_vi && (
                          <div style={{ color: "var(--cs-fg-muted)", marginTop: 2 }}>
                            Gợi ý: {f.fix_hint_vi}
                          </div>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              )}

              {/* YouTube Connect assistance */}
              {target === "youtube" && (
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    padding: "var(--cs-space-2) var(--cs-space-3)",
                    border: "1px solid var(--cs-border-subtle)",
                    backgroundColor: "var(--cs-bg-raised)",
                    borderRadius: "var(--cs-radius-control)",
                  }}
                >
                  <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                    Tài khoản YouTube chưa kết nối hoặc hết hạn token?
                  </div>
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={handleConnectYouTube}
                    disabled={busy}
                  >
                    Kết nối YouTube (OAuth)
                  </Button>
                </div>
              )}
            </div>
          )}

          {/* STEP 2: Second Confirmation Step */}
          {step === "confirm" && (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
              <Callout variant="warning">
                Vui lòng kiểm tra kỹ danh sách tệp và quyền riêng tư trước khi xác nhận. Sau khi gửi,
                hệ thống sẽ tải lên trực tiếp và <strong>không tự động thử lại</strong> nếu gặp lỗi
                nhằm bảo đảm an toàn dữ liệu.
              </Callout>

              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: 8,
                  padding: "var(--cs-space-3)",
                  backgroundColor: "var(--cs-bg-raised)",
                  borderRadius: "var(--cs-radius-control)",
                  border: "1px solid var(--cs-border-subtle)",
                  fontSize: "var(--cs-text-xs)",
                }}
              >
                <div>
                  <strong style={{ color: "var(--cs-fg-secondary)" }}>Nền tảng: </strong>
                  <span style={{ color: "var(--cs-fg-primary)" }}>{TARGET_VI[target]}</span>
                </div>
                {target === "youtube" && (
                  <div>
                    <strong style={{ color: "var(--cs-fg-secondary)" }}>Kênh YouTube: </strong>
                    <span style={{ color: "var(--cs-fg-primary)" }}>
                      {youtubeChannels.find((c) => c.id === selectedChannelId)?.title || selectedChannelId || "Mặc định"}
                    </span>
                  </div>
                )}
                <div>
                  <strong style={{ color: "var(--cs-fg-secondary)" }}>Tiêu đề: </strong>
                  <span style={{ color: "var(--cs-fg-primary)" }}>{title}</span>
                </div>
                <div>
                  <strong style={{ color: "var(--cs-fg-secondary)" }}>Quyền riêng tư: </strong>
                  <Badge variant={privacy === "public" ? "success" : "neutral"}>
                    {PRIVACY_VI[privacy]}
                  </Badge>
                </div>
                <div>
                  <strong style={{ color: "var(--cs-fg-secondary)" }}>Thẻ ({currentTags.length}): </strong>
                  <span style={{ color: "var(--cs-fg-primary)" }}>
                    {currentTags.length > 0 ? currentTags.join(", ") : "Không có thẻ"}
                  </span>
                </div>
                <div>
                  <strong style={{ color: "var(--cs-fg-secondary)" }}>Tệp tải lên / xuất bản ({files.length}): </strong>
                  {files.length === 0 ? (
                    <span style={{ color: "var(--cs-fg-muted)" }}>Tệp render mới nhất</span>
                  ) : (
                    <ul style={{ margin: "4px 0 0 16px", padding: 0 }}>
                      {files.map((file, idx) => (
                        <li key={idx} style={{ fontFamily: "var(--cs-font-mono)", color: "var(--cs-accent)" }}>
                          {file}
                        </li>
                      ))}
                    </ul>
                  )}
                </div>
              </div>

              {target === "youtube" && (privacy === "public" || privacy === "unlisted") && (
                <Callout variant="warning">
                  Project Google chưa qua kiểm duyệt API có thể bị YouTube giữ video ở chế độ Riêng tư.
                </Callout>
              )}
            </div>
          )}

          {/* STEP 3: In-Progress Publishing */}
          {step === "publishing" && (
            <div
              style={{
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                justifyContent: "center",
                padding: "var(--cs-space-5) 0",
                gap: "var(--cs-space-3)",
              }}
            >
              <Spinner size={32} />
              <div style={{ fontSize: "var(--cs-text-md)", fontWeight: 600, color: "var(--cs-fg-primary)" }}>
                {target === "youtube" ? "Đang tải lên YouTube..." : "Đang tạo gói xuất bản..."}
              </div>
              <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                {publishMessage || "Quá trình đang được xử lý trong nền..."}
              </div>
              {publishProgress !== null && (
                <div style={{ width: "80%", display: "flex", flexDirection: "column", gap: 4 }}>
                  <div
                    style={{
                      height: 6,
                      backgroundColor: "var(--cs-bg-raised)",
                      borderRadius: 3,
                      overflow: "hidden",
                      border: "1px solid var(--cs-border-subtle)",
                    }}
                  >
                    <div
                      style={{
                        height: "100%",
                        width: `${publishProgress}%`,
                        backgroundColor: "var(--cs-accent)",
                        transition: "width 0.3s ease",
                      }}
                    />
                  </div>
                  <div style={{ textAlign: "right", fontSize: 11, color: "var(--cs-fg-muted)" }}>
                    {publishProgress}%
                  </div>
                </div>
              )}
            </div>
          )}

          {/* STEP 4: Result */}
          {step === "result" && (
            <div
              style={{
                display: "flex",
                flexDirection: "column",
                gap: "var(--cs-space-3)",
                padding: "var(--cs-space-3) 0",
              }}
            >
              <Callout variant="success">
                {target === "youtube"
                  ? "Đã hoàn thành đăng tải video lên kênh YouTube của bạn!"
                  : "Đã tạo thành công gói xuất bản nội dung!"}
              </Callout>

              {publishResultUrl && publishResultUrl.startsWith("http") && (
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    padding: "var(--cs-space-3)",
                    backgroundColor: "var(--cs-bg-raised)",
                    border: "1px solid var(--cs-border-subtle)",
                    borderRadius: "var(--cs-radius-control)",
                  }}
                >
                  <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                    Liên kết video:
                  </span>
                  <a
                    href={publishResultUrl}
                    target="_blank"
                    rel="noreferrer"
                    style={{
                      fontSize: "var(--cs-text-sm)",
                      fontWeight: 600,
                      color: "var(--cs-accent)",
                      textDecoration: "underline",
                    }}
                  >
                    {publishResultUrl}
                  </a>
                </div>
              )}

              {publishResultUrl === "exported" && (
                <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                  Thư mục xuất bản đã được lưu tại <code>projects/{projectSlug}/publish/{currentRequest?.id}</code> và được mở trên trình quản lý tệp của bạn.
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer Actions */}
        <div
          style={{
            padding: "var(--cs-space-3) var(--cs-space-4)",
            borderTop: "1px solid var(--cs-border-subtle)",
            backgroundColor: "var(--cs-bg-raised)",
            display: "flex",
            justifyContent: "flex-end",
            alignItems: "center",
            gap: "var(--cs-space-2)",
          }}
        >
          {step === "edit" && (
            <>
              <Button size="sm" variant="secondary" onClick={handleCancel}>
                Hủy
              </Button>
              <Button
                size="sm"
                variant="primary"
                onClick={handleProceedToConfirm}
                disabled={busy || !title.trim()}
              >
                Tiếp tục xác nhận
              </Button>
            </>
          )}

          {step === "confirm" && (
            <>
              <Button size="sm" variant="secondary" onClick={() => setStep("edit")} disabled={busy}>
                Quay lại chỉnh sửa
              </Button>
              <Button
                size="sm"
                variant="primary"
                onClick={handleConfirmPublish}
                disabled={busy}
              >
                {target === "youtube"
                  ? "Xác nhận đăng lên YouTube"
                  : "Xác nhận xuất gói"}
              </Button>
            </>
          )}

          {step === "publishing" && (
            <Button size="sm" variant="secondary" disabled>
              Đang xuất bản...
            </Button>
          )}

          {step === "result" && (
            <Button size="sm" variant="primary" onClick={onClose}>
              Hoàn tất
            </Button>
          )}
        </div>
      </div>

      <YouTubeGuideModal open={guideModalOpen} onClose={() => setGuideModalOpen(false)} />
    </div>
  );
};
