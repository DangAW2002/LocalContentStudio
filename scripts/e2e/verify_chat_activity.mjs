// scripts/e2e/verify_chat_activity.mjs
// Plan 0012 live check: Home → agy + playbook → project chat. Verifies the agent placeholder and
// cancel button show at once, the live activity line changes with tools, the prompt is not
// duplicated, the result card lists files and agy brain artifacts land in <project>/agy-artifacts.
// Needs a running isolated app (node scripts/e2e/start.mjs) and a logged-in agy.
import { CDPClient } from "./cdp.mjs";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";

const run = JSON.parse(readFileSync(path.resolve(".local/e2e-run/run.json"), "utf-8"));
const shots = run.shotsDir;
const PLAYBOOK = "Kịch bản Video ngắn E2E";
const PROMPT =
  `/${PLAYBOOK} Viết dàn ý 5 ý cho video ngắn về pha cà phê phin và lưu vào tệp dan-y.md trong thư mục dự án. ` +
  `Sau đó dùng công cụ generate_image tạo 1 ảnh thumbnail đơn giản cho video.`;
const TURN_TIMEOUT_MS = 10 * 60_000;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const results = { checks: [], activityLabels: [], timeline: [] };
const check = (name, ok, detail = "") => {
  results.checks.push({ name, ok, detail });
  console.log(`${ok ? "PASS" : "FAIL"} ${name}${detail ? ` — ${detail}` : ""}`);
};

const client = await CDPClient.connect({ port: run.port });

/** Set a React-controlled <select>/<textarea> value and fire the events React listens to. */
async function setField(selector, value) {
  return client.eval(`(() => {
    const el = ${selector};
    if (!el) return false;
    const proto = el.tagName === "SELECT" ? HTMLSelectElement.prototype : HTMLTextAreaElement.prototype;
    Object.getOwnPropertyDescriptor(proto, "value").set.call(el, ${JSON.stringify(value)});
    el.dispatchEvent(new Event("input", { bubbles: true }));
    el.dispatchEvent(new Event("change", { bubbles: true }));
    return true;
  })()`);
}
const selectWithOption = (v) =>
  `[...document.querySelectorAll("select")].find(s => [...s.options].some(o => o.value === ${JSON.stringify(v)}))`;

/** Snapshot of the chat pane. */
const chatState = () =>
  client.eval(`(() => {
    const dot = document.querySelector(".cs-thinking-dot");
    const activity = dot ? dot.parentElement.innerText.replace(/\\s+/g, " ").trim() : null;
    const cancel = [...document.querySelectorAll("button")].some(b => b.innerText.trim() === "Hủy lượt");
    const body = document.body.innerText;
    const result = [...document.querySelectorAll("div")].find(d => /^Kết quả · \\d+ tệp/.test(d.innerText || "") && d.children.length >= 2);
    return { activity, cancel, hasResult: Boolean(result), resultText: result ? result.innerText : null, body };
  })()`);

