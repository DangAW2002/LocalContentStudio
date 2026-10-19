// scripts/e2e/cdp.mjs
// Minimal CDP driver for WebView2 in Local Content Studio.
// Works both as an imported ES module and as a CLI script.
import { writeFileSync, readFileSync, existsSync } from "node:fs";
import path from "node:path";

function resolveCdpPort(portArg) {
  if (portArg) return portArg;
  if (process.env.CDP_PORT) return process.env.CDP_PORT;

  // Try reading from .local/e2e-run/run.json or .local/e2e-run/latest/run.json
  const candidates = [
    path.resolve(".local/e2e-run/run.json"),
    path.resolve(".local/e2e-run/latest/run.json"),
  ];
  for (const p of candidates) {
    if (existsSync(p)) {
      try {
        const data = JSON.parse(readFileSync(p, "utf-8"));
        if (data.port) return String(data.port);
      } catch {}
    }
  }

  return "9333";
}

export class CDPClient {
  constructor() {
    this.ws = null;
    this.nextId = 1;
    this.pending = new Map();
    this.consoleErrors = [];
    this.page = null;
  }

  static async connect(options = {}) {
    const port = resolveCdpPort(options.port);
    let page = null;
    const maxRetries = options.retries ?? 30;

    for (let i = 0; i < maxRetries; i++) {
      try {
        const res = await fetch(`http://127.0.0.1:${port}/json`);
        const targets = await res.json();
        page = targets.find((t) => t.type === "page");
        if (page && page.webSocketDebuggerUrl) break;
      } catch {}
      await new Promise((r) => setTimeout(r, 300));
    }

    if (!page) {
      throw new Error(`Cannot connect to CDP on port ${port} (no page target found).`);
    }

    const client = new CDPClient();
    client.page = page;

    const ws = new WebSocket(page.webSocketDebuggerUrl);
    await new Promise((resolve, reject) => {
      ws.onopen = resolve;
      ws.onerror = reject;
    });
    client.ws = ws;

    ws.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);
        if (msg.id && client.pending.has(msg.id)) {
          client.pending.get(msg.id)(msg);
          client.pending.delete(msg.id);
        }

