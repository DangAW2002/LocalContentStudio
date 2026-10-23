// scripts/e2e/start.mjs
// Boots an isolated instance of Local Content Studio for E2E testing.
// Fully isolated via STUDIO_APP_DIR and WEBVIEW2_USER_DATA_FOLDER.
import { E2E_EXE, E2E_TARGET_DIR, getProcessInfo } from "./stop.mjs";
import { spawn, execSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, readFileSync, writeFileSync, openSync, closeSync } from "node:fs";
import { DatabaseSync } from "node:sqlite";
import net from "node:net";
import path from "node:path";
import { CDPClient } from "./cdp.mjs";

function getFreePort() {
  return new Promise((resolve, reject) => {
    const srv = net.createServer();
    srv.listen(0, "127.0.0.1", () => {
      const port = srv.address().port;
      srv.close(() => resolve(port));
    });
    srv.on("error", reject);
  });
}

function normalizePath(p) {
  let s = path.resolve(p).replace(/\\\\/g, "\\");
  if (s.startsWith("\\\\?\\UNC\\")) s = "\\\\" + s.slice(8);
  else if (s.startsWith("\\\\?\\")) s = s.slice(4);
  return s.replace(/[\/\\]+$/, "");
}

function seedWorkspaceAndAppDb({ appDir, wsDir }) {
  const now = new Date().toISOString();

  // 1. Filesystem layout for workspace
  mkdirSync(path.join(wsDir, "brands"), { recursive: true });
  mkdirSync(path.join(wsDir, "channels"), { recursive: true });
  mkdirSync(path.join(wsDir, "playbooks"), { recursive: true });
  mkdirSync(path.join(wsDir, "skills"), { recursive: true });
  mkdirSync(path.join(wsDir, "tools"), { recursive: true });
  mkdirSync(path.join(wsDir, "capabilities"), { recursive: true });
  mkdirSync(path.join(wsDir, "library"), { recursive: true });
  mkdirSync(path.join(wsDir, "projects/demo/artifacts/thumb-01"), { recursive: true });
  mkdirSync(path.join(wsDir, "projects/demo/renders"), { recursive: true });
  mkdirSync(path.join(wsDir, ".studio"), { recursive: true });

  // workspace studio.yaml
  writeFileSync(
    path.join(wsDir, "studio.yaml"),
    `name: "E2E Workspace"\nversion: 1\ndefault_brand: demo\nmonthly_budget_usd: 50\n`
  );

  // Brand/channel/playbook seeds are copied from the shipped defaults so they always match the
  // real schemas (one invalid file makes BrandSet::load fail for the whole scope chain).
  const defaults = path.resolve("crates/studio-brand/defaults");
  cpSync(path.join(defaults, "brands/example"), path.join(wsDir, "brands/demo"), { recursive: true });
  const brandYaml = path.join(wsDir, "brands/demo/brand.yaml");
  writeFileSync(
    brandYaml,
    readFileSync(brandYaml, "utf-8").replace(/^name: .*$/m, 'name: "Thương hiệu E2E"'),
  );
  writeFileSync(
    path.join(wsDir, "channels/youtube-e2e.yaml"),
    readFileSync(path.join(defaults, "channels/youtube.yaml"), "utf-8")
      .replace(/^id: .*$/m, 'id: "youtube-e2e"')
      .replace(/^title: .*$/m, 'title: "Kênh YouTube E2E"'),
  );
  writeFileSync(
    path.join(wsDir, "playbooks/e2e-short.md"),
    readFileSync(path.join(defaults, "playbooks/shorts.md"), "utf-8").replace(
      /^name: .*$/m,
      'name: "Kịch bản Video ngắn E2E"',
    ),
  );

  // demo project.yaml
  writeFileSync(
    path.join(wsDir, "projects/demo/project.yaml"),
    `slug: demo\ntitle: "Dự án mẫu E2E"\nseries: lich-su\ncreated_at: "${now}"\nkit: null\nmodules: []\n`
  );

  // series lich-su
  mkdirSync(path.join(wsDir, "series/lich-su/references"), { recursive: true });
  writeFileSync(
    path.join(wsDir, "series/lich-su/series.yaml"),
    `slug: lich-su\ntitle: "Series Lịch sử Việt Nam"\nreferences:\n  - ref-an\n`
  );

  // sample artifacts (5 linked artifacts: script-01, voice-01, thumb-01, ref-an, video-short)
  const dummyPng = Buffer.from(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
    "base64"
  );

  mkdirSync(path.join(wsDir, "projects/demo/artifacts/script-01"), { recursive: true });
  mkdirSync(path.join(wsDir, "projects/demo/artifacts/voice-01"), { recursive: true });
  mkdirSync(path.join(wsDir, "projects/demo/artifacts/ref-an"), { recursive: true });
  mkdirSync(path.join(wsDir, "projects/demo/artifacts/video-short"), { recursive: true });

  writeFileSync(
    path.join(wsDir, "projects/demo/artifacts/script-01/artifact.yaml"),
    `id: script-01\nkind: design\ntitle: "Kịch bản E2E"\nbrand: demo\nchannels:\n  - youtube-e2e\nsource: script.txt\n`
  );
  writeFileSync(
    path.join(wsDir, "projects/demo/artifacts/script-01/script.txt"),
    `Đây là nội dung kịch bản video ngắn E2E.`
  );

  writeFileSync(
    path.join(wsDir, "projects/demo/artifacts/voice-01/artifact.yaml"),
    `id: voice-01\nkind: audio\ntitle: "Giọng đọc E2E"\nbrand: demo\nchannels:\n  - youtube-e2e\nsource: voice.txt\ninputs:\n  - artifact:script-01\n`
  );
  writeFileSync(
    path.join(wsDir, "projects/demo/artifacts/voice-01/voice.txt"),
    `Nội dung giọng đọc được tạo từ kịch bản.`
  );

  writeFileSync(
    path.join(wsDir, "projects/demo/artifacts/ref-an/artifact.yaml"),
    `id: ref-an\nkind: reference\ntitle: "Hồ sơ nhân vật An"\nbrand: demo\nchannels:\n  - youtube-e2e\nsource: profile.md\nparams:\n  images:\n    - an-1.png\n`
  );
  writeFileSync(
    path.join(wsDir, "projects/demo/artifacts/ref-an/profile.md"),
    `# Hồ sơ nhân vật An\nNhân vật người dẫn chuyện phong cách lịch sử.`
  );
  writeFileSync(
    path.join(wsDir, "projects/demo/artifacts/ref-an/an-1.png"),
    dummyPng
  );

  writeFileSync(
    path.join(wsDir, "projects/demo/artifacts/thumb-01/artifact.yaml"),
    `id: thumb-01\nkind: design\ntitle: "Thumbnail E2E"\nbrand: demo\nchannels:\n  - youtube-e2e\nsource: index.html\ninputs:\n  - ref: artifact:script-01\n    role: reference\n  - ref: artifact:voice-01\n    role: uses\n`
  );
  writeFileSync(
    path.join(wsDir, "projects/demo/artifacts/thumb-01/index.html"),
    `<!DOCTYPE html><html><body style="background:#1e1e2e;color:#fff;display:flex;align-items:center;justify-content:center;height:100vh;"><h1>Thumbnail E2E</h1></body></html>`
  );

  writeFileSync(
    path.join(wsDir, "projects/demo/artifacts/video-short/artifact.yaml"),
    `id: video-short\nkind: timeline\ntitle: "Video ngắn 9:16"\nbrand: demo\nchannels:\n  - youtube-e2e\nsource: timeline.json\ninputs:\n  - ref: artifact:thumb-01\n    role: variant_of\n  - ref: artifact:ref-an\n    role: reference\n`
  );
  writeFileSync(
    path.join(wsDir, "projects/demo/artifacts/video-short/timeline.json"),
    `{"version":1,"width":1080,"height":1920,"fps":30,"tracks":[]}`
  );

  // Render dummy output for thumb-01
  writeFileSync(path.join(wsDir, "projects/demo/renders/thumb-01.png"), dummyPng);

  // 2. Workspace SQLite: .studio/studio.db
  // Only the tables the seed rows need, copied from migration v1. No schema_migrations rows: the
  // app's WorkspaceDb::open then runs every migration (IF NOT EXISTS), so the schema always
  // matches the real one (marking versions as applied here left publish_requests missing).
  const wsDbPath = path.join(wsDir, ".studio/studio.db");
  const wsDb = new DatabaseSync(wsDbPath);

  wsDb.exec(`
    CREATE TABLE IF NOT EXISTS threads (
      id TEXT PRIMARY KEY,
      project_slug TEXT,
      title TEXT NOT NULL,
      engine TEXT NOT NULL,
      model TEXT,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS turns (
      id TEXT PRIMARY KEY,
      thread_id TEXT NOT NULL,
      turn_index INTEGER NOT NULL,
      user_prompt TEXT NOT NULL,
      status TEXT NOT NULL,
      created_at TEXT NOT NULL,
      completed_at TEXT,
      FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS events (
      id TEXT PRIMARY KEY,
      turn_id TEXT NOT NULL,
      event_type TEXT NOT NULL,
      payload_json TEXT NOT NULL,
      created_at TEXT NOT NULL,
      FOREIGN KEY (turn_id) REFERENCES turns(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS artifacts_index (
      id TEXT PRIMARY KEY,
      project_slug TEXT NOT NULL,
      kind TEXT NOT NULL,
      title TEXT NOT NULL,
      path TEXT NOT NULL,
      brand TEXT,
      channels_json TEXT NOT NULL,
      source_file TEXT NOT NULL,
      renderer TEXT,
      updated_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS checkpoints (
      id TEXT PRIMARY KEY,
      label TEXT NOT NULL,
      turn_id TEXT,
      pinned INTEGER NOT NULL DEFAULT 0,
      created_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS renders (
      id TEXT PRIMARY KEY,
      artifact_id TEXT NOT NULL,
      hash TEXT NOT NULL,
      output_path TEXT NOT NULL,
      format TEXT NOT NULL,
      size_bytes INTEGER NOT NULL,
      created_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS artifact_checks (
      artifact_id TEXT NOT NULL,
      project TEXT NOT NULL,
      channel_id TEXT NOT NULL,
      render_hash TEXT NOT NULL,
      channel_version TEXT NOT NULL,
      status TEXT NOT NULL,
      errors_count INTEGER NOT NULL,
      warnings_count INTEGER NOT NULL,
      findings_json TEXT NOT NULL,
      checked_at TEXT NOT NULL,
      PRIMARY KEY (project, artifact_id, channel_id)
    );
  `);

  // Seed thread with 3 finished turns for a rich conversation view
  const doneEvt = JSON.stringify({ type: "Done", status: "success", text: null });
  const textEvt1 = JSON.stringify({
    type: "Text",
    text: "Chào bạn! Tôi đã phân tích yêu cầu và soạn thảo kịch bản phân cảnh 60 giây gồm 4 phân đoạn chính cùng lời bình chi tiết.",
  });
  const textEvt2 = JSON.stringify({
    type: "Text",
    text: "Đã tổng hợp giọng đọc qua Piper TTS (thời lượng 58.4s) và dựng bản thiết kế thumbnail chuẩn YouTube tại `projects/demo/artifacts/thumb-01`.",
  });
  const textEvt3 = JSON.stringify({
    type: "Text",
    text: "Ma trận kênh đã kiểm định 5 ấn phẩm: Đạt chuẩn độ phân giải và âm lượng -14 LUFS; sẵn sàng xuất bản.",
  });

  wsDb.exec(`
    INSERT OR REPLACE INTO threads VALUES (
      'demo-thread-1', 'demo', 'Hội thoại sản xuất video', 'agy', 'gemini-3.8-flash-medium', '${now}', '${now}'
    );
    INSERT OR REPLACE INTO turns VALUES (
      'demo-turn-1', 'demo-thread-1', 1, 'Xin chào, hãy lên ý tưởng và kịch bản video ngắn về pha cà phê phin.', 'success', '${now}', '${now}'
    );
    INSERT OR REPLACE INTO events VALUES ('demo-evt-1', 'demo-turn-1', 'Text', '${textEvt1.replace(/'/g, "''")}', '${now}');
    INSERT OR REPLACE INTO events VALUES ('demo-evt-2', 'demo-turn-1', 'Done', '${doneEvt.replace(/'/g, "''")}', '${now}');

    INSERT OR REPLACE INTO turns VALUES (
      'demo-turn-2', 'demo-thread-1', 2, 'Hãy tạo giọng đọc TTS và thiết kế ảnh bìa thumbnail 1280x720.', 'success', '${now}', '${now}'
    );
    INSERT OR REPLACE INTO events VALUES ('demo-evt-3', 'demo-turn-2', 'Text', '${textEvt2.replace(/'/g, "''")}', '${now}');
    INSERT OR REPLACE INTO events VALUES ('demo-evt-4', 'demo-turn-2', 'Done', '${doneEvt.replace(/'/g, "''")}', '${now}');

    INSERT OR REPLACE INTO turns VALUES (
      'demo-turn-3', 'demo-thread-1', 3, 'Kiểm tra tiêu chuẩn kênh YouTube xem có lỗi nào không.', 'success', '${now}', '${now}'
    );
    INSERT OR REPLACE INTO events VALUES ('demo-evt-5', 'demo-turn-3', 'Text', '${textEvt3.replace(/'/g, "''")}', '${now}');
    INSERT OR REPLACE INTO events VALUES ('demo-evt-6', 'demo-turn-3', 'Done', '${doneEvt.replace(/'/g, "''")}', '${now}');

    INSERT OR REPLACE INTO artifacts_index VALUES (
      'script-01', 'demo', 'design', 'Kịch bản E2E', 'projects/demo/artifacts/script-01', 'demo', '["youtube-e2e"]', 'script.txt', 'design', '${now}'
    );
    INSERT OR REPLACE INTO artifacts_index VALUES (
      'voice-01', 'demo', 'audio', 'Giọng đọc E2E', 'projects/demo/artifacts/voice-01', 'demo', '["youtube-e2e"]', 'voice.txt', 'audio', '${now}'
    );
    INSERT OR REPLACE INTO artifacts_index VALUES (
      'ref-an', 'demo', 'reference', 'Hồ sơ nhân vật An', 'projects/demo/artifacts/ref-an', 'demo', '["youtube-e2e"]', 'profile.md', 'reference', '${now}'
    );
    INSERT OR REPLACE INTO artifacts_index VALUES (
      'thumb-01', 'demo', 'design', 'Thumbnail E2E', 'projects/demo/artifacts/thumb-01', 'demo', '["youtube-e2e"]', 'index.html', 'design', '${now}'
    );
    INSERT OR REPLACE INTO artifacts_index VALUES (
      'video-short', 'demo', 'timeline', 'Video ngắn 9:16', 'projects/demo/artifacts/video-short', 'demo', '["youtube-e2e"]', 'timeline.json', 'timeline', '${now}'
    );

    INSERT OR REPLACE INTO checkpoints VALUES (
      'demo-cp-1', 'Khởi tạo ban đầu', 'demo-turn-1', 1, '${now}'
    );

    INSERT OR REPLACE INTO renders VALUES (
      'rnd-thumb-01', 'thumb-01', 'hash-thumb-01', 'projects/demo/renders/thumb-01.png', 'png', 68, '${now}'
    );

    INSERT OR REPLACE INTO artifact_checks VALUES (
      'thumb-01', 'demo', 'youtube-e2e', 'hash-thumb-01', 'default', 'passed', 0, 0, '[]', '${now}'
    );
    INSERT OR REPLACE INTO artifact_checks VALUES (
      'script-01', 'demo', 'youtube-e2e', 'hash-script-01', 'default', 'warning', 0, 1, '[{"code":"TEXT_TOO_LONG","message":"Kịch bản có thể hơi dài so với chuẩn 60s","severity":"warn"}]', '${now}'
    );
  `);
  wsDb.close();

  // 3. App-level SQLite: appdata/app.db
  const appDbPath = path.join(appDir, "app.db");
  const appDb = new DatabaseSync(appDbPath);

  appDb.exec(`
    CREATE TABLE IF NOT EXISTS schema_migrations (
      version INTEGER PRIMARY KEY,
      applied_at TEXT NOT NULL
    );
    INSERT OR IGNORE INTO schema_migrations VALUES (1, '${now}');

    CREATE TABLE IF NOT EXISTS recent_workspaces (
      path TEXT PRIMARY KEY,
      last_opened TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS roots (
      path TEXT PRIMARY KEY,
      name TEXT,
      created_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS orgs (
      path TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      created_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS settings (
      key TEXT PRIMARY KEY,
      value TEXT NOT NULL
    );
  `);

  const normWs = normalizePath(wsDir);
  appDb.exec(`
    INSERT OR REPLACE INTO recent_workspaces VALUES ('${normWs.replace(/'/g, "''")}', '${now}');
  `);
  appDb.close();
}

