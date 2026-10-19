// scripts/e2e/smoke.mjs
// Automated E2E smoke test suite for Local Content Studio.
// Tests all views (Home, Projects, Brand, Library, Settings), tabs, themes (light/dark),
// and window widths (1440 and 900), capturing screenshots and verifying error-free rendering.
import { mkdirSync, cpSync } from "node:fs";
import path from "node:path";
import { startApp } from "./start.mjs";
import { CDPClient } from "./cdp.mjs";
import { stopApp } from "./stop.mjs";

const VIEWS = [
  { id: "home", name: "Home", shortcutKey: "1" },
  { id: "projects", name: "Dự án", shortcutKey: "2" },
  { id: "brand", name: "Brand", shortcutKey: "3" },
  { id: "library", name: "Thư viện", shortcutKey: "4" },
  { id: "settings", name: "Cài đặt", shortcutKey: "5" },
];

const SETTINGS_TABS = [
  "engines",
  "tools",
  "voices",
  "accounts",
  "env",
  "api-keys",
  "catalog",
  "jobs",
  "disk",
  "web",
];

const BRAND_TABS = [
  "brands",
  "channels",
  "playbooks",
];

const OUTPUTS_TABS = [
  "artifacts",
  "checkpoints",
  "modules",
];

const VIEWPORTS = [
  { width: 1440, height: 900, name: "1440" },
  { width: 900, height: 750, name: "900" },
  { width: 768, height: 700, name: "768" },
];

