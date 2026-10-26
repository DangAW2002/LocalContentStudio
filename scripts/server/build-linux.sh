#!/usr/bin/env bash
set -euo pipefail

# Scripts build Studio trên máy chủ Linux (dang-server).
# Yêu cầu trước: Node 24+, pnpm 11+, Rust 1.99+ (rustup).

# Nạp môi trường nếu chạy trong shell không login
if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
fi
export PATH="$HOME/.local/bin:$PATH"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# Giới hạn số luồng build (dang-server 4 nhân, tránh OOM / nghẽn CPU)
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"

echo "==> [1/4] Cài đặt dependencies Node (pnpm)..."
pnpm install --frozen-lockfile

echo "==> [2/4] Build Design System..."
pnpm --filter @studio/design-system build

echo "==> [3/4] Build Desktop UI (nhúng vào binary)..."
pnpm --filter @studio/desktop-ui build

echo "==> [4/4] Biên dịch studio-tauri và studio-mcp (release mode, CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS})..."
cargo build --release -p studio-tauri -p studio-mcp

mkdir -p "$HOME/.local/bin"
# Chép ra file tạm rồi đổi tên: ghi đè trực tiếp lỗi "Text file busy" khi service đang chạy.
cp "$REPO_ROOT/target/release/studio-tauri" "$HOME/.local/bin/studio-tauri.new"
chmod +x "$HOME/.local/bin/studio-tauri.new"
mv -f "$HOME/.local/bin/studio-tauri.new" "$HOME/.local/bin/studio-tauri"

cp "$REPO_ROOT/target/release/studio-mcp" "$HOME/.local/bin/studio-mcp.new"
chmod +x "$HOME/.local/bin/studio-mcp.new"
mv -f "$HOME/.local/bin/studio-mcp.new" "$HOME/.local/bin/studio-mcp"

echo "==> Thành công! Đã cài đặt binary tại: $HOME/.local/bin/{studio-tauri, studio-mcp}"

# Tự động đăng ký Studio MCP cho Antigravity (agy) nếu agy có mặt trên máy
AGY_BIN=""
if command -v agy &>/dev/null; then
    AGY_BIN="$(command -v agy)"
elif [ -f "$HOME/.local/bin/agy" ]; then
    AGY_BIN="$HOME/.local/bin/agy"
fi

if [ -n "$AGY_BIN" ]; then
    echo "==> Đăng ký Studio MCP cho agy ($AGY_BIN)..."
    "$AGY_BIN" mcp add studio -- "$HOME/.local/bin/studio-mcp" --shim 2>/dev/null || true
fi

echo "==> Commit: $(git -C "$REPO_ROOT" log --oneline -1)"
if systemctl --user is-active --quiet studio-web 2>/dev/null; then
    echo "==> Service studio-web đang chạy bản cũ; nạp bản mới: systemctl --user restart studio-web"
fi
