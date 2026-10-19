import React from "react";
import { Tabs } from "@studio/design-system";
import { EnginesSection } from "./settings/EnginesSection";
import { ToolsSection } from "./settings/ToolsSection";
import { VoicesSection } from "./settings/VoicesSection";
import { AccountsSection } from "./settings/AccountsSection";
import { EnvSection } from "./settings/EnvSection";
import { ApiKeysSection } from "./settings/ApiKeysSection";
import { CatalogSection } from "./settings/CatalogSection";
import { JobsSection } from "./settings/JobsSection";
import { DiskSection } from "./settings/DiskSection";
import { WebAccessSection } from "./settings/WebAccessSection";

export interface SettingsViewProps {
  initialTab?: "engines" | "tools" | "voices" | "env" | "catalog" | "jobs" | "disk" | "api-keys" | "accounts" | "web";
}

export const SettingsView: React.FC<SettingsViewProps> = ({ initialTab = "engines" }) => {
  const [activeTab, setActiveTab] = React.useState<"engines" | "tools" | "voices" | "env" | "catalog" | "jobs" | "disk" | "api-keys" | "accounts" | "web">(initialTab);

  React.useEffect(() => {
    if (initialTab) {
      setActiveTab(initialTab);
    }
  }, [initialTab]);

  const tabs = [
    { id: "engines", label: "Engines" },
    { id: "tools", label: "Công cụ" },
    { id: "voices", label: "Giọng đọc (TTS)" },
    { id: "accounts", label: "Tài khoản đăng bài" },
    { id: "env", label: "Kiểm tra môi trường" },
    { id: "api-keys", label: "Khoá API & Endpoint" },
    { id: "catalog", label: "Catalog Modules & Kits" },
    { id: "jobs", label: "Cấu hình Tác vụ" },
    { id: "disk", label: "Dung lượng Đĩa" },
    { id: "web", label: "Truy cập Web" },
  ];

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        height: "100%",
        backgroundColor: "var(--cs-bg-app)",
      }}
    >
      {/* Header */}
      <div
        style={{
          padding: "var(--cs-space-3) var(--cs-space-5)",
          backgroundColor: "var(--cs-bg-panel)",
          borderBottom: "1px solid var(--cs-border-subtle)",
          paddingBottom: 0,
          display: "flex",
          flexDirection: "column",
          gap: "var(--cs-space-2)",
        }}
      >
        <div>
          <h1 style={{ margin: "0 0 4px 0", fontSize: "var(--cs-text-lg)", fontWeight: 700 }}>
            Tools & Cài đặt Hệ thống
          </h1>
          <p style={{ margin: 0, fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
            Kiểm tra phụ thuộc ngoài, catalog kỹ năng và dung lượng lưu trữ cục bộ.
          </p>
        </div>

        <Tabs
          items={tabs}
          activeId={activeTab}
          onChange={(id) => setActiveTab(id as any)}
          size="sm"
          style={{ borderBottom: "none" }}
        />
      </div>

      {/* Body */}
      <div style={{ flex: 1, overflowY: "auto", padding: "var(--cs-space-6)" }}>
        <div style={{ maxWidth: 880, margin: "0 auto", display: "flex", flexDirection: "column", gap: "var(--cs-space-5)" }}>
          {activeTab === "engines" && <EnginesSection />}
          {activeTab === "tools" && <ToolsSection />}
          {activeTab === "voices" && <VoicesSection />}
          {activeTab === "accounts" && <AccountsSection />}
          {activeTab === "env" && <EnvSection />}
          {activeTab === "api-keys" && <ApiKeysSection />}
          {activeTab === "catalog" && <CatalogSection />}
          {activeTab === "jobs" && <JobsSection />}
          {activeTab === "disk" && <DiskSection />}
          {activeTab === "web" && <WebAccessSection />}
        </div>
      </div>
    </div>
  );
};
