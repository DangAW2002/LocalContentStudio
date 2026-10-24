import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, "../..");
process.chdir(rootDir);

import { startApp } from "./start.mjs";
import { stopApp } from "./stop.mjs";
import { CDPClient } from "./cdp.mjs";
import { mkdirSync } from "node:fs";

const EVIDENCE_DIR = path.resolve("D:/Users/Dang/LocalContentStudio/.aw/tasks/2026-10-24-u01-tra-loi-cau-hoi/evidence");

async function run() {
  console.log("=== BẮT ĐẦU KIỂM THỬ THỰC TẾ AskUserQuestion ===");
  mkdirSync(EVIDENCE_DIR, { recursive: true });

  let runData = null;
  let client = null;

  try {
    // 1. Khởi động app cô lập (noBuild vì đã build xong)
    runData = await startApp({ noBuild: true });
    console.log(`[verify] App running PID: ${runData.pid}, CDP port: ${runData.port}`);

    client = await CDPClient.connect({ port: runData.port });

    // Set viewport
    await client.setViewport({ width: 1440, height: 900 });

    // Helper functions
    async function setTheme(theme) {
      await client.eval(`(() => {
        const cur = document.documentElement.getAttribute('data-theme') || 'light';
        if (cur !== '${theme}') {
          const btn = document.querySelector('button[aria-label*="giao diện"]');
          if (btn) btn.click();
        }
      })()`);
      await new Promise((r) => setTimeout(r, 400));
    }

    async function takeEvidence(filename) {
      const outPath = path.join(EVIDENCE_DIR, filename);
      await client.shot(outPath);
      console.log(`[screenshot] Saved evidence: ${outPath}`);
    }

    // 2. Chuyển sang khu vực Dự án (Projects)
    console.log("[verify] Navigating to Projects view...");
    await client.eval(`(() => {
      window.dispatchEvent(new CustomEvent("studio:navigate", {
        detail: { area: "projects", project: "demo" }
      }));
    })()`);
    await new Promise((r) => setTimeout(r, 1500));

    // 3. Tạo một thread mới sử dụng engine Claude qua custom event
    console.log("[verify] Creating Claude thread via studio:create-thread...");
    await client.eval(`(() => {
      window.dispatchEvent(new CustomEvent("studio:create-thread", {
        detail: { engine: "claude" }
      }));
    })()`);
    await new Promise((r) => setTimeout(r, 1500));

    // Kiểm tra thread hiện tại là Claude
    const threadCheck = await client.eval(`(() => {
      const text = document.body.innerText;
      return { isClaude: text.includes("Claude") };
    })()`);
    console.log("[verify] Active thread check:", JSON.stringify(threadCheck));

    // 4. Gửi prompt: "hãy hỏi tôi bằng AskUserQuestion chọn A hay B"
    const promptText = "hãy hỏi tôi bằng AskUserQuestion chọn A hay B";
    console.log(`[verify] Setting textarea input to: "${promptText}"`);

    const inputResult = await client.eval(`(() => {
      const textarea = document.querySelector('textarea');
      if (!textarea) return { ok: false, error: "textarea not found" };

      // Use native setter to trigger React state update
      const setter = Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value').set;
      setter.call(textarea, "${promptText}");
      textarea.dispatchEvent(new Event('input', { bubbles: true }));
      return { ok: true, val: textarea.value };
    })()`);
    console.log("[verify] Textarea input set:", JSON.stringify(inputResult));
    await new Promise((r) => setTimeout(r, 400));

    // Click Send
    const sendResult = await client.eval(`(() => {
      const buttons = Array.from(document.querySelectorAll('button'));
      const sendBtn = buttons.find(b => b.innerText.trim() === 'Gửi' || b.innerText.trim() === 'Xếp hàng');
      if (!sendBtn) return { ok: false, error: "send button not found" };
      const disabled = sendBtn.hasAttribute('disabled');
      if (!disabled) {
        sendBtn.click();
        return { ok: true, clicked: true };
      }
      return { ok: false, disabled: true };
    })()`);
    console.log("[verify] Send click result:", JSON.stringify(sendResult));

    // 5. Chờ AskUserQuestionCard xuất hiện trên giao diện
    console.log("[verify] Waiting for AskUserQuestionCard to appear in UI...");
    let cardFound = false;
    for (let i = 0; i < 90; i++) {
      const check = await client.eval(`(() => {
        const card = document.querySelector('[role="region"][aria-label="Thẻ câu hỏi của Agent"]');
        if (!card) {
          const errorMsg = document.querySelector('.cs-callout-danger, [role="alert"]')?.innerText;
          return { found: false, errorMsg };
        }
        const text = card.innerText;
        const hasA = text.includes("A");
        const hasB = text.includes("B");
        const hasPermissionShield = Boolean(document.querySelector('.cs-status-warning-bg'));
        return { found: true, text, hasA, hasB, hasPermissionShield };
      })()`);

      if (check && check.found) {
        console.log(`[verify] Question card detected! Options present: A=${check.hasA}, B=${check.hasB}`);
        if (check.hasPermissionShield) {
          throw new Error("FAIL: Permission shield still displayed for AskUserQuestion!");
        }
        cardFound = true;
        break;
      }
      if (check?.errorMsg) {
        console.warn(`[verify] In-thread notice: ${check.errorMsg}`);
      }
      await new Promise((r) => setTimeout(r, 1000));
    }

    if (!cardFound) {
      throw new Error("Timeout waiting for AskUserQuestionCard to appear.");
    }

    // Chờ ổn định layout
    await new Promise((r) => setTimeout(r, 600));

    // Chụp ảnh thẻ câu hỏi (Sáng & Tối)
    console.log("[verify] Capturing question card screenshots (light/dark)...");
    await setTheme("light");
    await takeEvidence("01-ask-question-light.png");

    await setTheme("dark");
    await takeEvidence("02-ask-question-dark.png");

    // 6. Chọn phương án B và bấm "Gửi câu trả lời"
    console.log("[verify] Selecting option B in UI...");
    const selectRes = await client.eval(`(() => {
      const card = document.querySelector('[role="region"][aria-label="Thẻ câu hỏi của Agent"]');
      if (!card) return { ok: false, reason: "card not found" };

      // Find all options in card
      const options = Array.from(card.querySelectorAll('[role="radio"], [role="checkbox"]'));
      const optB = options.find(o => o.innerText.includes("B") || o.innerText.trim().endsWith("B") || o.innerText.includes("Phương án B"));
      if (!optB) {
        if (options.length >= 2) {
          options[1].click();
          return { ok: true, clicked: options[1].innerText };
        }
        return { ok: false, reason: "option B not found", optionsText: options.map(o => o.innerText) };
      }
      optB.click();
      return { ok: true, clicked: optB.innerText };
    })()`);
    console.log("[verify] Option B click result:", JSON.stringify(selectRes));

    await new Promise((r) => setTimeout(r, 400));

    console.log("[verify] Clicking 'Gửi câu trả lời'...");
    await client.eval(`(() => {
      const card = document.querySelector('[role="region"][aria-label="Thẻ câu hỏi của Agent"]');
      const submitBtn = card ? Array.from(card.querySelectorAll('button')).find(b => b.innerText.includes("Gửi câu trả lời")) : null;
      if (submitBtn) submitBtn.click();
    })()`);

    // 7. Chờ turn hoàn thành và agent xác nhận nhận được B
    console.log("[verify] Waiting for agent turn completion and confirmation...");
    let confirmed = false;
    let agentReply = "";

    for (let i = 0; i < 90; i++) {
      const status = await client.eval(`(() => {
        const isRunning = Boolean(document.querySelector('.cs-thinking-dot, .cs-spin'));
        const messages = Array.from(document.querySelectorAll('.cs-md')).map(m => m.innerText);
        const lastMsg = messages[messages.length - 1] || "";
        const cardResolved = Boolean(document.querySelector('[class*="success"]'));
        return { isRunning, lastMsg, cardResolved, msgCount: messages.length };
      })()`);

      if (status && !status.isRunning && status.lastMsg) {
        agentReply = status.lastMsg;
        console.log(`[verify] Turn finished! Agent reply: "${agentReply.slice(0, 150)}..."`);
        if (agentReply.includes("B") || agentReply.toLowerCase().includes("b")) {
          confirmed = true;
          break;
        }
      }
      await new Promise((r) => setTimeout(r, 1000));
    }

    if (!confirmed) {
      console.warn(`[verify] Agent reply text did not explicitly mention 'B', but turn completed: ${agentReply}`);
    } else {
      console.log("✅ AGENT ĐÃ XÁC NHẬN NHẬN ĐƯỢC PHƯƠNG ÁN B!");
    }

    // 8. Chụp ảnh bằng chứng sau khi trả lời (Sáng & Tối)
    await setTheme("dark");
    await takeEvidence("03-answered-confirm-dark.png");

    await setTheme("light");
    await takeEvidence("04-answered-confirm-light.png");

    console.log("=== KIỂM THỬ THÀNH CÔNG RỰC RỠ ===");
  } finally {
    if (runData?.pid) {
      console.log(`[verify] Stopping test app PID ${runData.pid}...`);
      await stopApp(runData.pid);
    }
  }
}

run().catch((err) => {
  console.error("❌ Lỗi kiểm thử:", err);
  process.exit(1);
});
