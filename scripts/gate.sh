#!/usr/bin/env bash
set -e

SMOKE=0
for arg in "$@"; do
    case "$arg" in
        --smoke|-s)
            SMOKE=1
            ;;
    esac
done

START_TIME=$(date +%s)
echo "================================================================="
echo "      LOCAL CONTENT STUDIO - KIỂM SOÁT CỔNG CHẤT LƯỢNG (GATE)    "
echo "================================================================="

declare -a STEPS=()
declare -a RESULTS=()
declare -a DURATIONS=()

run_step() {
    local name="$1"
    shift
    echo -e "\n>>> Đang chạy: $name..."
    local step_start=$(date +%s)
    
    if "$@"; then
        local step_end=$(date +%s)
        local dur=$((step_end - step_start))
        STEPS+=("$name")
        RESULTS+=("PASS")
        DURATIONS+=("${dur}s")
        echo "✅ XONG: $name (${dur}s)"
    else
        local step_end=$(date +%s)
        local dur=$((step_end - step_start))
        STEPS+=("$name")
        RESULTS+=("FAIL")
        DURATIONS+=("${dur}s")
        echo "❌ BƯỚC THẤT BẠI: $name"
        print_summary
        exit 1
    fi
}

print_summary() {
    local end_time=$(date +%s)
    local total_dur=$((end_time - START_TIME))
    echo ""
    echo "================================================================="
    echo "                    BẢNG TỔNG KẾT CÁC CỔNG                       "
    echo "================================================================="
    printf "%-40s | %-8s | %-10s\n" "Cổng (Gate)" "Kết quả" "Thời gian"
    echo "-----------------------------------------+----------+-----------"
    for i in "${!STEPS[@]}"; do
        printf "%-40s | %-8s | %-10s\n" "${STEPS[$i]}" "${RESULTS[$i]}" "${DURATIONS[$i]}"
    done
    echo "-----------------------------------------+----------+-----------"
    echo "Tổng thời gian thực thi: ${total_dur}s"
}

run_step "1. cargo fmt check" cargo fmt --all -- --check
run_step "2. cargo clippy workspace" cargo clippy --workspace --all-targets -- -D warnings
run_step "3. cargo test workspace" cargo test --workspace
run_step "4. pnpm @studio/design-system build" pnpm --filter @studio/design-system build
run_step "5. pnpm @studio/desktop-ui typecheck" pnpm --filter @studio/desktop-ui typecheck
run_step "6. pnpm @studio/desktop-ui test" pnpm --filter @studio/desktop-ui test
run_step "7. pnpm @studio/desktop-ui build" pnpm --filter @studio/desktop-ui build

if [ "$SMOKE" -eq 1 ]; then
    run_step "8. E2E Smoke Test (custom-protocol)" node scripts/e2e/smoke.mjs --build
fi

print_summary
echo -e "\n🎉 TẤT CẢ CÁC CỔNG ĐỀU ĐẠT CHUẨN (GREEN)!"
exit 0
