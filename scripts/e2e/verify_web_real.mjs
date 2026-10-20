import { startApp } from "./start.mjs";
import { CDPClient } from "./cdp.mjs";
import { stopApp } from "./stop.mjs";
import { execSync } from "node:child_process";
import path from "node:path";
import { mkdirSync, writeFileSync } from "node:fs";

async function runRealVerification() {
  console.log("=== BẮT ĐẦU VERIFICATION WEB MODE REAL APP ===");
  const evidenceDir = path.resolve("D:/Users/Dang/LocalContentStudio/.aw/tasks/2026-10-18-m10-web-https/evidence");
  mkdirSync(evidenceDir, { recursive: true });

  const runData = await startApp({ noBuild: true });
  console.log(`[verify] App running PID ${runData.pid} port ${runData.port}`);
  const client = await CDPClient.connect({ port: runData.port });

  try {
    // 1. Configure Web mode with LAN enabled and password
    console.log("[verify] Configuring web mode via invoke...");
    const configRes = await client.invoke("web_configure", {
      req: {
        enabled: true,
        lanAllowed: true,
        port: 3456,
        password: "OwnerSuperSecret123!",
        uploadLimitMb: 50,
        jsonLimitMb: 2,
      }
    });
    console.log("[verify] Configure result:", JSON.stringify(configRes, null, 2));

    // Wait 1s for server to start
    await new Promise(r => setTimeout(r, 1000));

    // Check status
    const status = await client.invoke("web_status", {});
    console.log("[verify] Web Status:", JSON.stringify(status, null, 2));

    // Navigate UI to Settings -> Web tab
    console.log("[verify] Navigating to Settings web tab...");
    await client.key("5", ["ctrl"]);
    await new Promise(r => setTimeout(r, 800));

    // Click Web tab
    await client.clickText("Truy cập Web");
    await new Promise(r => setTimeout(r, 800));

    // The section must reflect the backend state (field names once mismatched: "undefined", LAN off).
    const pageText = await client.eval(`document.body.innerText`);
    for (const want of ["https://localhost:3456", "Đang bật"]) {
      if (!pageText.includes(want)) throw new Error(`Web settings UI lacks "${want}"`);
    }
    for (const bad of ["undefined", "Chưa cấu hình mật khẩu"]) {
      if (pageText.includes(bad)) throw new Error(`Web settings UI shows "${bad}"`);
    }
    console.log("[verify] Web settings UI matches backend state");

    // Capture Light shot
    await client.eval(`document.documentElement.setAttribute('data-theme', 'light')`);
    await new Promise(r => setTimeout(r, 300));
    await client.shot(path.join(evidenceDir, "web_settings_light_1440.png"));
    console.log("[verify] Saved light screenshot");

    // Capture Dark shot
    await client.eval(`document.documentElement.setAttribute('data-theme', 'dark')`);
    await new Promise(r => setTimeout(r, 300));
    await client.shot(path.join(evidenceDir, "web_settings_dark_1440.png"));
    console.log("[verify] Saved dark screenshot");

    // 2. Real HTTPS checks with curl (-k: self-signed). Payloads go through files, not argv.
    console.log("\n=== TESTING CURL HTTPS ===");
    const work = path.resolve(".local/verify-web");
    mkdirSync(work, { recursive: true });
    const jar = path.join(work, "cookies.txt");
    const base = "https://127.0.0.1:3456";
    const H = '-H "Host: 127.0.0.1:3456" -H "x-studio-csrf: 1"';
    const curl = (args) =>
      execSync(`curl -k -s -o NUL -D - ${args}`, { encoding: "utf8", maxBuffer: 1 << 20 });
    // Last status line: curl may print an interim "100 Continue" first.
    const code = (out) => Number([...out.matchAll(/^HTTP\/[\d.]+ (\d{3})/gm)].pop()?.[1]);
    const expect = (label, out, want) => {
      const got = code(out);
      console.log(`[curl] ${label}: ${got}`);
      if (got !== want) throw new Error(`${label}: expected ${want}, got ${got}\n${out}`);
      return out;
    };

    expect("GET / over https", curl(`"${base}/"`), 200);
    expect("protected route without session", curl(`-X POST ${H} "${base}/api/invoke/project_list"`), 401);
    // A well-formed body, so the request reaches the Origin check (axum rejects a bad body first).
    writeFileSync(path.join(work, "login.json"), JSON.stringify({ password: "OwnerSuperSecret123!" }));
    expect(
      "cross-origin POST",
      curl(`-X POST ${H} -H "Origin: https://evil.example" -H "Content-Type: application/json" --data-binary @"${path.join(work, "login.json")}" "${base}/api/auth/login"`),
      403,
    );

    const login = expect(
      "login",
      curl(`-c "${jar}" -X POST ${H} -H "Content-Type: application/json" --data-binary @"${path.join(work, "login.json")}" "${base}/api/auth/login"`),
      200,
    );
    if (!/set-cookie:[^\n]*studio_session=[^\n]*;\s*Secure/i.test(login)) throw new Error("session cookie lacks Secure over https");
    console.log("[curl] session cookie has Secure");

    const big = path.join(work, "big.bin");
    writeFileSync(big, Buffer.alloc(51 * 1024 * 1024)); // limit configured to 50 MB
    expect("upload over limit", curl(`-b "${jar}" ${H} -F "file=@${big}" "${base}/api/upload"`), 413);
    writeFileSync(path.join(work, "big.json"), JSON.stringify({ password: "a".repeat(3 * 1024 * 1024) }));
    expect(
      "JSON over limit",
      curl(`-X POST ${H} -H "Content-Type: application/json" --data-binary @"${path.join(work, "big.json")}" "${base}/api/auth/login"`),
      413,
    );
    const small = path.join(work, "small.png");
    writeFileSync(small, Buffer.from("89504e470d0a1a0a", "hex"));
    expect("upload under limit", curl(`-b "${jar}" ${H} -F "file=@${small}" "${base}/api/upload"`), 200);

    console.log("\n✅ ALL REAL VERIFICATIONS PASSED!");

  } finally {
    console.log("[verify] Stopping app...");
    await stopApp();
  }
}

runRealVerification().catch(e => {
  console.error("Verification failed:", e);
  stopApp().finally(() => process.exit(1));
});
