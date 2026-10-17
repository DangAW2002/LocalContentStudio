/**
 * Artboard dimension detection utilities matching studio-artifacts Rust renderer.
 */

export interface ArtboardSize {
  w: number;
  h: number;
}

export const DEFAULT_ARTBOARD_SIZE: ArtboardSize = { w: 1080, h: 1350 };

/**
 * Detect declared artboard size from HTML source (in px).
 * Matches order in Rust `studio_artifacts::detect_artboard_size`:
 * 1. `<meta name="viewport" content="...width=W...height=H...">`
 * 2. `data-artboard="WxH"`
 * 3. The first CSS rule whose selector mentions `artboard` or `frame` with `width: Wpx` and `height: Hpx`.
 * Fallback: 1080x1350.
 */
export function detectArtboardSize(html: string): ArtboardSize {
  if (!html) return { ...DEFAULT_ARTBOARD_SIZE };
  const lower = html.toLowerCase();

  // 1. Viewport meta
  // Search for meta tags with name="viewport"
  const metaRegex = /<meta\s+[^>]*name=["']viewport["'][^>]*>/gi;
  let metaMatch: RegExpExecArray | null;
  while ((metaMatch = metaRegex.exec(lower)) !== null) {
    const tag = metaMatch[0];
    const contentMatch = tag.match(/content=["']([^"']*)["']/i);
    if (contentMatch) {
      const content = contentMatch[1];
      const wMatch = content.match(/width\s*=\s*(\d+)/i);
      const hMatch = content.match(/height\s*=\s*(\d+)/i);
      if (wMatch && hMatch) {
        const w = parseInt(wMatch[1], 10);
        const h = parseInt(hMatch[1], 10);
        if (w > 0 && h > 0) {
          return { w, h };
        }
      }
    }
  }

  // 2. data-artboard="WxH"
  const daPrefix = 'data-artboard="';
  const daIndex = lower.indexOf(daPrefix);
  if (daIndex !== -1) {
    const rest = lower.slice(daIndex + daPrefix.length);
    const quoteEnd = rest.indexOf('"');
    const val = quoteEnd !== -1 ? rest.slice(0, quoteEnd) : rest;
    const parts = val.split("x");
    if (parts.length === 2) {
      const w = parseInt(parts[0].trim(), 10);
      const h = parseInt(parts[1].trim(), 10);
      if (w > 0 && h > 0) {
        return { w, h };
      }
    }
  }

  // 3. First CSS rule whose selector mentions artboard or frame with width: Wpx; height: Hpx
  let search = 0;
  while (true) {
    const open = lower.indexOf("{", search);
    if (open === -1) break;
    const close = lower.indexOf("}", open);
    if (close === -1) break;

    // Selector is between previous '}' or start and '{'
    const prevClose = Math.max(lower.lastIndexOf("}", open), lower.lastIndexOf(">", open));
    const selectorStart = prevClose !== -1 ? prevClose + 1 : 0;
    const selector = lower.slice(selectorStart, open);

    if (selector.includes("artboard") || selector.includes("frame")) {
      const body = lower.slice(open + 1, close);
      const w = numAfterProp(body, "width");
      const h = numAfterProp(body, "height");
      if (w && h && w > 0 && h > 0) {
        return { w, h };
      }
    }
    search = close + 1;
  }

  return { ...DEFAULT_ARTBOARD_SIZE };
}

/**
 * Parse `width: 1280px` inside a CSS rule body, ignoring `min-width`, `max-width`, etc.
 */
function numAfterProp(body: string, prop: string): number | null {
  let search = 0;
  while (true) {
    const idx = body.indexOf(prop, search);
    if (idx === -1) break;

    // Ensure not preceded by alphanumeric or '-'
    if (idx > 0) {
      const prevChar = body[idx - 1];
      if (/[a-z0-9-]/i.test(prevChar)) {
        search = idx + prop.length;
        continue;
      }
    }

    const rest = body.slice(idx + prop.length).trimStart();
    if (!rest.startsWith(":")) {
      search = idx + prop.length;
      continue;
    }

    const afterColon = rest.slice(1).trimStart();
    const match = afterColon.match(/^(\d+)\s*px/i);
    if (match) {
      return parseInt(match[1], 10);
    }

    search = idx + prop.length;
  }
  return null;
}
