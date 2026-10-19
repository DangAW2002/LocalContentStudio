/**
 * Tauri IPC & Web Transport Bridge
 * Wraps window.__TAURI_INTERNALS__ and window.__TAURI__ APIs when running in Tauri desktop shell.
 * When running in standard browser/dev server, transparently uses:
 * - POST /api/invoke/<cmd> with x-studio-csrf header and session cookies.
 * - GET /api/events Server-Sent Events (SSE) for real-time live events.
 */

export function isTauri(): boolean {
  if (typeof window === "undefined") return false;
  return Boolean(
    (window as any).__TAURI_INTERNALS__ ||
    (window as any).__TAURI__
  );
}

export type EventCallback<T> = (event: { event: string; payload: T }) => void;
export type UnlistenFn = () => void;

type SudoPromptHandler = (cmd: string) => Promise<boolean>;
let sudoPromptHandler: SudoPromptHandler | null = null;

/**
 * Register a handler to prompt user for password when an invoke fails with SUDO_REQUIRED.
 */
export function registerSudoHandler(handler: SudoPromptHandler | null) {
  sudoPromptHandler = handler;
}

/**
 * Invoke a Studio command.
 * In desktop Tauri: calls native IPC invoke.
 * In web browser: calls POST /api/invoke/<command>.
 */
export async function invoke<T = unknown>(
  cmd: string,
  args?: Record<string, unknown>
): Promise<T> {
  if (isTauri()) {
    try {
      const tauriInternals = (window as any).__TAURI_INTERNALS__;
      if (tauriInternals && typeof tauriInternals.invoke === "function") {
        return await tauriInternals.invoke(cmd, args);
      }

      const tauriCore = (window as any).__TAURI__?.core;
      if (tauriCore && typeof tauriCore.invoke === "function") {
        return await tauriCore.invoke(cmd, args);
      }

      const tauriTauri = (window as any).__TAURI__?.tauri;
      if (tauriTauri && typeof tauriTauri.invoke === "function") {
        return await tauriTauri.invoke(cmd, args);
      }
    } catch (err: any) {
      if (typeof err === "string") {
        throw new Error(err);
      }
      throw err;
    }
  }

  // Web mode fallback
  const res = await fetch(`/api/invoke/${cmd}`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-studio-csrf": "1",
    },
    body: JSON.stringify(args ?? {}),
  });

  if (res.status === 401) {
    if (typeof window !== "undefined") {
      window.dispatchEvent(new CustomEvent("studio-auth-required"));
    }
    throw new Error("Yêu cầu đăng nhập mật khẩu chủ sở hữu.");
  }

  if (res.status === 403) {
    const errData = await res.json().catch(() => ({}));
    if (errData.code === "SUDO_REQUIRED" && sudoPromptHandler) {
      const ok = await sudoPromptHandler(cmd);
      if (ok) {
        // Retry with active sudo session
        return invoke<T>(cmd, args);
      }
    }
    throw new Error(errData.error || "Thao tác bị từ chối.");
  }

  if (!res.ok) {
    const errData = await res.json().catch(() => ({}));
    throw new Error(errData.error || `Lỗi gọi lệnh ${cmd} (${res.status})`);
  }

  return res.json();
}

// ──────────────────────────────── SSE Listener Singleton ────────────────────────────────

let eventSource: EventSource | null = null;
const eventListeners: Map<string, Set<EventCallback<any>>> = new Map();

function ensureEventSource() {
  if (eventSource || typeof window === "undefined" || isTauri()) return;

  eventSource = new EventSource("/api/events");

  eventSource.onopen = () => {
    // Connected to SSE stream
  };

  eventSource.onerror = () => {
    // Disconnection / reconnect will be handled by EventSource automatically
  };
}

/**
 * Listen to a Studio IPC event (e.g. `job-event`, `turn-event`, `fs-event`).
 * In desktop: uses Tauri event listener.
 * In web: uses SSE (/api/events).
 */
