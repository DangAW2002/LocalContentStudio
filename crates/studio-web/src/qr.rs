//! QR Code generation for LAN pairing.

use qrcode::render::svg;
use qrcode::QrCode;

/// Generate an SVG string of a QR code encoding the provided text/URL.
pub fn generate_qr_svg(content: &str) -> Result<String, String> {
    let code = QrCode::new(content.as_bytes()).map_err(|e| format!("Lỗi tạo mã QR: {e}"))?;
    let image = code
        .render::<svg::Color>()
        .min_dimensions(240, 240)
        .dark_color(svg::Color("#111827"))
        .light_color(svg::Color("#ffffff"))
        .build();
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qr_generation() {
        let svg = generate_qr_svg("http://192.168.1.15:3000").expect("generate svg");
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
    }
}
