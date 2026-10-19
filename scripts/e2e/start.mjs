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
    `slug: demo\ntitle: "Dự án mẫu E2E"\ncreated_at: "${now}"\nkit: null\nmodules: []\n`
  );

  // sample artifact
  writeFileSync(
    path.join(wsDir, "projects/demo/artifacts/thumb-01/artifact.yaml"),
    `id: thumb-01\nkind: design\ntitle: "Thumbnail E2E"\nbrand: demo\nchannels:\n  - youtube\nsource: index.html\n`
  );
  writeFileSync(
    path.join(wsDir, "projects/demo/artifacts/thumb-01/index.html"),
    `<!DOCTYPE html><html><body style="background:#1e1e2e;color:#fff;display:flex;align-items:center;justify-content:center;height:100vh;"><h1>Thumbnail E2E</h1></body></html>`
  );

  // 2. Workspace SQLite: .studio/studio.db
  const wsDbPath = path.join(wsDir, ".studio/studio.db");
  const wsDb = new DatabaseSync(wsDbPath);

  wsDb.exec(`
    CREATE TABLE IF NOT EXISTS schema_migrations (
      version INTEGER PRIMARY KEY,
      checksum TEXT NOT NULL,
      applied_at TEXT NOT NULL
    );
    INSERT OR IGNORE INTO schema_migrations VALUES (1, 'init', '${now}'), (2, 'v2', '${now}');

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
      completed_at TEXT
    );

    CREATE TABLE IF NOT EXISTS events (
      id TEXT PRIMARY KEY,
      turn_id TEXT NOT NULL,
      event_type TEXT NOT NULL,
      payload_json TEXT NOT NULL,
      created_at TEXT NOT NULL
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

    CREATE TABLE IF NOT EXISTS assets_index (
      id TEXT PRIMARY KEY,
      path TEXT NOT NULL,
      name TEXT NOT NULL,
      mime_type TEXT NOT NULL,
      size_bytes INTEGER NOT NULL,
      sha256 TEXT NOT NULL,
      metadata_json TEXT,
      updated_at TEXT NOT NULL
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

    CREATE TABLE IF NOT EXISTS jobs (
      id TEXT PRIMARY KEY,
      kind TEXT NOT NULL,
      status TEXT NOT NULL,
      priority INTEGER NOT NULL DEFAULT 0,
      input_json TEXT NOT NULL,
      output_json TEXT,
      error_message TEXT,
      created_at TEXT NOT NULL,
      updated_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS checkpoints (
      id TEXT PRIMARY KEY,
      label TEXT NOT NULL,
      turn_id TEXT,
      pinned INTEGER NOT NULL DEFAULT 0,
      created_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS checkpoint_files (
      checkpoint_id TEXT NOT NULL,
      rel_path TEXT NOT NULL,
      blob_sha256 TEXT NOT NULL,
      size_bytes INTEGER NOT NULL,
      PRIMARY KEY (checkpoint_id, rel_path)
    );

    CREATE TABLE IF NOT EXISTS cost_ledger (
      id TEXT PRIMARY KEY,
      turn_id TEXT,
      job_id TEXT,
      provider TEXT NOT NULL,
      model TEXT,
      cost_usd REAL NOT NULL,
      created_at TEXT NOT NULL
    );
  `);

  // Insert seed thread, turn, event, artifact, checkpoint
  wsDb.exec(`
    INSERT OR REPLACE INTO threads VALUES (
      'demo-thread-1', 'demo', 'Hội thoại kiểm thử E2E', 'agy', 'gemini-3.8-flash-medium', '${now}', '${now}'
    );
    INSERT OR REPLACE INTO turns VALUES (
      'demo-turn-1', 'demo-thread-1', 1, 'Xin chào, hãy dựng một video quảng bá.', 'done', '${now}', '${now}'
    );
    INSERT OR REPLACE INTO events VALUES (
      'demo-evt-1', 'demo-turn-1', 'message', '{"text":"Chào mừng bạn đến với Local Content Studio! Tôi đã chuẩn bị sẵn kịch bản và khung làm việc."}', '${now}'
    );
    INSERT OR REPLACE INTO artifacts_index VALUES (
      'thumb-01', 'demo', 'design', 'Thumbnail E2E', 'projects/demo/artifacts/thumb-01', 'demo', '["youtube"]', 'index.html', 'design', '${now}'
    );
    INSERT OR REPLACE INTO checkpoints VALUES (
      'demo-cp-1', 'Khởi tạo ban đầu', 'demo-turn-1', 1, '${now}'
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
