export interface BrandColors {
  primary: string;
  accent: string;
  bg: string;
  text: string;
  extra: string[];
}

export interface BrandFonts {
  heading: string;
  body: string;
  files: string[];
}

export interface BrandLogos {
  full: string;
  mark: string;
  min_size_px: number;
  clear_space_ratio: number;
  placement_rules: string[];
}

export interface VoiceWords {
  prefer: string[];
  avoid: string[];
}

export interface BrandMotion {
  intro?: string;
  outro?: string;
  lower_third?: string;
}

export interface BrandSpec {
  name: string;
  slogan: string;
  colors: BrandColors;
  fonts: BrandFonts;
  logos: BrandLogos;
  voice: string;
  voice_words: VoiceWords;
  motion?: BrandMotion;
  rules: string[];
  version: string;
}

export const DEFAULT_BRAND_SPEC: BrandSpec = {
  name: "Local Content Studio",
  slogan: "Sáng tạo nội dung video và thiết kế chất lượng cao cùng AI",
  colors: {
    primary: "#1E88E5",
    accent: "#FFB300",
    bg: "#0B0F14",
    text: "#FFFFFF",
    extra: ["#43A047", "#E53935"],
  },
  fonts: {
    heading: "Be Vietnam Pro",
    body: "Inter",
    files: [],
  },
  logos: {
    full: "logo.svg",
    mark: "mark.svg",
    min_size_px: 48,
    clear_space_ratio: 0.25,
    placement_rules: ["Logo hiển thị rõ ràng, không bị che khuất bởi các thành phần khác"],
  },
  voice: "voice.md",
  voice_words: {
    prefer: ["đột phá", "chất lượng", "thực chiến", "hiệu quả"],
    avoid: ["cam kết 100%", "rẻ nhất", "thần thánh"],
  },
  rules: [
    "Sử dụng màu primary cho các điểm nhấn thương hiệu",
    "Đảm bảo độ tương phản WCAG AA giữa chữ và màu nền",
  ],
  version: "brand@1",
};

/**
 * Robust line-by-line YAML parser tailored for BrandSpec (brand@1).
 */
export function parseBrandYaml(yamlText: string): BrandSpec {
  const result: BrandSpec = JSON.parse(JSON.stringify(DEFAULT_BRAND_SPEC));
  const lines = yamlText.split(/\r?\n/);

  let currentSection: string | null = null;
  let currentSubSection: string | null = null;

  for (let rawLine of lines) {
    // Strip comments (ignore # inside quotes)
    let lineWithoutComment = rawLine;
    let inQuote: string | null = null;
    for (let i = 0; i < rawLine.length; i++) {
      const ch = rawLine[i];
      if (inQuote) {
        if (ch === inQuote) inQuote = null;
      } else {
        if (ch === '"' || ch === "'") {
          inQuote = ch;
        } else if (ch === "#") {
          lineWithoutComment = rawLine.slice(0, i);
          break;
        }
      }
    }
    const trimmed = lineWithoutComment.trim();
    if (!trimmed) continue;

    const indent = lineWithoutComment.search(/\S/);

    // List item (e.g. - "#43A047" or - "rule text")
    if (trimmed.startsWith("-")) {
      const val = cleanYamlVal(trimmed.slice(1).trim());
      if (val) {
        if (currentSection === "colors" && currentSubSection === "extra") {
          result.colors.extra.push(val);
        } else if (currentSection === "logos" && currentSubSection === "placement_rules") {
          result.logos.placement_rules.push(val);
        } else if (currentSection === "voice_words") {
          if (currentSubSection === "prefer") {
            result.voice_words.prefer.push(val);
          } else if (currentSubSection === "avoid") {
            result.voice_words.avoid.push(val);
          }
        } else if (currentSection === "rules") {
          result.rules.push(val);
        } else if (currentSection === "fonts" && currentSubSection === "files") {
          result.fonts.files.push(val);
        }
      }
      continue;
    }

    const colonIdx = trimmed.indexOf(":");
    if (colonIdx < 0) continue;

    const key = trimmed.slice(0, colonIdx).trim();
    const valRaw = trimmed.slice(colonIdx + 1).trim();
    const val = cleanYamlVal(valRaw);

    // Top-level keys (indent 0)
    if (indent === 0) {
      currentSection = key;
      currentSubSection = null;

      if (key === "name" && val) result.name = val;
      else if (key === "slogan") result.slogan = val;
      else if (key === "voice" && val) result.voice = val;
      else if (key === "version" && val) result.version = val;
      else if (key === "rules") {
        result.rules = [];
      } else if (key === "colors") {
        result.colors.extra = [];
      } else if (key === "voice_words") {
        result.voice_words.prefer = [];
        result.voice_words.avoid = [];
      }
      continue;
    }

    // Indented sub-properties
    if (currentSection === "colors") {
      if (key === "primary" && val) result.colors.primary = val;
      else if (key === "accent" && val) result.colors.accent = val;
      else if (key === "bg" && val) result.colors.bg = val;
      else if (key === "text" && val) result.colors.text = val;
      else if (key === "extra") {
        currentSubSection = "extra";
        result.colors.extra = parseInlineArray(valRaw);
      }
    } else if (currentSection === "fonts") {
      if (key === "heading" && val) result.fonts.heading = val;
      else if (key === "body" && val) result.fonts.body = val;
      else if (key === "files") {
        currentSubSection = "files";
        result.fonts.files = parseInlineArray(valRaw);
      }
    } else if (currentSection === "logos") {
      if (key === "full" && val) result.logos.full = val;
      else if (key === "mark" && val) result.logos.mark = val;
      else if (key === "min_size_px" && val) result.logos.min_size_px = parseInt(val, 10) || 32;
      else if (key === "clear_space_ratio" && val) result.logos.clear_space_ratio = parseFloat(val) || 0.2;
      else if (key === "placement_rules") {
        currentSubSection = "placement_rules";
        result.logos.placement_rules = parseInlineArray(valRaw);
      }
    } else if (currentSection === "voice_words") {
      if (key === "prefer") {
        currentSubSection = "prefer";
        result.voice_words.prefer = parseInlineArray(valRaw);
      } else if (key === "avoid") {
        currentSubSection = "avoid";
        result.voice_words.avoid = parseInlineArray(valRaw);
      }
    } else if (currentSection === "motion") {
      if (!result.motion) result.motion = {};
      if (key === "intro") result.motion.intro = val;
      else if (key === "outro") result.motion.outro = val;
      else if (key === "lower_third") result.motion.lower_third = val;
    }
  }

  return result;
}

