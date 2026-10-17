/**
 * Path cleaning utilities for user-facing display.
 * Strips Windows verbatim prefixes (`\\?\`, `\\?\UNC\`, `//?/`, `//?/UNC/`).
 */

export function cleanDisplayPath(path?: string | null): string {
  if (!path) return "";
  let clean = path;

  // Handle Windows verbatim UNC paths: \\?\UNC\server\share -> \\server\share
  if (clean.startsWith("\\\\?\\UNC\\")) {
    return "\\\\" + clean.slice(8);
  }
  if (clean.startsWith("//?/UNC/")) {
    return "//" + clean.slice(8);
  }

  // Handle regular Windows verbatim paths: \\?\D:\path -> D:\path
  if (clean.startsWith("\\\\?\\")) {
    return clean.slice(4);
  }
  if (clean.startsWith("//?/")) {
    return clean.slice(4);
  }

  return clean;
}
