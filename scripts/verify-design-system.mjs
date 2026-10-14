import fs from "node:fs";
import path from "node:path";

console.log("=== KIỂM TRA DESIGN SYSTEM & WCAG CONTRAST ===");

// 1. Relative luminance calculation (WCAG 2.1)
function getLuminance(hex) {
  const cleanHex = hex.replace("#", "");
  const r = parseInt(cleanHex.substring(0, 2), 16) / 255;
  const g = parseInt(cleanHex.substring(2, 4), 16) / 255;
  const b = parseInt(cleanHex.substring(4, 6), 16) / 255;

  const toLinear = (c) => (c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4));

  const lr = toLinear(r);
  const lg = toLinear(g);
  const lb = toLinear(b);

  return 0.2126 * lr + 0.7152 * lg + 0.0722 * lb;
}

function getContrastRatio(hex1, hex2) {
  const l1 = getLuminance(hex1);
  const l2 = getLuminance(hex2);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

// Token pairs from design-system.md
const pairs = [
  { name: "Light: fg-primary vs bg-panel", fg: "#182132", bg: "#ffffff", min: 4.5 },
  { name: "Light: fg-secondary vs bg-panel", fg: "#526078", bg: "#ffffff", min: 4.5 },
  { name: "Light: accent vs bg-panel", fg: "#334bd1", bg: "#ffffff", min: 4.5 },
  { name: "Dark: fg-primary vs bg-panel", fg: "#eff3fb", bg: "#141d30", min: 4.5 },
  { name: "Dark: fg-secondary vs bg-panel", fg: "#b2bfd3", bg: "#141d30", min: 4.5 },
  { name: "Dark: accent vs bg-panel", fg: "#9aaaff", bg: "#141d30", min: 4.5 },
];

let allPassed = true;
for (const p of pairs) {
  const ratio = getContrastRatio(p.fg, p.bg);
  const passed = ratio >= p.min;
  console.log(`- ${p.name}: ${ratio.toFixed(2)}:1 (Yêu cầu: ${p.min}:1) -> ${passed ? "PASS" : "FAIL"}`);
  if (!passed) allPassed = false;
}

if (!allPassed) {
  console.error("LỖI: Tương phản không đạt chuẩn WCAG AA!");
  process.exit(1);
}

// 2. Token Linter: Scan files in packages/design-system/src/components to verify no hardcoded hex
const componentsDir = path.join("packages", "design-system", "src", "components");
const files = fs.readdirSync(componentsDir).filter((f) => f.endsWith(".tsx"));

let linterPassed = true;
for (const f of files) {
  const content = fs.readFileSync(path.join(componentsDir, f), "utf-8");
  // Check for hardcoded hex colors (e.g. #abc or #123456)
  const hexMatches = content.match(/#[0-9a-fA-F]{3,8}\b/g);
  if (hexMatches) {
    console.error(`LỖI: Phát hiện mã màu hex cứng trong ${f}: ${hexMatches.join(", ")}`);
    linterPassed = false;
  }
}

if (linterPassed) {
  console.log("Token Linter: 100% components sử dụng CSS variables --cs-*, không có mã màu hex cứng!");
} else {
  process.exit(1);
}

console.log("KIỂM TRA DESIGN SYSTEM THÀNH CÔNG: PASS");
