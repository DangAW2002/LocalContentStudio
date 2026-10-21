/**
 * Mapping checker error codes to clear, friendly Vietnamese labels.
 */
export const CHECK_ERROR_LABELS: Record<string, string> = {
  DIMENSIONS_MISMATCH: "Sai kích thước định dạng",
  NO_RENDERS: "Chưa có bản render",
  DURATION_TOO_SHORT: "Thời lượng video quá ngắn",
  DURATION_EXCEEDED: "Thời lượng video vượt quá giới hạn",
  CAPTIONS_MISSING: "Thiếu phụ đề bắt buộc",
  BRAND_CONTRAST_LOW: "Độ tương phản màu thấp",
  LOGO_MISSING: "Thiếu logo thương hiệu",
  PALETTE_MISMATCH: "Sai bảng màu thương hiệu",
  SAFE_ZONE_VIOLATION: "Vi phạm vùng an toàn (Safe Zone)",
  SAFE_ZONE_TOP_VIOLATION: "Vi phạm vùng an toàn trên",
  SAFE_ZONE_BOTTOM_VIOLATION: "Vi phạm vùng an toàn dưới",
  SAFE_ZONE_LEFT_VIOLATION: "Vi phạm vùng an toàn trái",
  SAFE_ZONE_RIGHT_VIOLATION: "Vi phạm vùng an toàn phải",
  TEXT_LIMIT_EXCEEDED: "Vượt giới hạn ký tự",
  VOICE_WORD_DISALLOWED: "Chứa từ ngữ thương hiệu cấm dùng",
  BRAND_NOT_FOUND: "Không tìm thấy cấu hình thương hiệu",
  INPUTS_UNDECLARED: "Dùng tệp chưa khai báo trong inputs",
};

/**
 * Returns user-friendly Vietnamese label for a checker code.
 */
export function getCheckErrorLabel(code: string): string {
  return CHECK_ERROR_LABELS[code] || code;
}
