// scripts/e2e/stop.mjs
// Safely stops the isolated E2E app instance recorded in run.json.
// HARD SAFETY RULE: Never kill processes by image name.
// Only kill the exact PID from run.json, after verifying the exe path is inside this worktree!
import { existsSync, readFileSync, rmSync, unlinkSync } from "node:fs";
import { execSync } from "node:child_process";
import path from "node:path";

function findRunJson(runArg) {
  if (runArg && existsSync(runArg)) return path.resolve(runArg);
  const candidates = [
    path.resolve(".local/e2e-run/run.json"),
    path.resolve(".local/e2e-run/latest/run.json"),
  ];
  for (const c of candidates) {
    if (existsSync(c)) return c;
  }
  return null;
}

/**
 * Dedicated build dir for E2E runs, separate from the owner's `target/debug`.
 * `STUDIO_E2E_TARGET_DIR` lets a second session build and smoke-test while another E2E app runs.
 */
export const E2E_TARGET_DIR = path.resolve(process.env.STUDIO_E2E_TARGET_DIR || "target/e2e");
export const E2E_EXE = path.join(
  E2E_TARGET_DIR,
  "debug",
  process.platform === "win32" ? "studio-tauri.exe" : "studio-tauri",
);

/** Exe path and start time (ms since epoch) of a live process, or null. */
export function getProcessInfo(pid) {
  try {
    if (process.platform === "win32") {
      const ps = `$p = Get-Process -Id ${Number(pid)} -ErrorAction Stop; Write-Output $p.Path; Write-Output ([DateTimeOffset]$p.StartTime).ToUnixTimeMilliseconds()`;
      const out = execSync(`powershell -NoProfile -Command "${ps}"`, {
        encoding: "utf-8",
        stdio: ["ignore", "pipe", "ignore"],
      }).trim().split(/\r?\n/);
      if (!out[0]) return null;
      return { exe: out[0].trim(), startedAt: Number(out[1]) || null };
    }
    const exe = execSync(`readlink -f /proc/${Number(pid)}/exe`, {
      encoding: "utf-8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    return exe ? { exe, startedAt: null } : null;
  } catch {
    return null;
  }
}

export async function stopApp(options = {}) {
  const runFile = findRunJson(options.runFile);
  if (!runFile) {
    console.log("[stop] No active run.json found; nothing to stop.");
    return;
  }

  let runData;
  try {
    runData = JSON.parse(readFileSync(runFile, "utf-8"));
  } catch (err) {
    console.warn(`[stop] Failed to parse ${runFile}:`, err.message);
    return;
  }

  const pid = runData.pid;
  if (!pid) {
    console.warn("[stop] No PID in run.json");
    try { unlinkSync(runFile); } catch {}
    return;
  }

  console.log(`[stop] Inspecting process PID ${pid}...`);
  const info = getProcessInfo(pid);
  const exePath = info?.exe;

  if (exePath) {
    // HARD SAFETY CHECK: the exe must be the E2E build (never the owner's target/debug app) and
    // the process must be the one we started (same start time), not a reused PID.
    const actualExe = path.resolve(exePath).toLowerCase();
    const isE2eBuild = actualExe === path.resolve(E2E_EXE).toLowerCase();
    const sameProcess =
      runData.startedAt == null || info.startedAt == null || Math.abs(info.startedAt - runData.startedAt) < 2000;
    if (!isE2eBuild || !sameProcess) {
      console.error(`[stop] SAFETY VIOLATION PREVENTED!`);
      console.error(`[stop] PID ${pid} has exe: ${exePath} (expected ${E2E_EXE}), sameProcess=${sameProcess}`);
      console.error(`[stop] Refusing to kill external process!`);
      throw new Error(`Safety violation: PID ${pid} is not the E2E app started by this run.`);
    }

    console.log(`[stop] Verified exe is inside worktree (${exePath}). Terminating process tree...`);
    try {
      if (process.platform === "win32") {
        execSync(`taskkill /PID ${pid} /T /F`, { stdio: "ignore" });
      } else {
        process.kill(pid, "SIGKILL");
      }
      console.log(`[stop] Terminated PID ${pid}.`);
    } catch (err) {
      console.log(`[stop] Process ${pid} might already be stopped.`);
    }
  } else {
    console.log(`[stop] Process PID ${pid} is not running.`);
  }

  // Remove run.json
  try {
    unlinkSync(runFile);
  } catch {}

  // If latest/run.json exists and is different, remove it too
  const latestRun = path.resolve(".local/e2e-run/latest/run.json");
  if (existsSync(latestRun)) {
    try { unlinkSync(latestRun); } catch {}
  }

  // If --clean, remove the temporary run directory
  if (options.clean && runData.runDir && existsSync(runData.runDir)) {
    console.log(`[stop] Cleaning temporary directory: ${runData.runDir}`);
    try {
      rmSync(runData.runDir, { recursive: true, force: true });
    } catch (err) {
      console.warn(`[stop] Could not clean ${runData.runDir}:`, err.message);
    }
  }
}

// CLI entrypoint
if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve("scripts/e2e/stop.mjs")) {
  const args = process.argv.slice(2);
  const clean = args.includes("--clean");
  const runIdx = args.indexOf("--run");
  const runFile = runIdx !== -1 ? args[runIdx + 1] : null;

  await stopApp({ clean, runFile });
}
