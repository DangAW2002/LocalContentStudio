/**
 * Vietnamese diacritic folding and fuzzy scoring logic.
 * Inspired by VS Code's fuzzy scoring algorithm (MIT License).
 */

/**
 * Normalizes Vietnamese string to ASCII by folding diacritics and converting to lowercase.
 * Specifically handles 'đ'/'Đ' -> 'd'/'D' and unicode combining marks.
 */
export function foldVietnamese(str: string): string {
  return str
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/đ/g, "d")
    .replace(/Đ/g, "d")
    .toLowerCase();
}

export interface FuzzyScoreResult {
  score: number;
  /** Matched character indices in the original target string */
  matches: number[];
}

function isWordSeparator(ch: string): boolean {
  return /[\s\-_/\\.:,;()[\]{}|#@+*?~`'"!%^&=]/.test(ch);
}

/**
 * Fuzzy scores a query against a target string.
 *
 * Scoring rules:
 * - Sequential matches get bonus.
 * - Matches after word separators or camelCase boundaries get bonus.
 * - Matches at start of word get bonus.
 * - Target exact prefix gets high bonus.
 * - Target exact match gets maximum bonus.
 * - Distance between match indices penalizes score.
 *
 * Returns null if query does not match target.
 */
export function fuzzyScore(query: string, target: string): FuzzyScoreResult | null {
  const trimmedQuery = query.trim();
  if (!trimmedQuery) {
    return { score: 0, matches: [] };
  }

  const queryFolded = foldVietnamese(trimmedQuery);
  const targetFolded = foldVietnamese(target);

  const queryLen = queryFolded.length;
  const targetLen = targetFolded.length;

  if (queryLen > targetLen) {
    return null;
  }

  // Exact match bonus
  if (queryFolded === targetFolded) {
    const matches: number[] = [];
    for (let i = 0; i < targetLen; i++) matches.push(i);
    return { score: 1000 + queryLen * 10, matches };
  }

  // Exact prefix match bonus
  if (targetFolded.startsWith(queryFolded)) {
    const matches: number[] = [];
    for (let i = 0; i < queryLen; i++) matches.push(i);
    return { score: 500 + queryLen * 10, matches };
  }

  // Dynamic programming / Greedy with word-boundary prioritization to find best match
  // For command palette queries (typically short < 30 chars), find match positions
  const matches: number[] = [];
  let tIdx = 0;
  let score = 0;
  let consecutive = 0;

  for (let qIdx = 0; qIdx < queryLen; qIdx++) {
    const qChar = queryFolded[qIdx];
    let bestPos = -1;
    let bestPosBonus = -1;

    // Look ahead for candidate positions in target
    for (let i = tIdx; i < targetLen; i++) {
      if (targetFolded[i] === qChar) {
        let bonus = 0;
        // Word start bonus
        if (i === 0 || isWordSeparator(target[i - 1])) {
          bonus += 15;
        } else if (
          // Uppercase / camelCase transition
          target[i] !== targetFolded[i] &&
          target[i - 1] === targetFolded[i - 1]
        ) {
          bonus += 10;
        }

        // Consecutive bonus
        if (i === tIdx) {
          bonus += 8;
        }

        if (bonus > bestPosBonus || bestPos === -1) {
          bestPos = i;
          bestPosBonus = bonus;
          // If we found a high value match right at start or consecutive, take it
          if (bonus >= 15 || (i === tIdx && consecutive > 0)) {
            break;
          }
        }
      }
    }

    if (bestPos === -1) {
      return null;
    }

    // Calculate score for this character
    score += 10 + bestPosBonus;
    if (bestPos === tIdx && qIdx > 0) {
      consecutive++;
      score += consecutive * 5;
    } else {
      consecutive = 0;
      score -= (bestPos - tIdx) * 1; // Penalty for gaps
    }

    matches.push(bestPos);
    tIdx = bestPos + 1;
  }

  return { score, matches };
}
