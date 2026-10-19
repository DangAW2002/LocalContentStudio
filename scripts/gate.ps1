param(
    [switch]$Smoke,
    [switch]$Clean
)

$ErrorActionPreference = "Stop"
$StartTime = Get-Date

Write-Host "=================================================================" -ForegroundColor Cyan
Write-Host "      LOCAL CONTENT STUDIO - KIỂM SOÁT CỔNG CHẤT LƯỢNG (GATE)    " -ForegroundColor Cyan
Write-Host "=================================================================" -ForegroundColor Cyan

$Results = [System.Collections.Generic.List[PSCustomObject]]::new()

function Run-GateStep {
    param(
        [string]$Name,
        [scriptblock]$Command
    )
    Write-Host "`n>>> Đang chạy: $Name..." -ForegroundColor Yellow
    $StepStart = Get-Date
    $Passed = $false
    $ErrorMsg = ""

    try {
        & $Command
        if ($LASTEXITCODE -eq 0 -or $LASTEXITCODE -eq $null) {
            $Passed = $true
        } else {
            $ErrorMsg = "Exit code: $LASTEXITCODE"
        }
    } catch {
        $ErrorMsg = $_.Exception.Message
    }

    $Duration = ((Get-Date) - $StepStart).TotalSeconds
    $DurationStr = "{0:N1}s" -f $Duration

    $Results.Add([PSCustomObject]@{
        "Cổng (Gate)" = $Name
        "Kết quả"     = if ($Passed) { "PASS" } else { "FAIL" }
        "Thời gian"   = $DurationStr
    })

    if (-not $Passed) {
        Write-Host "❌ BƯỚC THẤT BẠI: $Name ($ErrorMsg)" -ForegroundColor Red
        Print-Summary
        exit 1
    } else {
        Write-Host "✅ XONG: $Name ($DurationStr)" -ForegroundColor Green
    }
}

function Print-Summary {
    $TotalDuration = ((Get-Date) - $StartTime).TotalSeconds
    $TotalStr = "{0:N1}s" -f $TotalDuration

    Write-Host "`n=================================================================" -ForegroundColor Cyan
    Write-Host "                    BẢNG TỔNG KẾT CÁC CỔNG                       " -ForegroundColor Cyan
    Write-Host "=================================================================" -ForegroundColor Cyan
    $Results | Format-Table -AutoSize
    Write-Host "Tổng thời gian thực thi: $TotalStr" -ForegroundColor Cyan
}

# 1. Rust Formatting
Run-GateStep "1. cargo fmt check" {
    cargo fmt --all -- --check
}

# 2. Rust Clippy
Run-GateStep "2. cargo clippy workspace" {
    cargo clippy --workspace --all-targets -- -D warnings
}

# 3. Rust Unit/Integration Tests
Run-GateStep "3. cargo test workspace" {
    cargo test --workspace
}

# 4. Design System Build
Run-GateStep "4. pnpm @studio/design-system build" {
    pnpm --filter @studio/design-system build
}

# 5. Desktop UI Typecheck
Run-GateStep "5. pnpm @studio/desktop-ui typecheck" {
    pnpm --filter @studio/desktop-ui typecheck
}

# 6. Desktop UI Test
Run-GateStep "6. pnpm @studio/desktop-ui test" {
    pnpm --filter @studio/desktop-ui test
}

# 7. Desktop UI Build
Run-GateStep "7. pnpm @studio/desktop-ui build" {
    pnpm --filter @studio/desktop-ui build
}

# 8. Optional E2E Smoke Test
if ($Smoke) {
    Run-GateStep "8. E2E Smoke Test (custom-protocol)" {
        node scripts/e2e/smoke.mjs --build
    }
}

Print-Summary
Write-Host "`n🎉 TẤT CẢ CÁC CỔNG ĐỀU ĐẠT CHUẨN (GREEN)!" -ForegroundColor Green
exit 0