        // Track console errors & unhandled exceptions
        if (msg.method === "Runtime.consoleAPICalled" && msg.params?.type === "error") {
          const args = msg.params.args?.map((a) => a.value ?? a.description ?? JSON.stringify(a)).join(" ");
          client.consoleErrors.push(args || "Unknown console.error");
        }
        if (msg.method === "Runtime.exceptionThrown" && msg.params?.exceptionDetails) {
          const desc = msg.params.exceptionDetails.exception?.description || msg.params.exceptionDetails.text;
          client.consoleErrors.push(desc || "Unhandled exception");
        }
      } catch {}
    };

    // Enable Runtime and Page domains
    await client.send("Runtime.enable");
    await client.send("Page.enable");

    return client;
  }

  send(method, params = {}) {
    return new Promise((resolve, reject) => {
      if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
        return reject(new Error("WebSocket is not connected"));
      }
      const id = this.nextId++;
      this.pending.set(id, resolve);
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }

  async eval(expression) {
    const res = await this.send("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
    });
    if (res.result?.exceptionDetails) {
      const ex = res.result.exceptionDetails.exception?.description || res.result.exceptionDetails.text;
      throw new Error(`Eval failed: ${ex}`);
    }
    return res.result?.result?.value;
  }

  async invoke(cmd, args = {}) {
    const expr = `(async () => {
      const internals = window.__TAURI_INTERNALS__ || window.__TAURI__?.core;
      if (!internals) throw new Error("Tauri internals not available on window");
      return await internals.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(args)});
    })()`;
    return await this.eval(expr);
  }

  async shot(outPath) {
    const res = await this.send("Page.captureScreenshot", { format: "png" });
    const buffer = Buffer.from(res.result.data, "base64");
    writeFileSync(outPath, buffer);
    return outPath;
  }

  async mouse(type, x, y, extra = {}) {
    return await this.send("Input.dispatchMouseEvent", {
      type,
      x,
      y,
      button: "left",
      clickCount: 1,
      ...extra,
    });
  }

  async click(x, y) {
    await this.mouse("mouseMoved", x, y, { button: "none" });
    await this.mouse("mousePressed", x, y);
    await this.mouse("mouseReleased", x, y);
  }

  async clickText(text, timeoutMs = 5000) {
    const startTime = Date.now();
    while (Date.now() - startTime < timeoutMs) {
      const found = await this.eval(`(() => {
        const norm = ${JSON.stringify(text)}.trim().toLowerCase();
        const all = [...document.querySelectorAll('*')];
        const matching = all.filter(el => {
          if (!el.textContent || !el.textContent.toLowerCase().includes(norm)) return false;
          const rect = el.getBoundingClientRect();
          return rect.width > 0 && rect.height > 0;
        });
        const inner = matching.filter(el => ![...el.children].some(c => {
          const r = c.getBoundingClientRect();
          return r.width > 0 && r.height > 0 && c.textContent && c.textContent.toLowerCase().includes(norm);
        }));
        if (inner.length === 0) return null;
        const target = inner[0].closest('button, [role=button], [role=tab], a, input, select') || inner[0];
        target.scrollIntoView({ block: 'center', inline: 'center' });
        target.click();
        return { tag: target.tagName, text: (target.textContent || '').trim().slice(0, 40) };
      })()`);

      if (found) {
        return found;
      }
      await new Promise((r) => setTimeout(r, 200));
    }
    throw new Error(`clickText("${text}") timed out after ${timeoutMs}ms (element not found)`);
  }

  async key(key, mods = []) {
    const modifiers =
      (mods.includes("alt") ? 1 : 0) |
      (mods.includes("ctrl") ? 2 : 0) |
      (mods.includes("meta") ? 4 : 0) |
      (mods.includes("shift") ? 8 : 0);
    const code = key.length === 1 ? `Key${key.toUpperCase()}` : key;
    const vkMap = {
      Escape: 27,
      Enter: 13,
      Tab: 9,
      Space: 32,
      ArrowLeft: 37,
      ArrowUp: 38,
      ArrowRight: 39,
      ArrowDown: 40,
    };
    const vk = key.length === 1 ? key.toUpperCase().charCodeAt(0) : (vkMap[key] || 0);

    await this.send("Input.dispatchKeyEvent", {
      type: "rawKeyDown",
      key,
      code,
      modifiers,
      windowsVirtualKeyCode: vk,
    });
    await this.send("Input.dispatchKeyEvent", {
      type: "keyUp",
      key,
      code,
      modifiers,
      windowsVirtualKeyCode: vk,
    });
  }

  async type(text) {
    await this.send("Input.insertText", { text });
  }

  async wheel(x, y, deltaY) {
    await this.send("Input.dispatchMouseEvent", {
      type: "mouseWheel",
      x,
      y,
      deltaX: 0,
      deltaY,
    });
  }

  async drag(x1, y1, x2, y2, steps = 12) {
    await this.mouse("mouseMoved", x1, y1, { button: "none" });
    await this.mouse("mousePressed", x1, y1);
    for (let i = 1; i <= steps; i++) {
      const cx = x1 + ((x2 - x1) * i) / steps;
      const cy = y1 + ((y2 - y1) * i) / steps;
      await this.mouse("mouseMoved", cx, cy, { buttons: 1 });
      await new Promise((r) => setTimeout(r, 16));
    }
    await this.mouse("mouseReleased", x2, y2);
  }

  async waitText(text, timeoutMs = 10000) {
    const startTime = Date.now();
    const norm = text.trim().toLowerCase();
    while (Date.now() - startTime < timeoutMs) {
      const has = await this.eval(`document.body.innerText.toLowerCase().includes(${JSON.stringify(norm)})`);
      if (has) return true;
      await new Promise((r) => setTimeout(r, 200));
    }
    throw new Error(`waitText("${text}") timed out after ${timeoutMs}ms`);
  }

  async setViewport(width, height) {
    await this.send("Emulation.setDeviceMetricsOverride", {
      width,
      height,
      deviceScaleFactor: 1,
      mobile: false,
    });
    // Trigger window resize event in DOM
    await this.eval("window.dispatchEvent(new Event('resize'))");
    await new Promise((r) => setTimeout(r, 150));
  }

  async checkErrors() {
    if (this.consoleErrors.length > 0) {
      const list = [...this.consoleErrors];
      this.consoleErrors = [];
      return { ok: false, reason: `Console errors: ${list.join("; ")}` };
    }

    const check = await this.eval(`(() => {
      // 1. Text checks
      const bodyText = document.body.innerText || "";
      if (bodyText.includes("Tệp nằm ngoài workspace")) {
        return { ok: false, reason: 'Phát hiện lỗi "Tệp nằm ngoài workspace"' };
      }
      // Every seeded/default channel declares formats; this text means the brand set failed to load.
      if (bodyText.includes("Kênh này chưa khai báo định dạng khung hình")) {
        return { ok: false, reason: "Kênh không có định dạng (BrandSet không tải được?)" };
      }

      // Check text nodes for undefined / NaN / [object Object]
      const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
      let node;
      const badPatterns = [
        { pattern: /\\bundefined\\b/, name: 'undefined' },
        { pattern: /\\bNaN\\b/, name: 'NaN' },
        { pattern: /\\[object Object\\]/, name: '[object Object]' }
      ];
      while ((node = walker.nextNode())) {
        const p = node.parentElement;
        if (!p || p.tagName === 'SCRIPT' || p.tagName === 'STYLE') continue;
        const text = node.nodeValue || '';
        for (const { pattern, name } of badPatterns) {
          if (pattern.test(text)) {
            return { ok: false, reason: 'Phát hiện "' + name + '" trên màn hình: "' + text.trim().slice(0, 60) + '"' };
          }
        }
      }

      // 2. Horizontal scroll overflow check on document and root
      const docEl = document.documentElement;
      const body = document.body;
      const root = document.getElementById('root');
      if (docEl.scrollWidth > docEl.clientWidth + 1) {
        return { ok: false, reason: 'Tràn ngang trên documentElement (' + docEl.scrollWidth + ' > ' + docEl.clientWidth + ')' };
      }
      if (body.scrollWidth > body.clientWidth + 1) {
        return { ok: false, reason: 'Tràn ngang trên body (' + body.scrollWidth + ' > ' + body.clientWidth + ')' };
      }
      if (root && root.scrollWidth > root.clientWidth + 1) {
        return { ok: false, reason: 'Tràn ngang trên #root (' + root.scrollWidth + ' > ' + root.clientWidth + ')' };
      }

      return { ok: true };
    })()`);

    return check;
  }

  close() {
    if (this.ws) {
      try {
        this.ws.close();
      } catch {}
      this.ws = null;
    }
  }
}

