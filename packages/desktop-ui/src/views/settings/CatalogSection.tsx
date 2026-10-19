import React from "react";
import { Badge } from "@studio/design-system";
import { studioApi } from "../../api/client";
import type { CatalogKit, CatalogModule } from "../../api/types";

export const CatalogSection: React.FC = () => {
  const [kits, setKits] = React.useState<CatalogKit[]>([]);
  const [modules, setModules] = React.useState<CatalogModule[]>([]);

  const loadCatalog = React.useCallback(async () => {
    try {
      const [kitList, modList] = await Promise.all([
        studioApi.getCatalogKits().catch(() => []),
        studioApi.getCatalogModules().catch(() => []),
      ]);
      setKits(kitList);
      setModules(modList);
    } catch {
      // ignore
    }
  }, []);

  React.useEffect(() => {
    loadCatalog();
  }, [loadCatalog]);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-5)" }}>
      {/* Kits section */}
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
        <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
          Bộ Kit Đặt Sẵn (Kits)
        </h2>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(260px, 1fr))", gap: "var(--cs-space-3)" }}>
          {kits.map((kit) => (
            <div
              key={kit.id}
              style={{
                padding: "var(--cs-space-3)",
                backgroundColor: "var(--cs-bg-panel)",
                border: "1px solid var(--cs-border-subtle)",
                display: "flex",
                flexDirection: "column",
                gap: 6,
              }}
            >
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
                <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>{kit.name}</span>
                <Badge variant="neutral">~{kit.tokenBudget} tokens</Badge>
              </div>
              <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", lineHeight: 1.4 }}>
                {kit.description}
              </p>
              <div style={{ fontSize: 11, color: "var(--cs-fg-muted)", marginTop: 4 }}>
                Gồm {kit.modules.length} modules: {kit.modules.join(", ")}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Modules section */}
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
        <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
          Mô-đun Kỹ Năng Tuyển Chọn (Modules)
        </h2>
        <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
          {modules.map((mod) => (
            <div
              key={mod.id}
              style={{
                padding: "var(--cs-space-3) var(--cs-space-4)",
                backgroundColor: "var(--cs-bg-panel)",
                border: "1px solid var(--cs-border-subtle)",
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
              }}
            >
              <div style={{ display: "flex", flexDirection: "column", gap: 2 }}>
                <span style={{ fontWeight: 600, fontSize: "var(--cs-text-sm)" }}>{mod.name}</span>
                <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
                  {mod.description}
                </span>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
                <Badge variant="info">{mod.skillCount} skills</Badge>
                <span style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)" }}>
                  ~{mod.tokenCount} tokens
                </span>
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