function cleanYamlVal(val: string): string {
  let v = val.trim();
  if ((v.startsWith('"') && v.endsWith('"')) || (v.startsWith("'") && v.endsWith("'"))) {
    v = v.slice(1, -1);
  }
  return v;
}

function parseInlineArray(valRaw: string): string[] {
  const trimmed = valRaw.trim();
  if (trimmed.startsWith("[") && trimmed.endsWith("]")) {
    return trimmed
      .slice(1, -1)
      .split(",")
      .map((s) => cleanYamlVal(s))
      .filter(Boolean);
  }
  return [];
}

/**
 * Serialize a BrandSpec object into valid brand@1 YAML format.
 */
export function stringifyBrandYaml(spec: BrandSpec): string {
  const lines: string[] = [
    `name: "${escapeQuotes(spec.name)}"`,
    `slogan: "${escapeQuotes(spec.slogan || "")}"`,
    `colors:`,
    `  primary: "${spec.colors.primary}"`,
    `  accent: "${spec.colors.accent}"`,
    `  bg: "${spec.colors.bg}"`,
    `  text: "${spec.colors.text}"`,
  ];

  if (spec.colors.extra && spec.colors.extra.length > 0) {
    lines.push(`  extra:`);
    for (const color of spec.colors.extra) {
      lines.push(`    - "${color}"`);
    }
  } else {
    lines.push(`  extra: []`);
  }

  lines.push(`fonts:`);
  lines.push(`  heading: "${escapeQuotes(spec.fonts.heading)}"`);
  lines.push(`  body: "${escapeQuotes(spec.fonts.body)}"`);
  if (spec.fonts.files && spec.fonts.files.length > 0) {
    lines.push(`  files:`);
    for (const f of spec.fonts.files) {
      lines.push(`    - "${escapeQuotes(f)}"`);
    }
  } else {
    lines.push(`  files: []`);
  }

  lines.push(`logos:`);
  lines.push(`  full: "${escapeQuotes(spec.logos.full)}"`);
  lines.push(`  mark: "${escapeQuotes(spec.logos.mark)}"`);
  lines.push(`  min_size_px: ${spec.logos.min_size_px || 32}`);
  lines.push(`  clear_space_ratio: ${spec.logos.clear_space_ratio || 0.25}`);
  if (spec.logos.placement_rules && spec.logos.placement_rules.length > 0) {
    lines.push(`  placement_rules:`);
    for (const r of spec.logos.placement_rules) {
      lines.push(`    - "${escapeQuotes(r)}"`);
    }
  } else {
    lines.push(`  placement_rules: []`);
  }

  lines.push(`voice: "${escapeQuotes(spec.voice || "voice.md")}"`);

  lines.push(`voice_words:`);
  if (spec.voice_words.prefer && spec.voice_words.prefer.length > 0) {
    lines.push(`  prefer:`);
    for (const w of spec.voice_words.prefer) {
      lines.push(`    - "${escapeQuotes(w)}"`);
    }
  } else {
    lines.push(`  prefer: []`);
  }

  if (spec.voice_words.avoid && spec.voice_words.avoid.length > 0) {
    lines.push(`  avoid:`);
    for (const w of spec.voice_words.avoid) {
      lines.push(`    - "${escapeQuotes(w)}"`);
    }
  } else {
    lines.push(`  avoid: []`);
  }

  if (spec.motion && (spec.motion.intro || spec.motion.outro || spec.motion.lower_third)) {
    lines.push(`motion:`);
    if (spec.motion.intro) lines.push(`  intro: "${escapeQuotes(spec.motion.intro)}"`);
    if (spec.motion.outro) lines.push(`  outro: "${escapeQuotes(spec.motion.outro)}"`);
    if (spec.motion.lower_third) lines.push(`  lower_third: "${escapeQuotes(spec.motion.lower_third)}"`);
  }

  if (spec.rules && spec.rules.length > 0) {
    lines.push(`rules:`);
    for (const r of spec.rules) {
      lines.push(`    - "${escapeQuotes(r)}"`);
    }
  } else {
    lines.push(`rules: []`);
  }

  lines.push(`version: "brand@1"`);
  lines.push("");

  return lines.join("\n");
}

function escapeQuotes(str: string): string {
  return str.replace(/"/g, '\\"');
}

/** Validate hex color string e.g. #1E88E5, #FFF, #112233AA */
export function isValidHexColor(hex: string): boolean {
  return /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.test(hex.trim());
}