try {
  await client.setViewport(1440, 900);
  await client.clickText("Trang chủ", 3000).catch(() => {});
  await client.waitText("Giao việc cho agent", 15000);

  check("chọn engine agy", await setField(selectWithOption("agy"), "agy"));
  await sleep(1500); // model list reloads for the engine
  check("chọn playbook", await setField(selectWithOption(PLAYBOOK), PLAYBOOK));
  check("nhập prompt", await setField(`document.querySelector("textarea")`, PROMPT));
  await sleep(300);
  await client.shot(path.join(shots, "c01-home.png"));

  const t0 = Date.now();
  await client.eval(`document.querySelector("form button[type=submit]").click()`);

  // 1. Right after opening the project the agent must visibly be working.
  let first = null;
  for (let i = 0; i < 40; i++) {
    await sleep(250);
    const s = await chatState();
    if (s.activity || s.cancel) { first = { ...s, atMs: Date.now() - t0 }; break; }
  }
  check("hiện dòng hoạt động ngay khi mở dự án", Boolean(first?.activity), first ? `${first.atMs}ms: ${first.activity}` : "không thấy");
  check("có nút Hủy lượt khi đang chạy", Boolean(first?.cancel));
  await client.shot(path.join(shots, "c02-started.png"));

  const body0 = (await chatState()).body;
  const occurrences = body0.split(`/${PLAYBOOK}`).length - 1;
  check("prompt không lặp prefix playbook", occurrences === 1, `xuất hiện ${occurrences} lần`);
  check("không còn nhãn 'Bạn'", !/^Bạn$/m.test(body0));
  check("không còn hàng gợi ý @ Tệp / Playbook", !/@\s*Tệp\s*\/\s*Playbook/.test(body0));

  // 2. Follow the turn until it ends.
  let idleTicks = 0;
  let midShot = false;
  while (Date.now() - t0 < TURN_TIMEOUT_MS) {
    await sleep(1000);
    const s = await chatState();
    const sec = Math.round((Date.now() - t0) / 1000);
    if (s.activity) {
      const label = s.activity.replace(/·.*$/, "").trim();
      if (!results.activityLabels.includes(label)) {
        results.activityLabels.push(label);
        results.timeline.push(`${sec}s ${s.activity}`);
        console.log(`  [${sec}s] ${s.activity}`);
      }
      if (!midShot && /Đang (đọc|tạo|sửa|chạy|tìm|gọi|dùng)/.test(label)) {
        await client.shot(path.join(shots, "c03-tool-activity.png"));
        midShot = true;
      }
    }
    idleTicks = !s.activity && !s.cancel ? idleTicks + 1 : 0;
    if (idleTicks >= 4) break;
  }
  const doneSec = Math.round((Date.now() - t0) / 1000);
  check("lượt kết thúc trong thời hạn", idleTicks >= 4, `${doneSec}s`);
  check("dòng hoạt động đổi theo bước (≥2 nhãn)", results.activityLabels.length >= 2, results.activityLabels.join(" | "));

  // Artifact sync runs right after Done; give the fs events a moment.
  await sleep(3000);
  const end = await chatState();
  results.resultText = end.resultText;
  check("có thẻ Kết quả", end.hasResult, end.resultText?.replace(/\s+/g, " ").slice(0, 200) ?? "");
  const errorShown = /Engine báo lỗi/.test(end.body);
  check("lượt không báo lỗi engine", !errorShown);

  const projects = readdirSync(path.join(run.wsDir, "projects")).filter((d) => d !== "demo");
  const proj = projects.map((p) => path.join(run.wsDir, "projects", p)).find((p) => existsSync(path.join(p, "project.yaml")));
  const projYaml = proj ? readFileSync(path.join(proj, "project.yaml"), "utf-8") : "";
  check("tên dự án không bắt đầu bằng /playbook", !/^title: "?\//m.test(projYaml), projYaml.match(/^title:.*$/m)?.[0] ?? "");
  const artDir = proj && path.join(proj, "agy-artifacts");
  const artifacts = artDir && existsSync(artDir) ? readdirSync(artDir) : [];
  results.syncedArtifacts = artifacts;
  check("artifact brain agy đã đồng bộ về dự án", artifacts.length > 0, artifacts.join(", ") || "thư mục trống/không có");
  if (artifacts.length) {
    check("thẻ Kết quả liệt kê artifact đồng bộ", artifacts.some((a) => end.resultText?.includes(a)));
  }

  // 3. Layout at other widths + dark theme.
  await client.shot(path.join(shots, "c04-done-1440.png"));
  await client.setViewport(390, 844);
  await sleep(400);
  await client.shot(path.join(shots, "c05-done-390.png"));
  const overflow = await client.eval(`document.documentElement.scrollWidth > window.innerWidth + 1`);
  check("390px không tràn ngang", !overflow);
  await client.setViewport(1440, 900);
  await client.eval(`document.documentElement.setAttribute('data-theme', 'dark')`);
  await sleep(300);
  await client.shot(path.join(shots, "c06-done-dark.png"));

  const errs = await client.checkErrors();
  check("không lỗi console/undefined/NaN", errs.ok, errs.reason || "");
} catch (err) {
  check("kịch bản chạy hết", false, err.message);
  await client.shot(path.join(shots, "c99-error.png")).catch(() => {});
} finally {
  writeFileSync(path.join(shots, "chat-activity-results.json"), JSON.stringify(results, null, 2));
  client.close();
  const failed = results.checks.filter((c) => !c.ok).length;
  console.log(`\n${results.checks.length - failed}/${results.checks.length} PASS — ảnh & kết quả: ${shots}`);
  process.exit(failed ? 1 : 0);
}
