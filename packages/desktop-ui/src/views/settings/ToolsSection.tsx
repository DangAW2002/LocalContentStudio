import React from "react";
import { Icon } from "../../components/Icon";
import { Badge, Button, Callout, EmptyState, Input } from "@studio/design-system";
import { studioApi } from "../../api/client";
import type {
  InstalledToolInfo,
  ToolInstallPreview,
  ToolInstallScope,
} from "../../api/types";

export const ToolsSection: React.FC = () => {
  const [installedTools, setInstalledTools] = React.useState<InstalledToolInfo[]>([]);
  const [loading, setLoading] = React.useState(true);
  const [installModalOpen, setInstallModalOpen] = React.useState<"folder" | "url" | null>(null);
  const [installSourceInput, setInstallSourceInput] = React.useState("");
  const [installScopeInput, setInstallScopeInput] = React.useState<ToolInstallScope>("app");
  const [installPreview, setInstallPreview] = React.useState<ToolInstallPreview | null>(null);
  const [installBusy, setInstallBusy] = React.useState(false);
  const [installError, setInstallError] = React.useState<string | null>(null);
  const [actionSuccess, setActionSuccess] = React.useState<string | null>(null);

  const loadTools = React.useCallback(async () => {
    setLoading(true);
    try {
      const list = await studioApi.toolListInstalled();
      setInstalledTools(list);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    loadTools();
  }, [loadTools]);

  const handleOpenInstallModal = (type: "folder" | "url") => {
    setInstallModalOpen(type);
    setInstallSourceInput("");
    setInstallScopeInput("app");
    setInstallPreview(null);
    setInstallError(null);
  };

  const handleCloseInstallModal = () => {
    setInstallModalOpen(null);
    setInstallPreview(null);
    setInstallError(null);
    setInstallBusy(false);
  };

  const handlePreviewInstall = async () => {
    const src = installSourceInput.trim();
    if (!src) {
      setInstallError("Vui lòng nhập đường dẫn thư mục hoặc địa chỉ URL");
      return;
    }
    setInstallBusy(true);
    setInstallError(null);
    try {
      const preview = await studioApi.toolInstallPreview({
        source: src,
        scope: installScopeInput,
      });
      setInstallPreview(preview);
    } catch (err: any) {
      setInstallError(err?.message || "Lỗi khi kiểm tra gói công cụ");
    } finally {
      setInstallBusy(false);
    }
  };

  const handleConfirmInstall = async () => {
    if (!installPreview) return;
    setInstallBusy(true);
    setInstallError(null);
    try {
      const installed = await studioApi.toolInstallConfirm({
        previewId: installPreview.previewId,
      });
      handleCloseInstallModal();
      setActionSuccess(`Đã cài đặt thành công công cụ ${installed.id} (${installed.version})`);
      setTimeout(() => setActionSuccess(null), 4000);
      loadTools();
    } catch (err: any) {
      setInstallError(err?.message || "Lỗi khi hoàn tất cài đặt");
    } finally {
      setInstallBusy(false);
    }
  };

  const handleUninstall = async (tool: InstalledToolInfo) => {
    if (!window.confirm(`Bạn có chắc chắn muốn gỡ bỏ công cụ "${tool.id}" (${tool.version}) khỏi hệ thống?`)) {
      return;
    }
    try {
      await studioApi.toolUninstall({
        toolId: tool.id,
        version: tool.version,
      });
      setActionSuccess(`Đã gỡ bỏ công cụ ${tool.id} thành công`);
      setTimeout(() => setActionSuccess(null), 4000);
      loadTools();
    } catch (err: any) {
      alert(`Lỗi khi gỡ bỏ công cụ: ${err?.message || err}`);
    }
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
      {actionSuccess && (
        <Callout variant="success">
          {actionSuccess}
        </Callout>
      )}

      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <div>
          <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
            Công cụ Hệ thống & Mở rộng
          </h2>
          <p style={{ margin: "2px 0 0", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
            Quản lý các công cụ có sẵn và công cụ cài đặt thêm từ thư mục cục bộ hoặc URL HTTPS.
          </p>
        </div>

        <div style={{ display: "flex", gap: "var(--cs-space-2)" }}>
          <Button
            size="sm"
            variant="secondary"
            onClick={() => handleOpenInstallModal("folder")}
            leftIcon={<Icon name="folder" size={13} />}
          >
            Cài từ thư mục…
          </Button>
          <Button
            size="sm"
            variant="primary"
            onClick={() => handleOpenInstallModal("url")}
            leftIcon={<Icon name="link" size={13} />}
          >
            Cài từ URL…
          </Button>
          <Button
            size="sm"
            variant="secondary"
            onClick={loadTools}
            busy={loading}
            leftIcon={<Icon name="refresh" size={13} />}
          >
            Làm mới
          </Button>
        </div>
      </div>

      {/* Install / Preview Modal */}
      {installModalOpen && (
        <div
          style={{
            padding: "var(--cs-space-4)",
            backgroundColor: "var(--cs-bg-panel)",
            border: "2px solid var(--cs-accent)",
            borderRadius: "var(--cs-radius-control)",
            display: "flex",
            flexDirection: "column",
            gap: "var(--cs-space-4)",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Icon name={installModalOpen === "folder" ? "folder" : "download"} size={18} color="var(--cs-accent)" />
              <h3 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
                {installModalOpen === "folder" ? "Cài đặt công cụ từ thư mục cục bộ" : "Cài đặt công cụ từ URL HTTPS"}
              </h3>
            </div>
            <Button size="sm" variant="secondary" onClick={handleCloseInstallModal}>
              Đóng
            </Button>
          </div>

          {!installPreview ? (
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
              <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                {installModalOpen === "folder"
                  ? "Nhập đường dẫn tuyệt đối đến thư mục chứa tệp cấu hình tool.yaml của công cụ."
                  : "Nhập đường dẫn HTTPS đến tệp nén .zip chứa công cụ (tối đa 100 MB). Hệ thống tự động kiểm tra an toàn chống zip-slip."}
              </p>

              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600 }}>
                  {installModalOpen === "folder" ? "Đường dẫn thư mục:" : "Địa chỉ HTTPS (.zip):"}
                </label>
                <Input
                  value={installSourceInput}
                  onChange={(e: any) => setInstallSourceInput(e.target.value)}
                  placeholder={
                    installModalOpen === "folder"
                      ? "Ví dụ: C:\\Users\\MyTools\\custom-tool"
                      : "Ví dụ: https://example.com/tools/my-tool-1.0.0.zip"
                  }
                />
              </div>

              <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600 }}>Phạm vi cài đặt:</label>
                <div style={{ display: "flex", gap: "var(--cs-space-4)", fontSize: "var(--cs-text-xs)" }}>
                  <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}>
                    <input
                      type="radio"
                      name="toolScope"
                      value="app"
                      checked={installScopeInput === "app"}
                      onChange={() => setInstallScopeInput("app")}
                    />
                    Toàn ứng dụng (App - dùng cho mọi workspace)
                  </label>
                  <label style={{ display: "flex", alignItems: "center", gap: 6, cursor: "pointer" }}>
                    <input
                      type="radio"
                      name="toolScope"
                      value="workspace"
                      checked={installScopeInput === "workspace"}
                      onChange={() => setInstallScopeInput("workspace")}
                    />
                    Riêng Workspace hiện tại (.studio/tools)
                  </label>
                </div>
              </div>

              {installError && <Callout variant="danger">{installError}</Callout>}

              <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 4 }}>
                <Button variant="secondary" onClick={handleCloseInstallModal}>
                  Hủy
                </Button>
                <Button
                  variant="primary"
                  onClick={handlePreviewInstall}
                  busy={installBusy}
                  busyText="Đang kiểm tra..."
                >
                  Kiểm tra & Xem trước
                </Button>
              </div>
            </div>
          ) : (
            /* PREVIEW CONFIRMATION DIALOG */
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
              <Callout variant="warning" title="Đánh giá rủi ro an toàn công cụ">
                Công cụ ngoài có thể chạy các tiến trình trên máy của bạn. Hãy xem xét kỹ các quyền hạn và câu lệnh dưới đây trước khi cấp phép cài đặt.
              </Callout>

              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "140px 1fr",
                  gap: "8px 12px",
                  fontSize: "var(--cs-text-xs)",
                  backgroundColor: "var(--cs-bg-raised)",
                  padding: "var(--cs-space-3)",
                  borderRadius: "var(--cs-radius-control)",
                }}
              >
                <span style={{ color: "var(--cs-fg-muted)" }}>Tên công cụ:</span>
                <span style={{ fontWeight: 700 }}>{installPreview.toolId} (phiên bản: {installPreview.version})</span>

                <span style={{ color: "var(--cs-fg-muted)" }}>Phân loại:</span>
                <span>{installPreview.kind}</span>

                <span style={{ color: "var(--cs-fg-muted)" }}>Mô tả:</span>
                <span>{installPreview.description || "Không có mô tả"}</span>

                <span style={{ color: "var(--cs-fg-muted)" }}>Lệnh thực thi:</span>
                <span style={{ fontFamily: "var(--cs-font-mono)", color: "var(--cs-accent)" }}>
                  {installPreview.command} {installPreview.args.join(" ")}
                </span>

                <span style={{ color: "var(--cs-fg-muted)" }}>Khả năng cung cấp:</span>
                <span>{installPreview.provides.join(", ") || "Không có"}</span>

                <span style={{ color: "var(--cs-fg-muted)" }}>Yêu cầu phụ thuộc:</span>
                <span>{installPreview.needs.join(", ") || "Không yêu cầu"}</span>

                <span style={{ color: "var(--cs-fg-muted)" }}>Mạng / Internet:</span>
                <span>{installPreview.network || "Không yêu cầu truy cập mạng"}</span>

                <span style={{ color: "var(--cs-fg-muted)" }}>Mức độ tác động:</span>
                <span style={{ color: "var(--cs-warning)", fontWeight: 600 }}>{installPreview.sideEffect}</span>

                <span style={{ color: "var(--cs-fg-muted)" }}>Thư mục cài đặt:</span>
                <span style={{ fontFamily: "var(--cs-font-mono)" }}>{installPreview.targetDir}</span>

                {installPreview.isReinstall && (
                  <>
                    <span style={{ color: "var(--cs-danger)", fontWeight: 700 }}>Lưu ý cài đè:</span>
                    <span style={{ color: "var(--cs-danger)" }}>Phiên bản này đã tồn tại, cài đặt sẽ ghi đè phiên bản cũ.</span>
                  </>
                )}
              </div>

              {installError && <Callout variant="danger">{installError}</Callout>}

              <div style={{ display: "flex", justifyContent: "flex-end", gap: 8 }}>
                <Button variant="secondary" onClick={() => setInstallPreview(null)}>
                  Quay lại
                </Button>
                <Button
                  variant="primary"
                  onClick={handleConfirmInstall}
                  busy={installBusy}
                  busyText="Đang cài đặt..."
                >
                  Xác nhận Cài đặt Công cụ
                </Button>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Tool list */}
      {installedTools.length === 0 ? (
        <EmptyState
          icon={<Icon name="cpu" size={28} />}
          title={loading ? "Đang tải danh sách công cụ..." : "Chưa có công cụ nào"}
          description={loading ? undefined : "Không tìm thấy công cụ trong hệ thống. Bấm Làm mới hoặc cài đặt thêm."}
        />
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
          {installedTools.map((tool) => {
            const isBuiltin =
              !tool.version ||
              tool.version === "builtin" ||
              tool.scope?.toLowerCase().includes("builtin") ||
              tool.scope?.toLowerCase().includes("có sẵn");

            const isRealSemver =
              !!tool.version &&
              tool.version !== "builtin" &&
              /^\d+(\.\d+)*(-.+)?$/.test(tool.version.trim());

            const note = (tool.probe.noteVi || tool.probe.note_vi || "").trim();
            let probeText = "";
            if (note.length > 0) {
              probeText = note;
            } else if (tool.probe.missing && tool.probe.missing.length > 0) {
              probeText = `Thiếu: ${tool.probe.missing.join(", ")}`;
            } else if (tool.probe.available) {
              probeText = "Sẵn sàng";
            }

            return (
              <div
                key={`${tool.id}@${tool.version || "builtin"}`}
                style={{
                  padding: "var(--cs-space-4)",
                  backgroundColor: "var(--cs-bg-panel)",
                  border: "1px solid var(--cs-border-subtle)",
                  borderRadius: "var(--cs-radius-control)",
                  display: "flex",
                  flexDirection: "column",
                  gap: 10,
                }}
              >
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <Icon name="tool" size={16} color="var(--cs-accent)" />
                    <span style={{ fontWeight: 700, fontSize: "var(--cs-text-sm)" }}>
                      {tool.id}
                    </span>
                    {isBuiltin ? (
                      <Badge variant="neutral">Có sẵn</Badge>
                    ) : (
                      <>
                        {tool.version && (
                          <span style={{ fontSize: "var(--cs-text-xs)", fontFamily: "var(--cs-font-mono)", color: "var(--cs-fg-muted)" }}>
                            {isRealSemver ? `v${tool.version}` : tool.version}
                          </span>
                        )}
                        <Badge variant="info">
                          {tool.scope}
                        </Badge>
                      </>
                    )}
                    <Badge variant={tool.probe.available ? "success" : "warning"}>
                      {tool.probe.available ? "Sẵn sàng" : "Thiếu phụ thuộc"}
                    </Badge>
                  </div>

                  {tool.canUninstall && (
                    <Button
                      size="sm"
                      variant="secondary"
                      onClick={() => handleUninstall(tool)}
                      leftIcon={<Icon name="trash" size={12} />}
                    >
                      Gỡ bỏ
                    </Button>
                  )}
                </div>

                <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                  {tool.manifest.description || "Không có mô tả"}
                </div>

                <div style={{ display: "flex", flexWrap: "wrap", gap: 12, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                  {tool.manifest.run?.command && (
                    <span>Lệnh: <code style={{ color: "var(--cs-fg-primary)" }}>{tool.manifest.run?.command}</code></span>
                  )}
                  {tool.manifest.provides?.length > 0 && (
                    <span>Cung cấp: {tool.manifest.provides.join(", ")}</span>
                  )}
                  {probeText.length > 0 && (
                    <span>
                      Trạng thái probe:{" "}
                      <strong style={{ color: tool.probe.available ? "var(--cs-success)" : "var(--cs-warning)" }}>
                        {probeText}
                      </strong>
                    </span>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