export async function runSmoke(options = {}) {
  let runData = null;
  let client = null;
  const failureReasons = [];

  try {
    console.log("=== BẮT ĐẦU E2E SMOKE TEST ===");
    runData = await startApp({ noBuild: options.noBuild ?? true, port: options.port });
    console.log(`[smoke] App running with PID ${runData.pid} on CDP port ${runData.port}`);

    client = await CDPClient.connect({ port: runData.port });
    const shotsDir = runData.shotsDir;
    mkdirSync(shotsDir, { recursive: true });

    // Wait until no active spinner or "Đang…" loading label is visible
    async function waitSettled(maxWaitMs = 5000) {
      const start = Date.now();
      while (Date.now() - start < maxWaitMs) {
        const isBusy = await client.eval(`(() => {
          const bodyText = document.body.innerText || "";
          if (document.querySelector('.cs-spin, [role="progressbar"], .cs-thinking-dot')) return true;
          if (bodyText.includes("Đang tải danh sách") || bodyText.includes("Đang quét…") || bodyText.includes("Đang quét công cụ")) return true;
          return false;
        })()`);
        if (!isBusy) break;
        await new Promise((r) => setTimeout(r, 150));
      }
      await new Promise((r) => setTimeout(r, 200));
    }

    // Helper to evaluate checkErrors and throw if failed
    async function assertCleanState(stepName) {
      await waitSettled();
      const res = await client.checkErrors();
      if (!res.ok) {
        const msg = `[LỖI TẠI ${stepName}] ${res.reason}`;
        console.error(msg);
        failureReasons.push(msg);
      }
    }

    // Helper to set theme
    async function setTheme(targetTheme) {
      await client.eval(`(() => {
        const cur = document.documentElement.getAttribute('data-theme') || 'light';
        if (cur !== '${targetTheme}') {
          const btn = document.querySelector('button[aria-label*="giao diện"]');
          if (btn) btn.click();
        }
      })()`);
      await new Promise((r) => setTimeout(r, 150));
    }

    // Run matrix: Theme (light, dark) x Viewport (1440, 900)
    for (const theme of ["light", "dark"]) {
      console.log(`\n--- KIỂM THỬ GIAO DIỆN: THEME = ${theme.toUpperCase()} ---`);
      await setTheme(theme);

      for (const vp of VIEWPORTS) {
        console.log(`  > Độ phân giải: ${vp.width}x${vp.height} (${vp.name})`);
        await client.setViewport(vp.width, vp.height);

        // 1. Home View
        console.log("    [Home] Mở trang chủ");
        await client.key("1", ["ctrl"]);
        await new Promise((r) => setTimeout(r, 300));
        await assertCleanState(`Home (${theme}-${vp.name})`);
        await client.shot(path.join(shotsDir, `home-${theme}-${vp.name}.png`));

        // 2. Project View (Thread, Stage, Outputs)
        console.log("    [Dự án] Mở trang Dự án");
        await client.key("2", ["ctrl"]);
        await new Promise((r) => setTimeout(r, 500));
        await assertCleanState(`Dự án (${theme}-${vp.name})`);
        await client.shot(path.join(shotsDir, `project-${theme}-${vp.name}.png`));

        // 3 tabs of Outputs pane: artifacts, checkpoints, modules
        for (const outTab of OUTPUTS_TABS) {
          console.log(`      [Outputs] Chuyển tab: ${outTab}`);
          await client.eval(`(() => {
            const tabs = [...document.querySelectorAll('[role=tab]')];
            const target = tabs.find(t => {
              const text = (t.textContent || '').toLowerCase();
              if ('${outTab}' === 'artifacts') return text.includes('thành phẩm');
              if ('${outTab}' === 'checkpoints') return text.includes('checkpoints');
              if ('${outTab}' === 'modules') return text.includes('modules');
              return false;
            });
            if (target) target.click();
          })()`);
          await new Promise((r) => setTimeout(r, 200));
          await assertCleanState(`Outputs tab ${outTab} (${theme}-${vp.name})`);
          await client.shot(path.join(shotsDir, `project-outputs-${outTab}-${theme}-${vp.name}.png`));
        }

        // 3. Brand View (3 tabs: brands, channels, playbooks)
        console.log("    [Brand] Mở trang Brand, Channel & Playbook");
        await client.key("3", ["ctrl"]);
        await new Promise((r) => setTimeout(r, 400));
        await assertCleanState(`Brand (${theme}-${vp.name})`);
        await client.shot(path.join(shotsDir, `brand-main-${theme}-${vp.name}.png`));

        for (const bTab of BRAND_TABS) {
          console.log(`      [Brand] Chuyển tab: ${bTab}`);
          await client.eval(`(() => {
            const tabs = [...document.querySelectorAll('[role=tab]')];
            const target = tabs.find(t => {
              const text = (t.textContent || '').toLowerCase();
              if ('${bTab}' === 'brands') return text.includes('thương hiệu');
              if ('${bTab}' === 'channels') return text.includes('kênh');
              if ('${bTab}' === 'playbooks') return text.includes('quy trình');
              return false;
            });
            if (target) target.click();
          })()`);
          await new Promise((r) => setTimeout(r, 200));
          await assertCleanState(`Brand tab ${bTab} (${theme}-${vp.name})`);
          await client.shot(path.join(shotsDir, `brand-${bTab}-${theme}-${vp.name}.png`));
        }

        // 4. Library View
        console.log("    [Thư viện] Mở Thư viện tài nguyên");
        await client.key("4", ["ctrl"]);
        await new Promise((r) => setTimeout(r, 300));
        await assertCleanState(`Thư viện (${theme}-${vp.name})`);
        await client.shot(path.join(shotsDir, `library-${theme}-${vp.name}.png`));

        // 5. Settings View (10 tabs)
        console.log("    [Cài đặt] Mở Tools & Cài đặt hệ thống");
        await client.key("5", ["ctrl"]);
        await new Promise((r) => setTimeout(r, 400));
        await assertCleanState(`Cài đặt (${theme}-${vp.name})`);

        for (let i = 0; i < SETTINGS_TABS.length; i++) {
          const sTab = SETTINGS_TABS[i];
          console.log(`      [Cài đặt] Chuyển tab: ${sTab}`);
          await client.eval(`(() => {
            const tabs = [...document.querySelectorAll('[role=tab]')];
            if (tabs[${i}]) tabs[${i}].click();
          })()`);
          await new Promise((r) => setTimeout(r, 200));
          await assertCleanState(`Cài đặt tab ${sTab} (${theme}-${vp.name})`);
          await client.shot(path.join(shotsDir, `settings-${sTab}-${theme}-${vp.name}.png`));
        }
      }
    }

    console.log("\n=== KẾT THÚC DUYỆT CÁC MÀN HÌNH ===");
    console.log(`Ảnh chụp đã lưu tại: ${shotsDir}`);

    // Copy screenshots to task evidence directory
    const evidenceDir = path.resolve("D:/Users/Dang/LocalContentStudio/.aw/tasks/2026-10-18-m03-ra-ui-vong-2/evidence");
    try {
      mkdirSync(evidenceDir, { recursive: true });
      cpSync(shotsDir, evidenceDir, { recursive: true });
      console.log(`[smoke] Đã đồng bộ ảnh chụp bằng chứng vào: ${evidenceDir}`);
    } catch (e) {
      console.warn(`[smoke] Không thể sao chép bằng chứng vào task folder: ${e.message}`);
    }

    if (failureReasons.length > 0) {
      console.error(`\n❌ SMOKE TEST THẤT BẠI với ${failureReasons.length} lỗi:`);
      for (const r of failureReasons) {
        console.error(` - ${r}`);
      }
      throw new Error(`Smoke test failed with ${failureReasons.length} errors.`);
    }

    console.log("\n✅ SMOKE TEST THÀNH CÔNG RỰC RỠ: Tất cả màn hình và tab đạt yêu cầu kiểm tra!");
  } finally {
    if (client) {
      client.close();
    }
    if (runData && !options.keepAlive) {
      console.log(`[smoke] Dừng ứng dụng PID ${runData.pid}...`);
      await stopApp({ runFile: path.join(runData.runDir, "run.json") });
    }
  }
}

// CLI entrypoint
if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve("scripts/e2e/smoke.mjs")) {
  const args = process.argv.slice(2);
  const noBuild = !args.includes("--build");
  const keepAlive = args.includes("--keep-alive");
  const portIdx = args.indexOf("--port");
  const port = portIdx !== -1 ? args[portIdx + 1] : null;

  try {
    await runSmoke({ noBuild, keepAlive, port });
    process.exit(0);
  } catch (err) {
    console.error("[smoke] Exception:", err.message);
    process.exit(1);
  }
}
