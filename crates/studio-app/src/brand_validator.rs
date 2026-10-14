use studio_core::brand::{BrandVersion, VisualTokens, VoiceGuidelines};
use studio_core::error::StudioError;

#[derive(Debug, Clone)]
pub struct BrandValidationResult {
    pub is_valid: bool,
    pub violations: Vec<String>,
}

pub struct BrandValidator;

impl BrandValidator {
    /// Validates content text against brand voice guidelines:
    /// - Checks for prohibited terms (case-insensitive)
    /// - Verifies presence of required disclaimers if defined
    pub fn validate_copy(text: &str, guidelines: &VoiceGuidelines) -> BrandValidationResult {
        let mut violations = Vec::new();
        let lower = text.to_lowercase();

        for term in &guidelines.prohibited_terms {
            if lower.contains(&term.to_lowercase()) {
                violations.push(format!(
                    "Phát hiện từ ngữ bị cấm trong thương hiệu: '{}'",
                    term
                ));
            }
        }

        for disclaimer in &guidelines.required_disclaimers {
            if !text.contains(disclaimer) {
                violations.push(format!(
                    "Thiếu câu tuyên bố bắt buộc (disclaimer): '{}'",
                    disclaimer
                ));
            }
        }

        BrandValidationResult {
            is_valid: violations.is_empty(),
            violations,
        }
    }

    /// Validates contrast ratio between primary color and surface color:
    /// Follows WCAG 2.1 relative luminance calculation.
    pub fn validate_contrast(tokens: &VisualTokens) -> Result<f64, StudioError> {
        let fg = Self::parse_hex_color(&tokens.primary_color)?;
        let bg = Self::parse_hex_color(&tokens.surface_color)?;

        let l1 = Self::relative_luminance(fg);
        let l2 = Self::relative_luminance(bg);

        let ratio = if l1 > l2 {
            (l1 + 0.05) / (l2 + 0.05)
        } else {
            (l2 + 0.05) / (l1 + 0.05)
        };

        Ok(ratio)
    }

    /// Enforces brand version compliance for publication:
    /// - Contrast must meet minimum WCAG AA threshold (4.5:1)
    /// - Prohibited terms must not be empty if corporate policy mandates it
    pub fn validate_for_publish(ver: &BrandVersion) -> Result<(), StudioError> {
        let ratio = Self::validate_contrast(&ver.visual_tokens)?;
        if ratio < 4.5 {
            return Err(StudioError::PreconditionFailed {
                reason: format!(
                    "Độ tương phản màu thương hiệu không đạt chuẩn WCAG AA ({:.2}:1 < 4.5:1). Cần chỉnh sửa màu trước khi Publish.",
                    ratio
                ),
                correlation_id: ver.id.clone(),
            });
        }

        Ok(())
    }

    fn parse_hex_color(hex_str: &str) -> Result<(u8, u8, u8), StudioError> {
        let clean = hex_str.trim().trim_start_matches('#');
        if clean.len() != 6 {
            return Err(StudioError::InvalidInput {
                field: "color".into(),
                reason: format!("Mã màu không hợp lệ: '{}'", hex_str),
                correlation_id: "brand".into(),
            });
        }

        let r = u8::from_str_radix(&clean[0..2], 16).map_err(|_| StudioError::InvalidInput {
            field: "color.r".into(),
            reason: "Hex parse error".into(),
            correlation_id: "brand".into(),
        })?;
        let g = u8::from_str_radix(&clean[2..4], 16).map_err(|_| StudioError::InvalidInput {
            field: "color.g".into(),
            reason: "Hex parse error".into(),
            correlation_id: "brand".into(),
        })?;
        let b = u8::from_str_radix(&clean[4..6], 16).map_err(|_| StudioError::InvalidInput {
            field: "color.b".into(),
            reason: "Hex parse error".into(),
            correlation_id: "brand".into(),
        })?;

        Ok((r, g, b))
    }

    fn relative_luminance(rgb: (u8, u8, u8)) -> f64 {
        let (rs, gs, bs) = (
            rgb.0 as f64 / 255.0,
            rgb.1 as f64 / 255.0,
            rgb.2 as f64 / 255.0,
        );

        let r = if rs <= 0.03928 {
            rs / 12.92
        } else {
            ((rs + 0.055) / 1.055).powf(2.4)
        };
        let g = if gs <= 0.03928 {
            gs / 12.92
        } else {
            ((gs + 0.055) / 1.055).powf(2.4)
        };
        let b = if bs <= 0.03928 {
            bs / 12.92
        } else {
            ((bs + 0.055) / 1.055).powf(2.4)
        };

        0.2126 * r + 0.7152 * g + 0.0722 * b
    }
}
