import React from "react";
import { Icon } from "../../components/Icon";
import { Badge, Button } from "@studio/design-system";
import { studioApi } from "../../api/client";
import { cleanDisplayPath } from "../../utils/path";
import type { EnvProbeItem } from "../../api/types";

export const EnvSection: React.FC = () => {
  const [envItems, setEnvItems] = React.useState<EnvProbeItem[]>([]);
  const [loading, setLoading] = React.useState(true);

  const loadEnv = React.useCallback(async () => {
    setLoading(true);
    try {
      const list = await studioApi.envProbe();
      setEnvItems(list);
    } catch {
      // ignore
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    loadEnv();
  }, [loadEnv]);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-4)" }}>
      <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <div>
          <h2 style={{ margin: 0, fontSize: "var(--cs-text-md)", fontWeight: 700 }}>
            Kiểm tra môi trường hệ thống
          </h2>
          <p style={{ margin: "2px 0 0", fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)" }}>
            Quét trực tiếp các công cụ nhị phân trên máy bằng cách thực thi lệnh thực tế (--version). Không đóng gói sẵn các công cụ này vào ứng dụng.
          </p>
        </div>
        <Button
          size="sm"
          variant="secondary"
          onClick={loadEnv}
          busy={loading}
          leftIcon={<Icon name="refresh" size={13} />}
        >
          Quét lại môi trường
        </Button>
      </div>

      <div style={{ display: "flex", flexDirection: "column", gap: "var(--cs-space-3)" }}>
        {envItems.map((item) => (
          <div
            key={item.id}
            style={{
              padding: "var(--cs-space-4)",
              backgroundColor: "var(--cs-bg-panel)",
              border: "1px solid var(--cs-border-subtle)",
              borderRadius: "var(--cs-radius-control)",
              display: "flex",
              flexDirection: "column",
              gap: 8,
            }}
          >
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between" }}>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <Icon
                  name={item.found ? "check-circle" : "alert-circle"}
                  size={18}
                  color={item.found ? "var(--cs-success)" : "var(--cs-danger)"}
                />
                <span style={{ fontWeight: 700, fontSize: "var(--cs-text-sm)" }}>
                  {item.name}
                </span>
                <Badge variant={item.found ? "success" : "danger"}>
                  {item.found ? "Đã có" : "Chưa có"}
                </Badge>
              </div>
              {item.version && (
                <span style={{ fontSize: "var(--cs-text-xs)", fontFamily: "var(--cs-font-mono)", color: "var(--cs-fg-primary)" }}>
                  Phiên bản: {item.version}
                </span>
              )}
            </div>

            {item.path && (
              <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-muted)", fontFamily: "var(--cs-font-mono)" }}>
                Đường dẫn: {cleanDisplayPath(item.path)}
              </div>
            )}

            {!item.found && (
              <div style={{ marginTop: 2, padding: "8px 12px", backgroundColor: "var(--cs-bg-raised)", border: "1px dashed var(--cs-border-subtle)", borderRadius: "var(--cs-radius-control)" }}>
                <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-warning)", fontWeight: 600 }}>
                  Hướng dẫn cài đặt:
                </div>
                <div style={{ fontSize: "var(--cs-text-xs)", color: "var(--cs-fg-secondary)", marginTop: 2 }}>
                  {item.hint}
                </div>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
};