export async function startApp(options = {}) {
  const rootDir = path.resolve(".");
  // Own target dir: never the owner's `target/debug` (their `pnpm desktop` exe is locked while
  // running and is a dev build that loads Vite on 3001 instead of the bundled dist).
  const targetDir = E2E_TARGET_DIR;

  // Build if requested (default: true). Both builds are incremental, so always run them to
  // avoid testing a stale dist or exe.
  if (!options.noBuild) {
    console.log("[start] Building frontend dist...");
    execSync("pnpm --filter @studio/design-system build", { stdio: "inherit" });
    execSync("pnpm --filter @studio/desktop-ui build", { stdio: "inherit" });

    console.log("[start] Building studio-tauri with custom-protocol feature...");
    execSync("cargo build -p studio-tauri --features custom-protocol", {
      env: { ...process.env, CARGO_TARGET_DIR: targetDir },
      stdio: "inherit",
    });
  }

  // Allocate port and directories
  const port = options.port ? Number(options.port) : await getFreePort();
  const ts = new Date().toISOString().replace(/[:.]/g, "-");
  const runDir = path.resolve(".local/e2e-run", ts);
  const appDir = path.join(runDir, "appdata");
  const wvDir = path.join(runDir, "webview");
  const wsDir = path.join(runDir, "workspace");
  const shotsDir = path.join(runDir, "shots");

  mkdirSync(appDir, { recursive: true });
  mkdirSync(wvDir, { recursive: true });
  mkdirSync(wsDir, { recursive: true });
  mkdirSync(shotsDir, { recursive: true });

  console.log(`[start] Setting up isolated environment under: ${runDir}`);
  seedWorkspaceAndAppDb({ appDir, wsDir });

  const exePath = E2E_EXE;
  if (!existsSync(exePath)) {
    throw new Error(`Executable not found at ${exePath}. Run with build first.`);
  }

  console.log(`[start] Launching studio-tauri on CDP port ${port}...`);
  const logFile = path.join(runDir, "app.log");
  const outFd = openSync(logFile, "a");

  const child = spawn(exePath, [], {
    env: {
      ...process.env,
      STUDIO_APP_DIR: appDir,
      WEBVIEW2_USER_DATA_FOLDER: wvDir,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${port}`,
    },
    stdio: ["ignore", outFd, outFd],
    detached: false,
  });

  const pid = child.pid;
  // The CLI stays alive as the app's parent: under Git Bash the app dies with it otherwise.
  // Run `node scripts/e2e/start.mjs` in the background and end the app with stop.mjs.
  console.log(`[start] studio-tauri started (PID: ${pid}).`);
  // Start time pins the PID: stop.mjs refuses to kill a reused PID.
  const startedAt = getProcessInfo(pid)?.startedAt ?? null;

  // Write run.json
  const runData = {
    pid,
    port,
    timestamp: ts,
    runDir,
    appDir,
    wvDir,
    wsDir,
    shotsDir,
    exe: exePath,
    startedAt,
  };

  const runFile = path.join(runDir, "run.json");
  writeFileSync(runFile, JSON.stringify(runData, null, 2));

  mkdirSync(path.resolve(".local/e2e-run"), { recursive: true });
  writeFileSync(path.resolve(".local/e2e-run/run.json"), JSON.stringify(runData, null, 2));

  const latestDir = path.resolve(".local/e2e-run/latest");
  mkdirSync(latestDir, { recursive: true });
  writeFileSync(path.join(latestDir, "run.json"), JSON.stringify(runData, null, 2));

  // Wait for CDP and app ready
  console.log(`[start] Waiting for WebView2 CDP connection on port ${port}...`);
  const client = await CDPClient.connect({ port, retries: 40 });

  // Wait for page to finish loading and title to be set
  let ready = false;
  for (let i = 0; i < 40; i++) {
    try {
      const state = await client.eval(`({
        url: window.location.href,
        title: document.title,
        ready: document.readyState,
        hasRoot: Boolean(document.getElementById('root')?.children?.length)
      })`);
      if (state && state.ready === "complete" && state.hasRoot) {
        ready = true;
        break;
      }
    } catch {}
    await new Promise((r) => setTimeout(r, 250));
  }

  client.close();

  if (!ready) {
    console.warn("[start] WebView2 did not reach ready state within expected time.");
  } else {
    console.log(`[start] App is ready at http://tauri.localhost/`);
  }

  return runData;
}

// CLI entrypoint
if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve("scripts/e2e/start.mjs")) {
  const args = process.argv.slice(2);
  const noBuild = args.includes("--no-build");
  const forceBuild = args.includes("--force-build");
  const portIdx = args.indexOf("--port");
  const port = portIdx !== -1 ? args[portIdx + 1] : null;

  try {
    const data = await startApp({ noBuild, forceBuild, port });
    console.log(JSON.stringify(data, null, 2));
  } catch (err) {
    console.error("[start] Failed:", err.message);
    process.exit(1);
  }
}
