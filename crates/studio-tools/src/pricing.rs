use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default cost per image in USD when no override is configured.
pub const DEFAULT_IMAGE_USD: f64 = 0.04;

/// Default cost per second of video in USD when no override is configured.
pub const DEFAULT_VIDEO_USD_PER_S: f64 = 0.20;

/// Estimated cost configuration for an AI generation provider (or model).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProviderPricing {
    /// Cost per image in USD (default fallback: 0.04).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_usd: Option<f64>,
    /// Cost per second of generated video in USD (default fallback: 0.20).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_usd_per_s: Option<f64>,
    /// Optional model-specific overrides (e.g. "dall-e-3" -> pricing).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub models: HashMap<String, ProviderPricing>,
}

impl ProviderPricing {
    /// Resolve cost per image. Returns `(cost, cost_is_default)`.
    pub fn resolve_image_cost(&self, model: Option<&str>) -> (f64, bool) {
        if let Some(m) = model {
            if let Some(mp) = self.models.get(m) {
                if let Some(c) = mp.image_usd {
                    return (c, false);
                }
            }
        }
        if let Some(c) = self.image_usd {
            (c, false)
        } else {
            (DEFAULT_IMAGE_USD, true)
        }
    }

    /// Resolve cost per second of video. Returns `(cost_per_s, cost_is_default)`.
    pub fn resolve_video_cost_per_s(&self, model: Option<&str>) -> (f64, bool) {
        if let Some(m) = model {
            if let Some(mp) = self.models.get(m) {
                if let Some(c) = mp.video_usd_per_s {
                    return (c, false);
                }
            }
        }
        if let Some(c) = self.video_usd_per_s {
            (c, false)
        } else {
            (DEFAULT_VIDEO_USD_PER_S, true)
        }
    }

    /// Validate that all prices are non-negative and finite.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(c) = self.image_usd {
            if !c.is_finite() || c < 0.0 {
                return Err("image_usd phải là số không âm và hữu hạn".into());
            }
        }
        if let Some(c) = self.video_usd_per_s {
            if !c.is_finite() || c < 0.0 {
                return Err("video_usd_per_s phải là số không âm và hữu hạn".into());
            }
        }
        for (m, sub) in &self.models {
            sub.validate().map_err(|e| format!("Model \"{m}\": {e}"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pricing_defaults() {
        let p = ProviderPricing::default();
        let (cost, is_default) = p.resolve_image_cost(None);
        assert_eq!(cost, 0.04);
        assert!(is_default);

        let (v_cost, v_default) = p.resolve_video_cost_per_s(None);
        assert_eq!(v_cost, 0.20);
        assert!(v_default);
    }

    #[test]
    fn test_pricing_overrides() {
        let mut p = ProviderPricing {
            image_usd: Some(0.02),
            video_usd_per_s: Some(0.10),
            models: HashMap::new(),
        };
        let (cost, is_default) = p.resolve_image_cost(None);
        assert_eq!(cost, 0.02);
        assert!(!is_default);

        // Model override
        p.models.insert(
            "muse-image".into(),
            ProviderPricing {
                image_usd: Some(0.015),
                ..Default::default()
            },
        );
        let (m_cost, m_default) = p.resolve_image_cost(Some("muse-image"));
        assert_eq!(m_cost, 0.015);
        assert!(!m_default);

        // Unmatched model uses provider base
        let (other_cost, other_default) = p.resolve_image_cost(Some("other-model"));
        assert_eq!(other_cost, 0.02);
        assert!(!other_default);
    }

    #[test]
    fn test_pricing_validation() {
        let p = ProviderPricing {
            image_usd: Some(-0.01),
            ..Default::default()
        };
        assert!(p.validate().is_err());

        let p_nan = ProviderPricing {
            image_usd: Some(f64::NAN),
            ..Default::default()
        };
        assert!(p_nan.validate().is_err());

        let p_valid = ProviderPricing {
            image_usd: Some(0.0),
            video_usd_per_s: Some(0.05),
            ..Default::default()
        };
        assert!(p_valid.validate().is_ok());
    }
}