// ─── CLI Entrypoint ────────────────────────────────────────────────────────────
if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve("scripts/e2e/cdp.mjs")) {
  const args = process.argv.slice(2);
  let port = null;

  // Check for --port <p>
  const portIdx = args.indexOf("--port");
  if (portIdx !== -1) {
    port = args[portIdx + 1];
    args.splice(portIdx, 2);
  }

  const [cmd, ...cmdArgs] = args;
  if (!cmd) {
    console.log("Usage: node cdp.mjs <shot|eval|invoke|click|clicktext|key|type|wheel|drag|waitText> [args]");
    process.exit(0);
  }

  const client = await CDPClient.connect({ port });

  try {
    if (cmd === "shot") {
      const out = cmdArgs[0] || "screenshot.png";
      await client.shot(out);
      console.log(`Saved screenshot to ${out}`);
    } else if (cmd === "eval") {
      const res = await client.eval(cmdArgs[0]);
      console.log(JSON.stringify(res, null, 2));
    } else if (cmd === "invoke") {
      const [command, jsonArgs] = cmdArgs;
      const parsedArgs = jsonArgs ? JSON.parse(jsonArgs) : {};
      const res = await client.invoke(command, parsedArgs);
      console.log(JSON.stringify(res, null, 2));
    } else if (cmd === "click") {
      const [x, y] = cmdArgs.map(Number);
      await client.click(x, y);
      console.log(`Clicked at ${x}, ${y}`);
    } else if (cmd === "clicktext") {
      const res = await client.clickText(cmdArgs[0]);
      console.log(`Clicked text "${cmdArgs[0]}" (${res.tag})`);
    } else if (cmd === "key") {
      const [key, ...mods] = cmdArgs;
      await client.key(key, mods);
      console.log(`Dispatched key ${key} [${mods.join("+")}]`);
    } else if (cmd === "type") {
      await client.type(cmdArgs[0]);
      console.log(`Typed "${cmdArgs[0]}"`);
    } else if (cmd === "wheel") {
      const [x, y, dy] = cmdArgs.map(Number);
      await client.wheel(x, y, dy);
      console.log(`Wheeled deltaY=${dy} at ${x}, ${y}`);
    } else if (cmd === "drag") {
      const [x1, y1, x2, y2] = cmdArgs.map(Number);
      await client.drag(x1, y1, x2, y2);
      console.log(`Dragged from ${x1},${y1} to ${x2},${y2}`);
    } else if (cmd === "waitText") {
      const timeout = cmdArgs[1] ? Number(cmdArgs[1]) : 10000;
      await client.waitText(cmdArgs[0], timeout);
      console.log(`Found text "${cmdArgs[0]}"`);
    } else {
      console.error(`Unknown command: ${cmd}`);
      process.exit(1);
    }
  } finally {
    client.close();
  }
}
