/**
 * Tauri IPC Bridge
 * Wraps window.__TAURI_INTERNALS__ and window.__TAURI__ APIs.
 * When running in standard browser/dev server without Tauri desktop shell,
 * invoke() throws "Chưa kết nối Studio core" to guarantee no fake data.
 */

export function isTauri(): boolean {
  if (typeof window === "undefined") return false;
  return Boolean(
    (window as any).__TAURI_INTERNALS__ ||
    (window as any).__TAURI__
  );
}

/**
 * Invoke a Tauri core IPC command.
 * Throws "Chưa kết nối Studio core" if not running in Tauri.
 */
export async function invoke<T = unknown>(
  cmd: string,
  args?: Record<string, unknown>
): Promise<T> {
  if (!isTauri()) {
    throw new Error("Chưa kết nối Studio core");
  }

  const tauriInternals = (window as any).__TAURI_INTERNALS__;
  if (tauriInternals && typeof tauriInternals.invoke === "function") {
    return tauriInternals.invoke(cmd, args);
  }

  const tauriCore = (window as any).__TAURI__?.core;
  if (tauriCore && typeof tauriCore.invoke === "function") {
    return tauriCore.invoke(cmd, args);
  }

  const tauriTauri = (window as any).__TAURI__?.tauri;
  if (tauriTauri && typeof tauriTauri.invoke === "function") {
    return tauriTauri.invoke(cmd, args);
  }

  throw new Error("Chưa kết nối Studio core");
}

export type EventCallback<T> = (event: { event: string; payload: T }) => void;
export type UnlistenFn = () => void;

/**
 * Listen to a Tauri IPC event.
 * If not running in Tauri, returns a no-op unlisten function.
 */
export async function listen<T = unknown>(
  event: string,
  handler: EventCallback<T>
): Promise<UnlistenFn> {
  if (!isTauri()) {
    return () => {};
  }

  const eventApi = (window as any).__TAURI__?.event || (window as any).__TAURI_INTERNALS__;
  if (eventApi && typeof eventApi.listen === "function") {
    return eventApi.listen(event, handler);
  }

  return () => {};
}
