import React from "react";
import { Button, Input, Select, Dialog } from "@studio/design-system";
import { Icon } from "../../components/Icon";
import { studioApi } from "../../api/client";
import type { SeriesSummary, ProjectSummary } from "../../api/types";

export interface CreateEpisodeDialogProps {
  open: boolean;
  onClose: () => void;
  onCreated: (project: ProjectSummary) => void;
  defaultSeriesSlug?: string;
}

function slugify(text: string): string {
  return (
    text
      .normalize("NFD")
      .replace(/[\u0300-\u036f]/g, "")
      .replace(/đ/g, "d")
      .replace(/Đ/g, "D")
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/(^-|-$)+/g, "")
      .slice(0, 36) || "tap-moi"
  );
}

export const CreateEpisodeDialog: React.FC<CreateEpisodeDialogProps> = ({
  open,
  onClose,
  onCreated,
  defaultSeriesSlug,
}) => {
  const [seriesList, setSeriesList] = React.useState<SeriesSummary[]>([]);
  const [selectedSeriesSlug, setSelectedSeriesSlug] = React.useState<string>(defaultSeriesSlug || "");
  const [title, setTitle] = React.useState("");
  const [slug, setSlug] = React.useState("");
  const [slugManuallyEdited, setSlugManuallyEdited] = React.useState(false);
  const [loading, setLoading] = React.useState(false);
  const [submitting, setSubmitting] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    if (open) {
      setLoading(true);
      setError(null);
      studioApi
        .listSeries()
        .then((list) => {
          setSeriesList(list);
          if (list.length > 0 && !selectedSeriesSlug) {
            setSelectedSeriesSlug(defaultSeriesSlug || list[0].slug);
          }
        })
        .catch((err: any) => {
          setError(err?.message || "Không thể tải danh sách series.");
        })
        .finally(() => setLoading(false));
    }
  }, [open, defaultSeriesSlug, selectedSeriesSlug]);

  React.useEffect(() => {
    if (!slugManuallyEdited && title.trim()) {
      setSlug(slugify(title));
    }
  }, [title, slugManuallyEdited]);

  const selectedSeries = seriesList.find((s) => s.slug === selectedSeriesSlug);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedSeriesSlug || !title.trim() || !slug.trim() || submitting) {
      return;
    }

    setSubmitting(true);
    setError(null);
    try {
      const created = await studioApi.createEpisodeFromSeries(
        selectedSeriesSlug,
        slug.trim(),
        title.trim()
      );
      onCreated(created);
      onClose();
    } catch (err: any) {
      setError(err?.message || "Lỗi khi tạo tập mới từ series.");
    } finally {
      setSubmitting(false);
    }
  };

  if (!open) return null;

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title="Tạo tập mới từ series"
    >
      <form onSubmit={handleSubmit} style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
        {error && (
          <div
            style={{
              padding: "var(--cs-space-2) var(--cs-space-3)",
              backgroundColor: "var(--cs-status-danger-bg)",
              color: "var(--cs-danger)",
              border: "1px solid var(--cs-danger)",
              borderRadius: "var(--cs-radius-control)",
              fontSize: "var(--cs-text-sm)",
            }}
          >
            {error}
          </div>
        )}

        {/* Series Selection */}
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-1)" }}>
          <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-secondary)" }}>
            Chọn Series
          </label>
          <Select
            size="md"
            value={selectedSeriesSlug}
            onChange={(e) => setSelectedSeriesSlug(e.target.value)}
            disabled={loading || seriesList.length === 0}
            options={
              seriesList.length > 0
                ? seriesList.map((s) => ({ value: s.slug, label: `${s.title} (${s.slug})` }))
                : [{ value: "", label: loading ? "Đang tải series..." : "Chưa có series nào" }]
            }
          />
        </div>

        {/* Series Recipe Summary */}
        {selectedSeries && (
          <div
            style={{
              padding: "var(--cs-space-3)",
              backgroundColor: "var(--cs-bg-raised)",
              borderRadius: "var(--cs-radius-control)",
              fontSize: "var(--cs-text-xs)",
              display: "flex",
              flexDirection: "column",
              gap: 4,
              color: "var(--cs-fg-secondary)",
            }}
          >
            <div>
              <strong style={{ color: "var(--cs-fg-primary)" }}>Mô tả:</strong>{" "}
              {selectedSeries.description || "Không có mô tả"}
            </div>
            <div>
              <strong style={{ color: "var(--cs-fg-primary)" }}>Kit / Playbook:</strong>{" "}
              {selectedSeries.recipe.kit || "Tự do"} / {selectedSeries.recipe.playbook || "Không có"}
            </div>
            <div>
              <strong style={{ color: "var(--cs-fg-primary)" }}>Hồ sơ áp dụng:</strong>{" "}
              {selectedSeries.references && selectedSeries.references.length > 0
                ? selectedSeries.references.join(", ")
                : "Chưa có hồ sơ ghim sẵn"}
            </div>
          </div>
        )}

        {/* Episode Title */}
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-1)" }}>
          <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-secondary)" }}>
            Tiêu đề tập mới
          </label>
          <Input
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder="Ví dụ: Tập 2: Chiến dịch Bạch Đằng"
            autoFocus
          />
        </div>

        {/* Episode Slug */}
        <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-1)" }}>
          <label style={{ fontSize: "var(--cs-text-xs)", fontWeight: 600, color: "var(--cs-fg-secondary)" }}>
            Slug (tên thư mục dự án)
          </label>
          <Input
            value={slug}
            onChange={(e) => {
              setSlug(e.target.value);
              setSlugManuallyEdited(true);
            }}
            placeholder="tap-2-chien-dich-bach-dang"
          />
        </div>

        {/* Action Buttons */}
        <div style={{ display: "flex", justifyContent: "flex-end", gap: "var(--cs-space-2)", marginTop: "var(--cs-space-2)" }}>
          <Button type="button" variant="secondary" onClick={onClose} disabled={submitting}>
            Hủy
          </Button>
          <Button
            type="submit"
            variant="primary"
            disabled={!selectedSeriesSlug || !title.trim() || !slug.trim() || submitting}
            busy={submitting}
            busyText="Đang tạo tập..."
            leftIcon={<Icon name="sparkles" size={14} />}
          >
            Tạo tập mới
          </Button>
        </div>
      </form>
    </Dialog>
  );
};
