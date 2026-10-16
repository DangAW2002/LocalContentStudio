/// RGB color representation in 0..=255.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parse hex color (e.g. `#FFF`, `#1E88E5`, `#1E88E5FF`).
    pub fn from_hex(hex: &str) -> Option<Self> {
        let s = hex.trim().trim_start_matches('#');
        match s.len() {
            3 => {
                let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?;
                Some(Self::new(r, g, b))
            }
            6 | 8 => {
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                Some(Self::new(r, g, b))
            }
            _ => None,
        }
    }

    /// Parse CSS color string (hex or rgb/rgba).
    pub fn parse_css(val: &str) -> Option<Self> {
        let trimmed = val.trim().to_lowercase();
        if trimmed.starts_with('#') {
            return Self::from_hex(&trimmed);
        }

        if trimmed.starts_with("rgb(") || trimmed.starts_with("rgba(") {
            let start = trimmed.find('(')?;
            let end = trimmed.find(')')?;
            let inner = &trimmed[start + 1..end];
            let parts: Vec<&str> = inner
                .split([',', ' ', '/'])
                .filter(|s| !s.is_empty())
                .collect();
            if parts.len() >= 3 {
                let r: u8 = parts[0].trim().parse().ok()?;
                let g: u8 = parts[1].trim().parse().ok()?;
                let b: u8 = parts[2].trim().parse().ok()?;
                return Some(Self::new(r, g, b));
            }
        }

        // Common CSS named colors
        match trimmed.as_str() {
            "white" => Some(Self::new(255, 255, 255)),
            "black" => Some(Self::new(0, 0, 0)),
            "red" => Some(Self::new(255, 0, 0)),
            "green" => Some(Self::new(0, 128, 0)),
            "blue" => Some(Self::new(0, 0, 255)),
            _ => None,
        }
    }

    /// Relative luminance according to WCAG 2.1.
    pub fn luminance(&self) -> f64 {
        let convert = |channel: u8| -> f64 {
            let c = (channel as f64) / 255.0;
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };

        0.2126 * convert(self.r) + 0.7152 * convert(self.g) + 0.0722 * convert(self.b)
    }

    /// Contrast ratio between this and another color (WCAG 2.1: 1.0 to 21.0).
    pub fn contrast_ratio(&self, other: &Self) -> f64 {
        let l1 = self.luminance();
        let l2 = other.luminance();
        let lighter = l1.max(l2);
        let darker = l1.min(l2);
        (lighter + 0.05) / (darker + 0.05)
    }

    /// Convert sRGB to CIE L*a*b* (D65 standard illuminant).
    pub fn to_lab(&self) -> (f64, f64, f64) {
        let to_linear = |c: u8| -> f64 {
            let v = (c as f64) / 255.0;
            if v > 0.04045 {
                ((v + 0.055) / 1.055).powf(2.4)
            } else {
                v / 12.92
            }
        };

        let r = to_linear(self.r);
        let g = to_linear(self.g);
        let b = to_linear(self.b);

        // sRGB to XYZ (D65)
        let x = r * 0.4124564 + g * 0.3575761 + b * 0.1804375;
        let y = r * 0.2126729 + g * 0.7151522 + b * 0.0721750;
        let z = r * 0.0193339 + g * 0.1191920 + b * 0.9503041;

        // D65 reference white
        let xn = 0.95047;
        let yn = 1.00000;
        let zn = 1.08883;

        let f = |t: f64| -> f64 {
            let delta = 6.0 / 29.0;
            if t > delta * delta * delta {
                t.cbrt()
            } else {
                t / (3.0 * delta * delta) + 4.0 / 29.0
            }
        };

        let fx = f(x / xn);
        let fy = f(y / yn);
        let fz = f(z / zn);

        let l = 116.0 * fy - 16.0;
        let a = 500.0 * (fx - fy);
        let b_val = 200.0 * (fy - fz);

        (l, a, b_val)
    }

    /// CIE76 Delta E color difference.
    pub fn delta_e_cie76(&self, other: &Self) -> f64 {
        let (l1, a1, b1) = self.to_lab();
        let (l2, a2, b2) = other.to_lab();

        let dl = l1 - l2;
        let da = a1 - a2;
        let db = b1 - b2;

        (dl * dl + da * da + db * db).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_parsing() {
        let c1 = Rgb::from_hex("#1E88E5").unwrap();
        assert_eq!(c1, Rgb::new(30, 136, 229));

        let c2 = Rgb::from_hex("fff").unwrap();
        assert_eq!(c2, Rgb::new(255, 255, 255));
    }

    #[test]
    fn test_wcag_contrast() {
        let black = Rgb::new(0, 0, 0);
        let white = Rgb::new(255, 255, 255);
        let ratio = black.contrast_ratio(&white);
        assert!((ratio - 21.0).abs() < 0.1);
    }

    #[test]
    fn test_delta_e() {
        let c1 = Rgb::new(255, 0, 0);
        let c2 = Rgb::new(255, 0, 0);
        assert_eq!(c1.delta_e_cie76(&c2), 0.0);
    }
}
