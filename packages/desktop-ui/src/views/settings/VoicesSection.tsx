import React from "react";
import {
  Badge,
  Button,
  Callout,
  Dialog,
  ErrorState,
  Select,
  Spinner,
} from "@studio/design-system";
import { Icon } from "../../components/Icon";
import { studioApi } from "../../api/client";
import type {
  TtsStatusResponse,
  TtsModelPreview,
  TtsPreviewResponse,
} from "../../api/types";

export const VoicesSection: React.FC = () => {
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
  const [ttsStatus, setTtsStatus] = React.useState<TtsStatusResponse | null>(null);

  // Default selection state
  const [defaultProvider, setDefaultProvider] = React.useState<string>("tts.vieneu");
  const [defaultVoice, setDefaultVoice] = React.useState<string>("vi_VN-vais1000-medium");
  const [saveSuccess, setSaveSuccess] = React.useState<string | null>(null);

  // Model download confirmation dialog state
  const [installModalOpen, setInstallModalOpen] = React.useState(false);
  const [installPreview, setInstallPreview] = React.useState<TtsModelPreview | null>(null);
  const [installBusy, setInstallBusy] = React.useState(false);
  const [installError, setInstallError] = React.useState<string | null>(null);
  const [installSuccess, setInstallSuccess] = React.useState<string | null>(null);

  // Audio testing / "Nghe thử" state
  const [previewText, setPreviewText] = React.useState(
    "Xin chào! Đây là bản nghe thử giọng đọc tiếng Việt của Local Content Studio."
  );
  const [testProvider, setTestProvider] = React.useState<string>("tts.piper");
  const [testVoice, setTestVoice] = React.useState<string>("vi_VN-vais1000-medium");
  const [testSpeed, setTestSpeed] = React.useState<number>(1.0);
  const [testBusy, setTestBusy] = React.useState(false);
  const [testError, setTestError] = React.useState<string | null>(null);
  const [testResult, setTestResult] = React.useState<TtsPreviewResponse | null>(null);

  const loadData = React.useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const status = await studioApi.ttsStatus();
      setTtsStatus(status);
      if (status.defaultProvider) {
        setDefaultProvider(status.defaultProvider);
        setTestProvider(status.defaultProvider);
      }
      if (status.defaultVoice) {
        setDefaultVoice(status.defaultVoice);
        setTestVoice(status.defaultVoice);
      }
    } catch (err: any) {
      setError(err?.message || "Không thể tải trạng thái hệ thống TTS.");
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    loadData();
  }, [loadData]);

  const handleOpenInstallPreview = async (provider: string, voice: string) => {
    setInstallModalOpen(true);
    setInstallPreview(null);
    setInstallError(null);
    setInstallBusy(true);
    try {
      const preview = await studioApi.ttsModelPreview(provider, voice);
      setInstallPreview(preview);
    } catch (err: any) {
      setInstallError(err?.message || "Không thể lấy thông tin gói tải về.");
    } finally {
      setInstallBusy(false);
    }
  };

  const handleConfirmInstall = async () => {
    if (!installPreview) return;
    setInstallBusy(true);
    setInstallError(null);
    try {
      const res = await studioApi.ttsModelInstall(
        installPreview.provider,
        installPreview.voice
      );
      setInstallSuccess(res.message);
      setInstallModalOpen(false);
      setInstallPreview(null);
      setTimeout(() => setInstallSuccess(null), 4000);
      loadData();
    } catch (err: any) {
      setInstallError(err?.message || "Lỗi trong quá trình tải và cài đặt.");
    } finally {
      setInstallBusy(false);
    }
  };

  const handleSaveConfig = async (p: string, v: string) => {
    try {
      await studioApi.ttsSetConfig({
        defaultProvider: p,
        defaultVoice: v,
      });
      setSaveSuccess("Đã lưu thiết lập giọng đọc mặc định.");
      setTimeout(() => setSaveSuccess(null), 3000);
      loadData();
    } catch (err: any) {
      alert(`Lỗi lưu thiết lập: ${err?.message || err}`);
    }
  };

  const handleSynthesizePreview = async () => {
    if (!previewText.trim()) return;
    setTestBusy(true);
    setTestError(null);
    setTestResult(null);
    try {
      const res = await studioApi.ttsSynthesizePreview({
        text: previewText.trim(),
        provider: testProvider,
        voice: testVoice,
        speed: testSpeed,
      });
      setTestResult(res);
    } catch (err: any) {
      setTestError(err?.message || "Lỗi khi tổng hợp âm thanh nghe thử.");
    } finally {
      setTestBusy(false);
    }
  };

  if (loading && !ttsStatus) {
    return (
      <div style={{ padding: "var(--cs-space-8)", textAlign: "center" }}>
        <Spinner size={24} />
        <p style={{ marginTop: "var(--cs-space-2)", color: "var(--cs-fg-secondary)" }}>
          Đang kiểm tra các hệ thống giọng đọc tiếng Việt...
        </p>
      </div>
    );
  }

  if (error) {
    return (
      <ErrorState
        title="Không thể tải thiết lập giọng đọc"
        reason={error}
        safetyNote="Cần kết nối với Studio Core để kiểm tra các provider TTS cục bộ."
        onRetry={loadData}
      />
    );
  }

  const vieneuProvider = ttsStatus?.providers.find((p) => p.id === "tts.vieneu");
  const piperProvider = ttsStatus?.providers.find((p) => p.id === "tts.piper");
  const sapiProvider = ttsStatus?.providers.find((p) => p.id === "tts.windows-sapi");

  // Collect all available voices for the picker
  const allVoiceOptions: { value: string; label: string }[] = [];
  ttsStatus?.providers.forEach((prov) => {
    if (prov.available) {
      prov.voices.forEach((v) => {
        if (v.downloaded) {
          allVoiceOptions.push({
            value: v.id,
            label: `${v.name} (${prov.name})`,
          });
        }
      });
    }
  });

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-6)" }}>
      {/* Intro callout */}
      <Callout variant="neutral" title="Tổng hợp giọng nói tiếng Việt (Offline & Miễn phí)">
        Studio hỗ trợ 3 hệ thống đọc văn bản ngoại tuyến (TTS) được tối ưu cho tiếng Việt:
        <strong> VieNeu-TTS v3 Turbo</strong> (chất lượng cao, CPU ONNX),
        <strong> Piper TTS</strong> (nhẹ, nhanh, giọng trung tính/tự nhiên) và
        <strong> Windows SAPI</strong> (tích hợp sẵn trong Windows).
      </Callout>

      {installSuccess && (
        <Callout variant="success" title="Cài đặt thành công">
          {installSuccess}
        </Callout>
      )}

      {saveSuccess && (
        <Callout variant="success" title="Thiết lập đã lưu">
          {saveSuccess}
        </Callout>
      )}

      {/* SECTION 1: DEFAULT VOICE PICKER */}
      <div
        style={{
          backgroundColor: "var(--cs-bg-panel)",
          border: "1px solid var(--cs-border-subtle)",
          padding: "var(--cs-space-4)",
          display: "flex",
          flexDirection: "column",
          gap: "var(--cs-space-3)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <div>
            <h2 style={{ margin: 0, fontSize: "var(--cs-text-base)", fontWeight: 600 }}>
              Giọng đọc mặc định cho tiếng Việt
            </h2>
            <p style={{ margin: "2px 0 0 0", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
              Sử dụng tự động khi kịch bản hoặc trợ lý AI yêu cầu đọc văn bản tiếng Việt.
            </p>
          </div>
        </div>

        <div style={{ display: "flex", gap: "var(--cs-space-3)", alignItems: "center" }}>
          <div style={{ width: 200 }}>
            <Select
              label="Hệ thống (Provider)"
              value={defaultProvider}
              onChange={(e) => {
                const nextP = e.target.value;
                setDefaultProvider(nextP);
                // Choose first available voice in this provider
                const targetProv = ttsStatus?.providers.find((p) => p.id === nextP);
                const firstVoice = targetProv?.voices.find((v) => v.downloaded)?.id || "";
                if (firstVoice) setDefaultVoice(firstVoice);
                handleSaveConfig(nextP, firstVoice || defaultVoice);
              }}
              options={[
                { value: "tts.vieneu", label: "VieNeu-TTS (v3 Turbo)" },
                { value: "tts.piper", label: "Piper TTS" },
                { value: "tts.windows-sapi", label: "Windows SAPI (Hệ thống)" },
              ]}
            />
          </div>

          <div style={{ flex: 1 }}>
            <Select
              label="Giọng đọc (Voice)"
              value={defaultVoice}
              onChange={(e) => {
                const nextV = e.target.value;
                setDefaultVoice(nextV);
                handleSaveConfig(defaultProvider, nextV);
              }}
              options={
                allVoiceOptions.length > 0
                  ? allVoiceOptions
                  : [{ value: defaultVoice, label: defaultVoice }]
              }
            />
          </div>
        </div>
      </div>

      {/* SECTION 2: PROVIDER CARDS */}
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
        <h2 style={{ margin: 0, fontSize: "var(--cs-text-base)", fontWeight: 600 }}>
          Trạng thái & Quản lý các bộ đọc
        </h2>

        {/* PROVIDER 1: VieNeu-TTS */}
        <div
          style={{
            backgroundColor: "var(--cs-bg-panel)",
            border: "1px solid var(--cs-border-subtle)",
            padding: "var(--cs-space-4)",
            display: "flex",
            flexDirection: "column",
            gap: "var(--cs-space-3)",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)" }}>
              <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>
                VieNeu-TTS v3 Turbo
              </span>
              <Badge variant={vieneuProvider?.available ? "success" : "neutral"}>
                {vieneuProvider?.available ? "Sẵn sàng" : "Chưa cài đặt"}
              </Badge>
              <Badge variant="neutral">Apache-2.0</Badge>
            </div>
          </div>

          <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
            Mô hình TTS tự nhiên chất lượng 48kHz chạy ONNX tối ưu trên CPU, hỗ trợ đọc ngữ điệu tiếng Việt và nhân bản giọng nói ngắn (voice cloning).
          </p>

          <div
            style={{
              padding: "var(--cs-space-2) var(--cs-space-3)",
              backgroundColor: "var(--cs-bg-canvas)",
              border: "1px solid var(--cs-border-subtle)",
              fontSize: "var(--cs-text-xs)",
              color: vieneuProvider?.available ? "var(--cs-success)" : "var(--cs-fg-muted)",
            }}
          >
            {vieneuProvider?.noteVi}
          </div>

          {!vieneuProvider?.available && (
            <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
              Hướng dẫn cài đặt nhanh trong terminal:
              <pre
                style={{
                  margin: "4px 0 0 0",
                  padding: "6px 8px",
                  backgroundColor: "var(--cs-bg-canvas)",
                  border: "1px solid var(--cs-border-subtle)",
                  borderRadius: 2,
                  fontFamily: "var(--cs-font-mono)",
                  color: "var(--cs-accent)",
                }}
              >
                pip install vieneu
              </pre>
            </div>
          )}

          {vieneuProvider?.available && (
            <div style={{ display: "flex", gap: "var(--cs-space-2)", alignItems: "center", fontSize: "var(--cs-text-xs)" }}>
              <span style={{ color: "var(--cs-fg-secondary)" }}>Các giọng có sẵn:</span>
              {vieneuProvider.voices.map((v) => (
                <span
                  key={v.id}
                  style={{
                    padding: "2px 6px",
                    backgroundColor: "var(--cs-bg-canvas)",
                    border: "1px solid var(--cs-border-subtle)",
                    fontSize: "var(--cs-text-xs)",
                  }}
                >
                  {v.name}
                </span>
              ))}
            </div>
          )}
        </div>

        {/* PROVIDER 2: Piper TTS */}
        <div
          style={{
            backgroundColor: "var(--cs-bg-panel)",
            border: "1px solid var(--cs-border-subtle)",
            padding: "var(--cs-space-4)",
            display: "flex",
            flexDirection: "column",
            gap: "var(--cs-space-3)",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)" }}>
              <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>
                Piper TTS (Fast & Lightweight)
              </span>
              <Badge variant={piperProvider?.available ? "success" : "neutral"}>
                {piperProvider?.available ? "Sẵn sàng" : "Cần cài đặt thêm"}
              </Badge>
              <Badge variant="neutral">MIT / CC-BY 4.0</Badge>
            </div>

            {!piperProvider?.binaryInstalled && (
              <Button
                variant="primary"
                size="sm"
                onClick={() => handleOpenInstallPreview("piper", "binary")}
              >
                <Icon name="download" size={14} style={{ marginRight: 4 }} />
                Cài đặt Piper binary (21.4 MB)
              </Button>
            )}
          </div>

          <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
            Trình tổng hợp giọng nói offline siêu nhẹ, chạy bằng tệp thực thi đơn lẻ và mô hình ONNX tiếng Việt được ghim phiên bản v1.0.0.
          </p>

          <Callout variant="warning" title="Lưu ý điều khoản giọng đọc tiếng Việt">
            Dữ liệu giọng đọc Piper tiếng Việt được cấp phép CC-BY 4.0 nhưng fine-tune từ mô hình gốc dành cho nghiên cứu → <strong>chỉ nên dùng cá nhân/phi thương mại</strong>.
          </Callout>

          {/* List of Piper Vietnamese Voice Models */}
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-2)" }}>
            <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600 }}>
              Danh sách mô hình tiếng Việt đã kiểm duyệt (Hugging Face v1.0.0):
            </span>

            {piperProvider?.voices.map((voice) => (
              <div
                key={voice.id}
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  padding: "var(--cs-space-2) var(--cs-space-3)",
                  backgroundColor: "var(--cs-bg-canvas)",
                  border: "1px solid var(--cs-border-subtle)",
                }}
              >
                <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)" }}>
                    <span style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600 }}>
                      {voice.name}
                    </span>
                    {voice.downloaded ? (
                      <Badge variant="success">Đã tải về</Badge>
                    ) : (
                      <Badge variant="neutral">Chưa tải ({voice.sizeHuman})</Badge>
                    )}
                  </div>
                  <span style={{ fontSize: "11px", color: "var(--cs-fg-muted)" }}>
                    Định dạng: ONNX ({voice.quality}) • SHA-256 đã xác minh
                  </span>
                </div>

                {!voice.downloaded && (
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => handleOpenInstallPreview("piper", voice.id)}
                  >
                    <Icon name="download" size={12} style={{ marginRight: 4 }} />
                    Tải về ({voice.sizeHuman})
                  </Button>
                )}
              </div>
            ))}
          </div>
        </div>

        {/* PROVIDER 3: Windows SAPI */}
        <div
          style={{
            backgroundColor: "var(--cs-bg-panel)",
            border: "1px solid var(--cs-border-subtle)",
            padding: "var(--cs-space-4)",
            display: "flex",
            flexDirection: "column",
            gap: "var(--cs-space-3)",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <div style={{ display: "flex", alignItems: "center", gap: "var(--cs-space-2)" }}>
              <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>
                Windows SAPI (System.Speech)
              </span>
              <Badge variant={sapiProvider?.available ? "success" : "neutral"}>
                {sapiProvider?.available ? "Sẵn sàng" : "Không khả dụng"}
              </Badge>
              <Badge variant="neutral">Hệ thống Windows</Badge>
            </div>
          </div>

          <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
            Bộ đọc có sẵn trong Windows (PowerShell System.Speech Synthesizer). Đóng vai trò là phương án dự phòng mặc định khi máy chưa cài đặt VieNeu hoặc Piper.
          </p>

          <div style={{ display: "flex", gap: "var(--cs-space-2)", alignItems: "center", fontSize: "var(--cs-text-xs)" }}>
            <span style={{ color: "var(--cs-fg-secondary)" }}>Các giọng SAPI phát hiện được:</span>
            {sapiProvider?.voices.map((v) => (
              <span
                key={v.id}
                style={{
                  padding: "2px 6px",
                  backgroundColor: "var(--cs-bg-canvas)",
                  border: "1px solid var(--cs-border-subtle)",
                  fontSize: "var(--cs-text-xs)",
                }}
              >
                {v.name}
              </span>
            ))}
          </div>
        </div>
      </div>

      {/* SECTION 3: AUDIO PREVIEW / "NGHE THỬ" */}
      <div
        style={{
          backgroundColor: "var(--cs-bg-panel)",
          border: "1px solid var(--cs-border-subtle)",
          padding: "var(--cs-space-4)",
          display: "flex",
          flexDirection: "column",
          gap: "var(--cs-space-3)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
          <div>
            <h2 style={{ margin: 0, fontSize: "var(--cs-text-base)", fontWeight: 600 }}>
              Nghe thử giọng đọc (Preview)
            </h2>
            <p style={{ margin: "2px 0 0 0", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
              Nhập đoạn văn bản tiếng Việt để tổng hợp âm thanh WAV trực tiếp và kiểm tra chất lượng giọng đọc.
            </p>
          </div>
        </div>

        <div>
          <textarea
            value={previewText}
            onChange={(e) => setPreviewText(e.target.value)}
            rows={3}
            style={{
              width: "100%",
              padding: "var(--cs-space-2) var(--cs-space-3)",
              backgroundColor: "var(--cs-bg-canvas)",
              border: "1px solid var(--cs-border-subtle)",
              color: "var(--cs-fg-primary)",
              fontSize: "var(--cs-text-sm)",
              borderRadius: "var(--cs-radius-sm)",
              resize: "vertical",
              outline: "none",
            }}
            placeholder="Nhập nội dung tiếng Việt cần nghe thử..."
          />
        </div>

        <div style={{ display: "flex", gap: "var(--cs-space-4)", alignItems: "flex-end", flexWrap: "wrap" }}>
          <div style={{ width: 180 }}>
            <Select
              label="Bộ đọc"
              value={testProvider}
              onChange={(e) => {
                const nextP = e.target.value;
                setTestProvider(nextP);
                const targetProv = ttsStatus?.providers.find((p) => p.id === nextP);
                const firstVoice = targetProv?.voices.find((v) => v.downloaded)?.id || "";
                if (firstVoice) setTestVoice(firstVoice);
              }}
              options={[
                { value: "tts.vieneu", label: "VieNeu-TTS" },
                { value: "tts.piper", label: "Piper TTS" },
                { value: "tts.windows-sapi", label: "Windows SAPI" },
              ]}
            />
          </div>

          <div style={{ width: 220 }}>
            <Select
              label="Giọng đọc"
              value={testVoice}
              onChange={(e) => setTestVoice(e.target.value)}
              options={
                ttsStatus?.providers
                  .find((p) => p.id === testProvider)
                  ?.voices.map((v) => ({
                    value: v.id,
                    label: `${v.name}${v.downloaded ? "" : " (Chưa tải)"}`,
                  })) || []
              }
            />
          </div>

          <div style={{ width: 140 }}>
            <label style={{ display: "block", fontSize: "var(--cs-text-xs)", marginBottom: 4, color: "var(--cs-fg-secondary)" }}>
              Tốc độ: {testSpeed.toFixed(1)}x
            </label>
            <input
              type="range"
              min="0.5"
              max="2.0"
              step="0.1"
              value={testSpeed}
              onChange={(e) => setTestSpeed(parseFloat(e.target.value))}
              style={{ width: "100%" }}
            />
          </div>

          <Button
            variant="primary"
            size="md"
            onClick={handleSynthesizePreview}
            disabled={testBusy || !previewText.trim()}
          >
            {testBusy ? (
              <>
                <Spinner size={14} />
                <span style={{ marginLeft: 6 }}>Đang tổng hợp...</span>
              </>
            ) : (
              <>
                <Icon name="play" size={14} style={{ marginRight: 6 }} />
                Tổng hợp & Nghe thử
              </>
            )}
          </Button>
        </div>

        {testError && (
          <div style={{ color: "var(--cs-danger)", fontSize: "var(--cs-text-xs)" }}>
            {testError}
          </div>
        )}

        {testResult && (
          <div
            style={{
              marginTop: "var(--cs-space-2)",
              padding: "var(--cs-space-3)",
              backgroundColor: "var(--cs-bg-canvas)",
              border: "1px solid var(--cs-border-subtle)",
              display: "flex",
              flexDirection: "column",
              gap: "var(--cs-space-2)",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <div style={{ display: "flex", gap: "var(--cs-space-2)", alignItems: "center" }}>
                <Badge variant="success">Hoàn thành</Badge>
                <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                  Provider: <strong>{testResult.provider}</strong> • Giọng: <strong>{testResult.voice}</strong>
                </span>
              </div>
              <span style={{ fontSize: "11px", color: "var(--cs-fg-muted)" }}>
                Kích thước: {(testResult.sizeBytes / 1024).toFixed(1)} KB
              </span>
            </div>

            <audio
              controls
              src={testResult.audioUrl}
              autoPlay
              style={{ width: "100%", height: 36, marginTop: 4 }}
            />

            {testResult.note && (
              <div style={{ fontSize: "11px", color: "var(--cs-fg-muted)" }}>
                {testResult.note}
              </div>
            )}
          </div>
        )}
      </div>

      {/* CONFIRMATION PREVIEW MODAL */}
      <Dialog
        open={installModalOpen}
        title="Xác nhận tải về & cài đặt mô hình"
        onClose={() => {
          if (!installBusy) {
            setInstallModalOpen(false);
            setInstallPreview(null);
          }
        }}
        actions={
          <>
            <Button
              variant="secondary"
              onClick={() => {
                setInstallModalOpen(false);
                setInstallPreview(null);
              }}
              disabled={installBusy}
            >
              Hủy
            </Button>
            <Button
              variant="primary"
              onClick={handleConfirmInstall}
              disabled={installBusy || !installPreview}
            >
              {installBusy ? (
                <>
                  <Spinner size={14} />
                  <span style={{ marginLeft: 6 }}>Đang tải & xác minh SHA-256...</span>
                </>
              ) : (
                "Xác nhận tải về"
              )}
            </Button>
          </>
        }
      >
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
          {installBusy && !installPreview && (
            <div style={{ textAlign: "center", padding: "var(--cs-space-4)" }}>
              <Spinner size={16} />
              <p style={{ fontSize: "var(--cs-text-xs)", marginTop: 8 }}>
                Đang kiểm tra thông tin gói từ máy chủ...
              </p>
            </div>
          )}

          {installError && (
            <Callout variant="danger" title="Lỗi">
              {installError}
            </Callout>
          )}

          {installPreview && (
            <>
              <div>
                <strong style={{ fontSize: "var(--cs-text-sm)" }}>{installPreview.name}</strong>
                <p style={{ margin: "2px 0 0 0", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                  Gói: {installPreview.provider} / {installPreview.voice}
                </p>
              </div>

              <div
                style={{
                  padding: "var(--cs-space-3)",
                  backgroundColor: "var(--cs-bg-canvas)",
                  border: "1px solid var(--cs-border-subtle)",
                  display: "flex",
                  flexDirection: "column",
                  gap: "var(--cs-space-2)",
                  fontSize: "var(--cs-text-xs)",
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between" }}>
                  <span style={{ color: "var(--cs-fg-secondary)" }}>Dung lượng tải:</span>
                  <strong>{installPreview.sizeHuman}</strong>
                </div>

                <div style={{ display: "flex", justifyContent: "space-between" }}>
                  <span style={{ color: "var(--cs-fg-secondary)" }}>Giấy phép:</span>
                  <strong>{installPreview.license}</strong>
                </div>

                <div style={{ display: "flex", justifyContent: "space-between" }}>
                  <span style={{ color: "var(--cs-fg-secondary)" }}>Kiểm tra toàn vẹn:</span>
                  <span style={{ color: "var(--cs-success)" }}>SHA-256 pinned v1.0.0</span>
                </div>
              </div>

              <Callout variant="warning" title="Điều khoản sử dụng">
                {installPreview.licenseNote}
              </Callout>

              <p style={{ margin: 0, fontSize: "11px", color: "var(--cs-fg-muted)" }}>
                Tệp sẽ được lưu vào thư mục cục bộ của ứng dụng tại %LOCALAPPDATA%/LocalContentStudio và được xác minh mã băm SHA-256 trước khi kích hoạt.
              </p>
            </>
          )}
        </div>
      </Dialog>
    </div>
  );
};
