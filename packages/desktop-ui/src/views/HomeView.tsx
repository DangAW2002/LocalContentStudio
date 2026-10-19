import React from "react";
import { Icon } from "../components/Icon";
import { Button, Chip, Select, Callout, EmptyState, ErrorState, Badge } from "@studio/design-system";
import { studioApi } from "../api/client";
import type {
  ProjectSummary,
  JobSnapshot,
  CatalogKit,
  EngineType,
  EngineModelOption,
  ScopeItem,
} from "../api/types";

export interface HomeViewProps {
  onOpenProject: (slug: string, threadId?: string) => void;
  onNavigateSettings: () => void;
  onLoginSuccess?: () => void;
}

function slugify(text: string): string {
  return text
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/đ/g, "d")
    .replace(/Đ/g, "D")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/(^-|-$)+/g, "")
    .slice(0, 36) || "du-an-moi";
}

export const HomeView: React.FC<HomeViewProps> = ({
  onOpenProject,
  onNavigateSettings,
  onLoginSuccess,
}) => {
  const [prompt, setPrompt] = React.useState("");
  const [engine, setEngine] = React.useState<EngineType>("claude");
  const [model, setModel] = React.useState<string>("");
  const [modelOptions, setModelOptions] = React.useState<EngineModelOption[]>([]);
  const [modelsLoading, setModelsLoading] = React.useState(false);
  const [modelsError, setModelsError] = React.useState<string | null>(null);

  // Model list comes from the engine itself, so new models show up without an app update.
  const loadEngineModels = React.useCallback(async (targetEngine: EngineType) => {
    setModelsLoading(true);
    setModelsError(null);
    try {
      const options = await studioApi.getEngineModels(targetEngine);
      setModelOptions(options);
      const remembered = localStorage.getItem(`lcs.model.${targetEngine}`);
      const pick = options.find((o) => o.value === remembered) ?? options[0];
      setModel(pick?.value ?? "");
    } catch (err: any) {
      setModelOptions([]);
      setModel("");
      setModelsError(typeof err === "string" ? err : err?.message || "Không lấy được danh sách model.");
    } finally {
      setModelsLoading(false);
    }
  }, []);

  React.useEffect(() => {
    loadEngineModels(engine);
  }, [engine, loadEngineModels]);
  const [selectedKit, setSelectedKit] = React.useState<string | null>(null);
  const [suggestedKits, setSuggestedKits] = React.useState<CatalogKit[]>([]);

  // Brand and Playbook scope items
  const [brands, setBrands] = React.useState<ScopeItem[]>([]);
  const [playbooks, setPlaybooks] = React.useState<ScopeItem[]>([]);
  const [selectedBrand, setSelectedBrand] = React.useState<string>("");
  const [selectedPlaybook, setSelectedPlaybook] = React.useState<string>("");

  const [projects, setProjects] = React.useState<ProjectSummary[]>([]);
  const [jobs, setJobs] = React.useState<JobSnapshot[]>([]);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);
  const [submitting, setSubmitting] = React.useState(false);

  // Load initial data
  const loadData = React.useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [projList, jobList, brandList, pbList] = await Promise.all([
        studioApi.getProjects(),
        studioApi.getJobsSnapshot(),
        studioApi.getScopeItems("brands").catch(() => []),
        studioApi.getScopeItems("playbooks").catch(() => []),
      ]);
      setProjects(projList);
      setJobs(jobList);
      setBrands(brandList);
      setPlaybooks(pbList);
    } catch (err: any) {
      setError(err?.message || "Chưa kết nối Studio core");
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    loadData();
  }, [loadData]);

  // Unified login success handler: clear core error state, reload data and engine models
  const handleLoginSuccess = React.useCallback(async () => {
    setError(null);
    setModelsError(null);
    await Promise.all([
      loadData(),
      loadEngineModels(engine),
    ]);
    if (onLoginSuccess) {
      onLoginSuccess();
    }
  }, [loadData, loadEngineModels, engine, onLoginSuccess]);

  React.useEffect(() => {
    const onAuthSuccess = () => {
      handleLoginSuccess();
    };
    window.addEventListener("studio:login-success", onAuthSuccess);
    return () => window.removeEventListener("studio:login-success", onAuthSuccess);
  }, [handleLoginSuccess]);

  // Debounced suggest kit
  React.useEffect(() => {
    if (!prompt.trim() || prompt.length < 3) {
      setSuggestedKits([]);
      return;
    }

    const timer = setTimeout(async () => {
      try {
        const res = await studioApi.suggestCatalog({ text: prompt });
        setSuggestedKits(res.suggestedKits || []);
      } catch {
        // Silently skip if core is not connected
      }
    }, 250);

    return () => clearTimeout(timer);
  }, [prompt]);

  // Handle request submission
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!prompt.trim() || submitting) return;

    setSubmitting(true);
    setError(null);
    try {
      const slug = `${slugify(prompt)}-${Date.now().toString(36).slice(-4)}`;
      const title = prompt.trim().slice(0, 60);

      // 1. Create project
      const proj = await studioApi.createProject({
        slug,
        title,
        kit: selectedKit || undefined,
      });

      // 2. Create initial thread
      const thread = await studioApi.createThread({
        project: proj.slug,
        engine,
        model: model || undefined,
      });

      // 3. Compose initial turn prompt with brand/playbook context if selected
      const contextPrefixes: string[] = [];
      if (selectedPlaybook) {
        contextPrefixes.push(`/${selectedPlaybook}`);
      }
      if (selectedBrand) {
        const brandName = brands.find((b) => b.id === selectedBrand)?.name ?? selectedBrand;
        contextPrefixes.push(`[Áp dụng Brand: ${brandName} (id: ${selectedBrand})]`);
      }
      const initialPrompt =
        contextPrefixes.length > 0
          ? `${contextPrefixes.join(" ")} ${prompt.trim()}`
          : prompt.trim();

      // Start first turn
      await studioApi.startTurn({
        threadId: thread.id,
        prompt: initialPrompt,
      });

      // 4. Open project view with the new thread
      onOpenProject(proj.slug, thread.id);
    } catch (err: any) {
      setError(err?.message || "Không thể khởi tạo dự án với agent.");
      setSubmitting(false);
    }
  };

  const runningJobs = jobs.filter((j) => j.state === "RUNNING");

  return (
    <div
      style={{
        flex: 1,
        overflowY: "auto",
        padding: "var(--cs-space-6)",
        backgroundColor: "var(--cs-bg-app)",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
      }}
    >
      <div style={{ width: "100%", maxWidth: 840, display: "flex", flexDirection: "column", gap: "var(--cs-space-6)" }}>
        {error && (() => {
          const isNoWs = error.includes("workspace") || error.includes("Workspace");
          return (
            <ErrorState
              title={isNoWs ? "Chưa chọn Workspace" : "Kết nối Studio core"}
              reason={error}
              safetyNote={
                isNoWs
                  ? "Vui lòng chọn hoặc tạo mới một thư mục Workspace để bắt đầu làm việc."
                  : "Khi chạy độc lập không qua Tauri shell, vui lòng kiểm tra backend Rust hoặc mở ứng dụng desktop."
              }
              actionText={isNoWs ? "Mở / Tạo Workspace" : "Thử lại"}
              onRetry={isNoWs ? () => window.dispatchEvent(new CustomEvent("studio:open-workspace-switcher")) : loadData}
            />
          );
        })()}

        {/* Big Request Box */}
        <section
          style={{
            backgroundColor: "var(--cs-bg-panel)",
            border: "1px solid var(--cs-border-subtle)",
            boxShadow: "0 2px 8px rgb(0 0 0 / 4%)",
            padding: "var(--cs-space-5)",
            display: "flex",
            flexDirection: "column",
            gap: "var(--cs-space-4)",
          }}
        >
          <div>
            <h1
              style={{
                margin: "0 0 4px 0",
                fontSize: "var(--cs-text-lg)",
                fontWeight: 700,
                color: "var(--cs-fg-primary)",
              }}
            >
              Bạn muốn tạo nội dung gì hôm nay?
            </h1>
            <p style={{ margin: 0, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-secondary)" }}>
              Giao việc cho agent. Agent sẽ tự chọn công cụ, áp dụng Brand và dựng sản phẩm cho bạn.
            </p>
          </div>

          <form onSubmit={handleSubmit} style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
            {/* Engine, Model, Brand & Playbook Selectors */}
            <div style={{ display: "flex", gap: "var(--cs-space-3)", alignItems: "center", flexWrap: "wrap" }}>
              <div style={{ width: 170 }}>
                <Select
                  size="sm"
                  label="Agent Engine"
                  value={engine}
                  onChange={(e) => {
                    const next = e.target.value as EngineType;
                    setEngine(next);
                  }}
                  options={[
                    { value: "claude", label: "Claude Code CLI" },
                    { value: "agy", label: "Antigravity CLI (agy)" },
                  ]}
                />
              </div>

              <div style={{ width: 190 }}>
                <Select
                  size="sm"
                  label="Mô hình (Model)"
                  value={model}
                  onChange={(e) => {
                    setModel(e.target.value);
                    localStorage.setItem(`lcs.model.${engine}`, e.target.value);
                  }}
                  disabled={modelsLoading || modelOptions.length === 0}
                  options={
                    modelsLoading
                      ? [{ value: "", label: "Đang tải danh sách model..." }]
                      : modelOptions.length === 0
                        ? [{ value: "", label: "Mặc định của engine" }]
                        : modelOptions
                  }
                />
                {modelsError && (
                  <div
                    role="alert"
                    title={modelsError}
                    style={{ marginTop: 4, fontSize: "var(--cs-text-xs)", color: "var(--cs-danger)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
                  >
                    Không lấy được model — dùng mặc định của engine
                  </div>
                )}
              </div>

              <div style={{ width: 180 }}>
                <Select
                  size="sm"
                  label="Brand áp dụng"
                  value={selectedBrand}
                  onChange={(e) => setSelectedBrand(e.target.value)}
                  options={[
                    { value: "", label: "Mặc định (Không chọn)" },
                    ...brands.map((b) => ({ value: b.id, label: b.name })),
                  ]}
                />
              </div>

              <div style={{ width: 180 }}>
                <Select
                  size="sm"
                  label="Quy trình (Playbook)"
                  value={selectedPlaybook}
                  onChange={(e) => setSelectedPlaybook(e.target.value)}
                  options={[
                    { value: "", label: "Không áp dụng" },
                    ...playbooks.map((pb) => ({ value: pb.name, label: pb.name })),
                  ]}
                />
              </div>

              {selectedKit && (
                <div style={{ marginTop: 18 }}>
                  <Chip
                    label={`Kit: ${selectedKit}`}
                    selected
                    onRemove={() => setSelectedKit(null)}
                  />
                </div>
              )}
            </div>

            {/* Prompt Textarea */}
            <div style={{ position: "relative" }}>
              <textarea
                value={prompt}
                onChange={(e) => setPrompt(e.target.value)}
                rows={4}
                placeholder="Ví dụ: Làm video YouTube phân tích iPhone 16 kèm 3 Shorts và thumbnail nổi bật theo Brand..."
                style={{
                  width: "100%",
                  padding: "var(--cs-space-3)",
                  fontFamily: "var(--cs-font-ui)",
                  fontSize: "var(--cs-text-md)",
                  lineHeight: 1.5,
                  backgroundColor: "var(--cs-bg-panel)",
                  color: "var(--cs-fg-primary)",
                  border: "1px solid var(--cs-border-subtle)",
                  borderRadius: "var(--cs-radius-control)",
                  outline: "none",
                  resize: "vertical",
                  boxSizing: "border-box",
                }}
              />
            </div>

            {/* Suggested Kit Chips */}
            {suggestedKits.length > 0 && (
              <div style={{ display: "flex", alignItems: "center", gap: 6, flexWrap: "wrap" }}>
                <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                  Gợi ý Kit:
                </span>
                {suggestedKits.map((kit) => (
                  <Chip
                    key={kit.id}
                    label={`+ ${kit.name}`}
                    selected={selectedKit === kit.id}
                    onClick={() => setSelectedKit(kit.id)}
                  />
                ))}
              </div>
            )}

            {/* Action Bar */}
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginTop: 4 }}>
              <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)" }}>
                Nhấn Enter hoặc bấm nút để tạo dự án và bắt đầu hội thoại
              </span>
              <div
                title={
                  !prompt.trim()
                    ? "Vui lòng nhập nội dung yêu cầu dự án để bắt đầu"
                    : submitting
                    ? "Đang khởi tạo dự án..."
                    : "Bắt đầu dự án mới cùng agent"
                }
              >
                <Button
                  type="submit"
                  variant="primary"
                  disabled={!prompt.trim() || submitting}
                  busy={submitting}
                  busyText="Đang khởi tạo dự án..."
                  leftIcon={<Icon name="sparkles" size={16} />}
                >
                  Bắt đầu dự án
                </Button>
              </div>
            </div>
          </form>
        </section>

        {/* Running Jobs Section (if any) */}
        {runningJobs.length > 0 && (
          <section
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
              <h2 style={{ margin: 0, fontSize: "var(--cs-text-sm)", fontWeight: 600 }}>
                Tác vụ nền đang chạy ({runningJobs.length})
              </h2>
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {runningJobs.map((j) => (
                <div
                  key={j.id}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    padding: "8px 12px",
                    backgroundColor: "var(--cs-bg-raised)",
                    border: "1px solid var(--cs-border-subtle)",
                    fontSize: "var(--cs-text-sm)",
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <Icon name="loader" size={16} color="var(--cs-accent)" className="cs-spin" />
                    <span style={{ fontWeight: 600 }}>{j.title}</span>
                    <span style={{ color: "var(--cs-fg-muted)", fontSize: "var(--cs-text-xs)" }}>
                      ({j.stageName || "Đang xử lý"})
                    </span>
                  </div>
                  <Badge variant="info">Đang chạy</Badge>
                </div>
              ))}
            </div>
          </section>
        )}

        {/* Recent Projects Section */}
        <section
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
            <h2 style={{ margin: 0, fontSize: "var(--cs-text-sm)", fontWeight: 600 }}>
              Dự án gần đây
            </h2>
          </div>

          {loading ? (
            <div style={{ padding: "var(--cs-space-4)", textAlign: "center", color: "var(--cs-fg-muted)" }}>
              Đang tải danh sách dự án...
            </div>
          ) : projects.length === 0 ? (
            <EmptyState
              icon={<Icon name="folder" size={24} />}
              title="Chưa có dự án nào"
              description="Nhập yêu cầu vào ô phía trên để bắt đầu dự án sáng tạo đầu tiên của bạn."
            />
          ) : (
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fill, minmax(240px, 1fr))",
                gap: "var(--cs-space-3)",
              }}
            >
              {projects.map((proj) => (
                <div
                  key={proj.slug}
                  onClick={() => onOpenProject(proj.slug)}
                  style={{
                    padding: "var(--cs-space-3)",
                    border: "1px solid var(--cs-border-subtle)",
                    backgroundColor: "var(--cs-bg-panel)",
                    cursor: "pointer",
                    display: "flex",
                    flexDirection: "column",
                    gap: 6,
                    transition: "border-color var(--cs-motion-duration) ease, background-color var(--cs-motion-duration) ease",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.borderColor = "var(--cs-accent)";
                    e.currentTarget.style.backgroundColor = "var(--cs-bg-hover)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.borderColor = "var(--cs-border-subtle)";
                    e.currentTarget.style.backgroundColor = "var(--cs-bg-panel)";
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                    <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)", color: "var(--cs-fg-primary)" }}>
                      {proj.title}
                    </span>
                    {proj.kit && <Badge variant="neutral">{proj.kit}</Badge>}
                  </div>
                  <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)" }}>
                    {proj.slug}
                  </span>
                  <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginTop: 4, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                    <span>{proj.artifactCount ? `${proj.artifactCount} thành phẩm` : "Chưa có thành phẩm"}</span>
                    <span>{proj.updatedAt ? new Date(proj.updatedAt).toLocaleDateString("vi-VN") : ""}</span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </section>
      </div>
    </div>
  );
};
