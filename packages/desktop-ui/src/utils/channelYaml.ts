import type { ChannelFormat } from "../api/types";

export interface SafeZoneSpec {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

export interface FormatSpec extends ChannelFormat {}

export interface TextLimitsSpec {
  title_max?: number;
  description_max?: number;
  caption_max?: number;
  hashtags_max?: number;
}

export interface FileConstraintsSpec {
  containers: string[];
  max_size_mb?: number;
}

export interface ChannelSpec {
  id: string;
  platform: string;
  title: string;
  formats: FormatSpec[];
  text: TextLimitsSpec;
  captions_required: boolean;
  loudness_lufs?: number;
  file: FileConstraintsSpec;
  notes: string;
}

export const DEFAULT_CHANNEL_SPEC: ChannelSpec = {
  id: "custom-channel",
  platform: "custom",
  title: "Kênh mới",
  formats: [
    {
      name: "standard",
      width: 1080,
      height: 1920,
      aspect: "9:16",
      fps: 30,
      max_duration_s: 60,
      min_duration_s: 3,
      safe_zone: { top: 0.05, right: 0.05, bottom: 0.15, left: 0.05 },
    },
  ],
  text: {
    title_max: 100,
    description_max: 2000,
    caption_max: 150,
    hashtags_max: 10,
  },
  captions_required: false,
  loudness_lufs: -14,
  file: {
    containers: ["mp4", "mov"],
    max_size_mb: 500,
  },
  notes: "Cấu hình kênh tiêu chuẩn",
};

/**
 * Computes friendly aspect ratio string (e.g. "9:16", "16:9", "1:1", "4:5") from pixel dimensions.
 */
export function computeAspectRatio(width: number, height: number): string {
  if (!width || !height || width <= 0 || height <= 0) return "1:1";

  const gcd = (a: number, b: number): number => (b === 0 ? a : gcd(b, a % b));
  const d = gcd(Math.round(width), Math.round(height));
  const wRatio = Math.round(width) / d;
  const hRatio = Math.round(height) / d;

  if (
    (wRatio === 9 && hRatio === 16) ||
    (wRatio === 16 && hRatio === 9) ||
    (wRatio === 1 && hRatio === 1) ||
    (wRatio === 4 && hRatio === 5)
  ) {
    return `${wRatio}:${hRatio}`;
  }

  const ratio = width / height;
  if (Math.abs(ratio - 9 / 16) < 0.02) return "9:16";
  if (Math.abs(ratio - 16 / 9) < 0.02) return "16:9";
  if (Math.abs(ratio - 1) < 0.02) return "1:1";
  if (Math.abs(ratio - 4 / 5) < 0.02) return "4:5";
  if (Math.abs(ratio - 1200 / 628) < 0.02) return "1.91:1";

  return `${wRatio}:${hRatio}`;
}

function cleanYamlVal(val: string): string {
  let v = val.trim();
  if (
    (v.startsWith('"') && v.endsWith('"')) ||
    (v.startsWith("'") && v.endsWith("'"))
  ) {
    v = v.slice(1, -1);
  }
  return v;
}

/**
 * Parse channel YAML into ChannelSpec.
 */
export function parseChannelYaml(yamlText: string, defaultId: string = "channel"): ChannelSpec {
  const spec: ChannelSpec = {
    ...DEFAULT_CHANNEL_SPEC,
    id: defaultId,
    formats: [],
    text: { ...DEFAULT_CHANNEL_SPEC.text },
    file: { containers: ["mp4", "mov"], max_size_mb: 500 },
  };

  const lines = yamlText.split(/\r?\n/);
  let currentSection: "root" | "formats" | "text" | "file" = "root";
  let currentFormat: FormatSpec | null = null;
  let inSafeZone = false;

  for (const rawLine of lines) {
    // Strip comment
    let line = rawLine;
    const commentIdx = line.indexOf("#");
    if (commentIdx >= 0) {
      line = line.substring(0, commentIdx);
    }
    const trimmed = line.trim();
    if (!trimmed) continue;

    const indent = line.search(/\S/);

    if (trimmed.startsWith("formats:")) {
      currentSection = "formats";
      continue;
    } else if (trimmed.startsWith("text:") || trimmed.startsWith("limits:")) {
      currentSection = "text";
      continue;
    } else if (trimmed.startsWith("file:") || trimmed.startsWith("files:")) {
      currentSection = "file";
      continue;
    } else if (indent === 0 && trimmed.includes(":")) {
      currentSection = "root";
    }

    if (currentSection === "root") {
      const colonIdx = trimmed.indexOf(":");
      if (colonIdx > 0) {
        const key = trimmed.slice(0, colonIdx).trim();
        const val = cleanYamlVal(trimmed.slice(colonIdx + 1));
        if (key === "id") spec.id = val;
        else if (key === "platform") spec.platform = val;
        else if (key === "title" || key === "name") spec.title = val;
        else if (key === "notes" || key === "description") spec.notes = val;
        else if (key === "captions_required") spec.captions_required = val === "true";
        else if (key === "loudness_lufs") spec.loudness_lufs = parseFloat(val) || undefined;
      }
    } else if (currentSection === "formats") {
      if (
        trimmed.startsWith("- name:") ||
        trimmed.startsWith("- id:") ||
        (trimmed.startsWith("-") && (trimmed.includes("name:") || trimmed.includes("id:")))
      ) {
        const match = trimmed.match(/(?:name|id):\s*([^\s#]+|"[^"]*"|'[^']*')/);
        const nameVal = match ? cleanYamlVal(match[1]) : "format";
        currentFormat = {
          name: nameVal || "format",
          width: 1080,
          height: 1920,
          aspect: "9:16",
          safe_zone: { top: 0.05, right: 0.05, bottom: 0.15, left: 0.05 },
        };
        spec.formats.push(currentFormat);
        inSafeZone = false;
        continue;
      }

      if (!currentFormat) continue;

      if (trimmed.startsWith("safe_zone:")) {
        inSafeZone = true;
        continue;
      }

      const colonIdx = trimmed.indexOf(":");
      if (colonIdx > 0) {
        const key = trimmed.slice(0, colonIdx).trim();
        const val = cleanYamlVal(trimmed.slice(colonIdx + 1));
        if (inSafeZone && indent >= 6) {
          const num = parseFloat(val) || 0;
          if (key === "top") currentFormat.safe_zone.top = num;
          else if (key === "right") currentFormat.safe_zone.right = num;
          else if (key === "bottom") currentFormat.safe_zone.bottom = num;
          else if (key === "left") currentFormat.safe_zone.left = num;
        } else {
          inSafeZone = false;
          if (key === "width") currentFormat.width = parseInt(val, 10) || 1080;
          else if (key === "height") currentFormat.height = parseInt(val, 10) || 1920;
          else if (key === "aspect" || key === "ratio") currentFormat.aspect = val;
          else if (key === "fps") currentFormat.fps = parseFloat(val) || undefined;
          else if (key === "max_duration_s" || key === "duration_max_s") currentFormat.max_duration_s = parseFloat(val) || undefined;
          else if (key === "min_duration_s" || key === "duration_min_s") currentFormat.min_duration_s = parseFloat(val) || undefined;
        }
      }
    } else if (currentSection === "text") {
      const colonIdx = trimmed.indexOf(":");
      if (colonIdx > 0) {
        const key = trimmed.slice(0, colonIdx).trim();
        const valStr = cleanYamlVal(trimmed.slice(colonIdx + 1));
        const val = parseInt(valStr, 10);
        if (key === "captions_required") {
          spec.captions_required = valStr === "true";
        } else if (!isNaN(val)) {
          if (key === "title_max" || key === "title_max_chars") spec.text.title_max = val;
          else if (key === "description_max" || key === "desc_max_chars") spec.text.description_max = val;
          else if (key === "caption_max" || key === "caption_max_chars") spec.text.caption_max = val;
          else if (key === "hashtags_max") spec.text.hashtags_max = val;
        }
      }
    } else if (currentSection === "file") {
      const colonIdx = trimmed.indexOf(":");
      if (colonIdx > 0) {
        const key = trimmed.slice(0, colonIdx).trim();
        const val = cleanYamlVal(trimmed.slice(colonIdx + 1));
        if (key === "containers") {
          // parse list e.g. ["mp4", "mov"] or [mp4, mov]
          const cleaned = val.replace(/[\[\]"]/g, "");
          spec.file.containers = cleaned
            .split(",")
            .map((c) => c.trim())
            .filter(Boolean);
        } else if (key === "max_size_mb") {
          spec.file.max_size_mb = parseInt(val, 10) || undefined;
        }
      }
    }
  }

  if (spec.formats.length === 0) {
    spec.formats = [...DEFAULT_CHANNEL_SPEC.formats];
  }

  return spec;
}

/**
 * Serialize ChannelSpec into clean, readable YAML string matching channel@1.
 */
export function stringifyChannelYaml(spec: ChannelSpec): string {
  const lines: string[] = [];

  lines.push(`id: "${spec.id}"`);
  lines.push(`platform: "${spec.platform || "custom"}"`);
  lines.push(`title: "${spec.title}"`);

  lines.push("formats:");
  for (const f of spec.formats) {
    lines.push(`  - name: "${f.name}"`);
    lines.push(`    width: ${f.width}`);
    lines.push(`    height: ${f.height}`);
    lines.push(`    aspect: "${f.aspect || computeAspectRatio(f.width, f.height)}"`);
    if (f.fps != null) lines.push(`    fps: ${f.fps}`);
    if (f.max_duration_s != null) lines.push(`    max_duration_s: ${f.max_duration_s}`);
    if (f.min_duration_s != null) lines.push(`    min_duration_s: ${f.min_duration_s}`);
    lines.push("    safe_zone:");
    lines.push(`      top: ${f.safe_zone.top}`);
    lines.push(`      right: ${f.safe_zone.right}`);
    lines.push(`      bottom: ${f.safe_zone.bottom}`);
    lines.push(`      left: ${f.safe_zone.left}`);
  }

  lines.push("text:");
  if (spec.text.title_max != null) lines.push(`  title_max: ${spec.text.title_max}`);
  if (spec.text.description_max != null) lines.push(`  description_max: ${spec.text.description_max}`);
  if (spec.text.caption_max != null) lines.push(`  caption_max: ${spec.text.caption_max}`);
  if (spec.text.hashtags_max != null) lines.push(`  hashtags_max: ${spec.text.hashtags_max}`);

  lines.push(`captions_required: ${spec.captions_required ? "true" : "false"}`);
  if (spec.loudness_lufs != null) {
    lines.push(`loudness_lufs: ${spec.loudness_lufs}`);
  }

  lines.push("file:");
  const containers = spec.file.containers && spec.file.containers.length > 0 ? spec.file.containers : ["mp4"];
  lines.push(`  containers: [${containers.map((c) => `"${c}"`).join(", ")}]`);
  if (spec.file.max_size_mb != null) {
    lines.push(`  max_size_mb: ${spec.file.max_size_mb}`);
  }

  if (spec.notes) {
    lines.push(`notes: "${spec.notes.replace(/"/g, '\\"')}"`);
  }

  return lines.join("\n") + "\n";
}
