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

echo "==> [4/4] Biên dịch studio-tauri (release mode, CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS})..."
cargo build --release -p studio-tauri

mkdir -p "$HOME/.local/bin"
# Chép ra file tạm rồi đổi tên: ghi đè trực tiếp lỗi "Text file busy" khi service đang chạy.
cp "$REPO_ROOT/target/release/studio-tauri" "$HOME/.local/bin/studio-tauri.new"
chmod +x "$HOME/.local/bin/studio-tauri.new"
mv -f "$HOME/.local/bin/studio-tauri.new" "$HOME/.local/bin/studio-tauri"

echo "==> Thành công! Đã cài đặt binary tại: $HOME/.local/bin/studio-tauri"
echo "==> Commit: $(git -C "$REPO_ROOT" log --oneline -1)"
if systemctl --user is-active --quiet studio-web 2>/dev/null; then
    echo "==> Service studio-web đang chạy bản cũ; nạp bản mới: systemctl --user restart studio-web"
fi