export async function listen<T = unknown>(
  event: string,
  handler: EventCallback<T>
): Promise<UnlistenFn> {
  if (isTauri()) {
    const eventApi = (window as any).__TAURI__?.event;
    if (eventApi && typeof eventApi.listen === "function") {
      return eventApi.listen(event, handler);
    }
    // Without `withGlobalTauri` only the internals exist: register through the event plugin
    // the same way @tauri-apps/api does.
    const internals = (window as any).__TAURI_INTERNALS__;
    if (internals && typeof internals.transformCallback === "function") {
      const handlerId = internals.transformCallback(handler);
      const eventId = await internals.invoke("plugin:event|listen", {
        event,
        target: { kind: "Any" },
        handler: handlerId,
      });
      return () => {
        internals.invoke("plugin:event|unlisten", { event, eventId }).catch(() => {});
      };
    }
    console.error(`[studio] Không đăng ký được sự kiện "${event}": thiếu Tauri event API.`);
    return () => {};
  }

  // Web mode
  ensureEventSource();

  if (!eventListeners.has(event)) {
    eventListeners.set(event, new Set());
    if (eventSource) {
      eventSource.addEventListener(event, (msgEvent: MessageEvent) => {
        try {
          const payload = JSON.parse(msgEvent.data);
          const listeners = eventListeners.get(event);
          if (listeners) {
            listeners.forEach((fn) => fn({ event, payload }));
          }
        } catch {
          // Ignore malformed event payload
        }
      });
    }
  }

  eventListeners.get(event)!.add(handler);

  return () => {
    const listeners = eventListeners.get(event);
    if (listeners) {
      listeners.delete(handler);
    }
  };
}

// ──────────────────────────────── Web Mode Auth & Upload Helpers ────────────────────────────────

export interface WebAuthStatus {
  authenticated: boolean;
  sudo_until: string | null;
}

/**
 * Check whether the current web session is authenticated.
 */
export async function checkAuthStatus(): Promise<WebAuthStatus> {
  try {
    const res = await fetch("/api/auth/status");
    if (!res.ok) return { authenticated: false, sudo_until: null };
    return await res.json();
  } catch {
    return { authenticated: false, sudo_until: null };
  }
}

/**
 * Log into Personal Web Mode with the owner password.
 */
export async function login(password: string): Promise<{ success: boolean; message?: string }> {
  try {
    const res = await fetch("/api/auth/login", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "x-studio-csrf": "1",
      },
      body: JSON.stringify({ password }),
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) {
      return { success: false, message: data.error || "Mật khẩu không chính xác." };
    }
    return { success: true };
  } catch (err: any) {
    return { success: false, message: err?.message || "Lỗi mạng khi đăng nhập." };
  }
}

/**
 * Log out of Personal Web Mode.
 */
export async function logout(): Promise<void> {
  try {
    await fetch("/api/auth/logout", {
      method: "POST",
      headers: { "x-studio-csrf": "1" },
    });
  } catch {
    // Ignore error on logout
  }
}

/**
 * Elevate web session to sudo mode by entering owner password.
 */
export async function verifySudo(
  password: string
): Promise<{ success: boolean; sudo_until?: string; message?: string }> {
  try {
    const res = await fetch("/api/auth/sudo", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "x-studio-csrf": "1",
      },
      body: JSON.stringify({ password }),
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) {
      return { success: false, message: data.error || "Mật khẩu không chính xác." };
    }
    return { success: true, sudo_until: data.sudo_until };
  } catch (err: any) {
    return { success: false, message: err?.message || "Lỗi mạng khi xác thực sudo." };
  }
}

/**
 * Upload a file fallback for browser mode via POST /api/upload.
 * Returns the absolute path on the host machine where the file was saved temporarily.
 */
export async function uploadFile(file: File): Promise<string> {
  const formData = new FormData();
  formData.append("file", file);

  const res = await fetch("/api/upload", {
    method: "POST",
    headers: {
      "x-studio-csrf": "1",
    },
    body: formData,
  });

  if (!res.ok) {
    const data = await res.json().catch(() => ({}));
    throw new Error(data.error || "Không thể tải tệp lên.");
  }

  const data = await res.json();
  return data.path;
}
