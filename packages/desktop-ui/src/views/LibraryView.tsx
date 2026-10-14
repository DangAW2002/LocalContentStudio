import React from "react";
import { Badge, Button, Dialog, Input } from "@studio/design-system";

export const LibraryView: React.FC = () => {
  const [reviewAsset, setReviewAsset] = React.useState<any | null>(null);
  const [licenseBasis, setLicenseBasis] = React.useState("Free commercial license");
  const [source, setSource] = React.useState("Chụp nội bộ");

  const assets = [
    {
      id: "asset-1",
      name: "banner-chinh.png",
      kind: "image",
      size: "1.4 MB",
      rightsState: "READY",
      attestation: "Bản quyền sở hữu doanh nghiệp",
    },
    {
      id: "asset-2",
      name: "anh-san-pham-chua-duyet.jpg",
      kind: "image",
      size: "850 KB",
      rightsState: "QUARANTINED",
    },
    {
      id: "asset-3",
      name: "intro-audio.mp3",
      kind: "audio",
      size: "3.2 MB",
      rightsState: "READY",
      attestation: "Creative Commons 0",
    },
  ];

  return (
    <div style={{ padding: "var(--cs-space-4)", display: "flex", flexDirection: "column", gap: "var(--cs-space-4)", height: "100%", overflowY: "auto" }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <div>
          <h1 style={{ margin: 0, fontSize: "var(--cs-text-lg)", fontWeight: 700 }}>
            Thư viện tài nguyên (Asset Library)
          </h1>
          <span style={{ fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)" }}>
            Quản lý tệp, deduplication theo SHA-256, cách ly QUARANTINED và phê duyệt bản quyền Human-only
          </span>
        </div>
        <Button variant="primary">+ Tải lên tệp mới</Button>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(260px, 1fr))", gap: "var(--cs-space-3)" }}>
        {assets.map((asset) => (
          <div
            key={asset.id}
            style={{
              padding: "var(--cs-space-3)",
              backgroundColor: "var(--cs-bg-panel)",
              borderRadius: "var(--cs-radius-md)",
              border: "1px solid var(--cs-border-subtle)",
              display: "flex",
              flexDirection: "column",
              gap: "var(--cs-space-2)",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>{asset.name}</span>
              <Badge variant={asset.rightsState === "READY" ? "success" : "warning"}>
                {asset.rightsState}
              </Badge>
            </div>
            <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
              {asset.kind.toUpperCase()} · Dung lượng: {asset.size}
            </span>
            {asset.attestation ? (
              <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-success)" }}>
                ✓ {asset.attestation}
              </span>
            ) : (
              <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-warning)" }}>
                ⚠️ Chưa duyệt: Bị khóa xuất bản/bàn giao
              </span>
            )}
            <div style={{ marginTop: "auto", paddingTop: "var(--cs-space-2)" }}>
              {asset.rightsState === "QUARANTINED" ? (
                <Button variant="primary" style={{ width: "100%" }} onClick={() => setReviewAsset(asset)}>
                  Duyệt bản quyền (Human)
                </Button>
              ) : (
                <Button variant="secondary" style={{ width: "100%" }}>
                  Sử dụng tài nguyên
                </Button>
              )}
            </div>
          </div>
        ))}
      </div>

      {/* Human Rights Review Modal */}
      {reviewAsset && (
        <Dialog
          open={Boolean(reviewAsset)}
          title={`Xác nhận bản quyền: ${reviewAsset.name}`}
          onClose={() => setReviewAsset(null)}
          footer={
            <>
              <Button variant="secondary" onClick={() => setReviewAsset(null)}>Hủy</Button>
              <Button
                variant="primary"
                onClick={() => {
                  alert("Đã xác nhận bản quyền và chuyển trạng thái sang READY.");
                  setReviewAsset(null);
                }}
              >
                Xác nhận phê duyệt (READY)
              </Button>
            </>
          }
        >
          <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
            <p style={{ margin: 0, fontSize: "var(--cs-text-sm)" }}>
              Theo quy tắc bất biến của hệ thống, chỉ <strong>người dùng được xác thực (Human)</strong> mới có quyền hợp pháp để phê duyệt bản quyền tài nguyên. Agent AI không có thẩm quyền này.
            </p>
            <Input
              label="Nguồn gốc tài nguyên (Source):"
              value={source}
              onChange={(e) => setSource(e.target.value)}
            />
            <Input
              label="Cơ sở giấy phép / Pháp lý (License Basis):"
              value={licenseBasis}
              onChange={(e) => setLicenseBasis(e.target.value)}
            />
          </div>
        </Dialog>
      )}
    </div>
  );
};
