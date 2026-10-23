//! Vietnamese text normalization for Text-to-Speech (TTS).
//!
//! Transforms numbers, currencies, percentages, dates, units, and common
//! abbreviations into spoken Vietnamese words prior to speech synthesis.
//! In-text pause tags `[ngắt ...]` are strictly preserved without normalization.

use regex::{Captures, Regex};
use std::sync::LazyLock;

static TAG_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[ngắt\s+[^\]]+\]").expect("valid tag regex"));

static DATE_DMY_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(\d{1,2})/(\d{1,2})/(\d{4})\b").expect("valid date regex"));

static DATE_DM_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(\d{1,2})/(\d{1,2})\b").expect("valid date regex"));

static TIME_HM_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(\d{1,2})h(\d{1,2})\b").expect("valid time regex"));

static TIME_H_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(\d{1,2})h\b").expect("valid time regex"));

static PERCENT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d+(?:[.,]\d+)?)\s*%").expect("valid percent regex"));

static CURRENCY_VND_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(\d+(?:[.,]\d+)?)\s*(?:đ|vnđ|vnd)\b").expect("valid vnd regex")
});

static CURRENCY_USD_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:\$\s*(\d+(?:[.,]\d+)?)|(\d+(?:[.,]\d+)?)\s*usd\b)")
        .expect("valid usd regex")
});

static UNIT_TEMP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(\d+(?:[.,]\d+)?)\s*(?:°c|oc|độ\s*c)\b").expect("valid temp regex")
});

static UNIT_SPEED_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(\d+(?:[.,]\d+)?)\s*km/h\b").expect("valid speed regex"));

static UNIT_KM_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(\d+(?:[.,]\d+)?)\s*km\b").expect("valid km regex"));

static UNIT_KG_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(\d+(?:[.,]\d+)?)\s*kg\b").expect("valid kg regex"));

static UNIT_CM_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(\d+(?:[.,]\d+)?)\s*cm\b").expect("valid cm regex"));

static UNIT_MM_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(\d+(?:[.,]\d+)?)\s*mm\b").expect("valid mm regex"));

static UNIT_METER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(\d+(?:[.,]\d+)?)\s*m\b").expect("valid meter regex"));

static UNIT_ML_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(\d+(?:[.,]\d+)?)\s*ml\b").expect("valid ml regex"));

static UNIT_LITER_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(\d+(?:[.,]\d+)?)\s*l\b").expect("valid liter regex"));

static UNIT_MINUTES_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(\d+)\s*phút\b").expect("valid minutes regex"));

static UNIT_SECONDS_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(\d+)\s*(?:s|giây)\b").expect("valid seconds regex"));

static THOUSANDS_INT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d{1,3}(?:\.\d{3})+\b").expect("valid thousands regex"));

static COMMA_DECIMAL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(\d+),(\d+)\b").expect("valid comma decimal regex"));

static STANDALONE_INT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\d+\b").expect("valid int regex"));

/// Map single digit to Vietnamese word.
fn digit_to_word(d: u8) -> &'static str {
    match d {
        0 => "không",
        1 => "một",
        2 => "hai",
        3 => "ba",
        4 => "bốn",
        5 => "năm",
        6 => "sáu",
        7 => "bảy",
        8 => "tám",
        9 => "chín",
        _ => "",
    }
}

/// Convert a 3-digit group (0..=999) to Vietnamese words.
/// `has_higher` is true if there are higher non-zero groups (millions, thousands, etc.).
fn read_group_3(n: u32, has_higher: bool) -> String {
    let hundreds = (n / 100) as u8;
    let tens = ((n % 100) / 10) as u8;
    let units = (n % 10) as u8;

    let mut parts = Vec::new();

    if hundreds > 0 {
        parts.push(format!("{} trăm", digit_to_word(hundreds)));
    } else if has_higher && (tens > 0 || units > 0) {
        parts.push("không trăm".to_string());
    }

    if tens > 1 {
        parts.push(format!("{} mươi", digit_to_word(tens)));
        if units == 1 {
            parts.push("mốt".to_string());
        } else if units == 5 {
            parts.push("lăm".to_string());
        } else if units > 0 {
            parts.push(digit_to_word(units).to_string());
        }
    } else if tens == 1 {
        parts.push("mười".to_string());
        if units == 5 {
            parts.push("lăm".to_string());
        } else if units > 0 {
            parts.push(digit_to_word(units).to_string());
        }
    } else {
        // tens == 0
        if units > 0 {
            if hundreds > 0 || has_higher {
                parts.push("lẻ".to_string());
            }
            parts.push(digit_to_word(units).to_string());
        }
    }

    parts.join(" ")
}

/// Convert an integer (up to billions) into spoken Vietnamese words.
pub fn integer_to_vietnamese(n: i64) -> String {
    if n == 0 {
        return "không".to_string();
    }

    let is_negative = n < 0;
    let mut val = n.unsigned_abs();

    let billions = val / 1_000_000_000;
    val %= 1_000_000_000;
    let millions = val / 1_000_000;
    val %= 1_000_000;
    let thousands = val / 1_000;
    val %= 1_000;
    let units = val;

    let mut result_parts = Vec::new();
    if is_negative {
        result_parts.push("âm".to_string());
    }

    let mut has_higher = false;

    if billions > 0 {
        let b_str = read_group_3(billions as u32, false);
        result_parts.push(format!("{b_str} tỷ"));
        has_higher = true;
    }

    if millions > 0 {
        let m_str = read_group_3(millions as u32, has_higher);
        result_parts.push(format!("{m_str} triệu"));
        has_higher = true;
    } else if has_higher && (thousands > 0 || units > 0) {
        // Skip
    }

    if thousands > 0 {
        let t_str = read_group_3(thousands as u32, has_higher);
        result_parts.push(format!("{t_str} nghìn"));
        has_higher = true;
    }

    if units > 0 {
        let u_str = read_group_3(units as u32, has_higher);
        result_parts.push(u_str);
    }

    result_parts.join(" ")
}

/// Convert decimal digits after comma into words, e.g. "5" -> "năm", "05" -> "không năm", "25" -> "hai mươi lăm".
pub fn decimal_part_to_vietnamese(part: &str) -> String {
    if part.starts_with('0') || part.len() > 3 {
        // Read digit by digit
        part.chars()
            .filter_map(|c| c.to_digit(10).map(|d| digit_to_word(d as u8)))
            .collect::<Vec<_>>()
            .join(" ")
    } else if let Ok(val) = part.parse::<i64>() {
        integer_to_vietnamese(val)
    } else {
        part.to_string()
    }
}

/// Parse a raw number string in Vietnamese format ("1.500", "2,5", "100", "2.500.000") to spoken words.
pub fn parse_number_to_vietnamese(raw: &str) -> String {
    let trimmed = raw.trim();

    // Check for comma decimal "2,5"
    if let Some((int_p, dec_p)) = trimmed.split_once(',') {
        let clean_int = int_p.replace('.', "");
        if let Ok(int_val) = clean_int.parse::<i64>() {
            let int_word = integer_to_vietnamese(int_val);
            let dec_word = decimal_part_to_vietnamese(dec_p);
            return format!("{int_word} phẩy {dec_word}");
        }
    }

    // Check for dot-thousands separator e.g. "1.500"
    if trimmed.contains('.') {
        let clean_int = trimmed.replace('.', "");
        if let Ok(int_val) = clean_int.parse::<i64>() {
            return integer_to_vietnamese(int_val);
        }
    }

    // Simple integer
    if let Ok(int_val) = trimmed.parse::<i64>() {
        return integer_to_vietnamese(int_val);
    }

    trimmed.to_string()
}

/// Normalize common Vietnamese abbreviations.
fn normalize_abbreviations(text: &str) -> String {
    let mut res = text.to_string();

    // Common location and government abbreviations
    let abbrevs: &[(&str, &str)] = &[
        // Locations
        (r"(?i)\bTP\.?\s*HCM\b", "thành phố Hồ Chí Minh"),
        (r"(?i)\bTPHCM\b", "thành phố Hồ Chí Minh"),
        (r"(?i)\bTP\.?\s*HN\b", "thành phố Hà Nội"),
        (r"\bHN\b", "Hà Nội"),
        (r"\bVN\b", "Việt Nam"),
        // Government & Organizations
        (r"\bUBND\b", "Ủy ban nhân dân"),
        (r"\bHĐND\b", "Hội đồng nhân dân"),
        (r"\bBCA\b", "Bộ Công an"),
        (r"\bBQP\b", "Bộ Quốc phòng"),
        (r"\bBYT\b", "Bộ Y tế"),
        (r"(?i)\b(?:bộ\s+)?gd&đt\b", "Bộ Giáo dục và Đào tạo"),
        (r"\bBGD\b", "Bộ Giáo dục"),
        (r"\bCNTT\b", "công nghệ thông tin"),
        (r"\bAI\b", "A I"),
        (r"\bNXB\b", "nhà xuất bản"),
        (r"\bSTT\b", "số thứ tự"),
        (r"\bCLB\b", "câu lạc bộ"),
        // Titles (only when preceding a word or dot)
        (r"(?i)\bbs\.?\s+(?=[A-ZÀ-Ỹ])", "bác sĩ "),
        (r"(?i)\bgs\.?\s+(?=[A-ZÀ-Ỹ])", "giáo sư "),
        (r"(?i)\bpgs\.?\s+(?=[A-ZÀ-Ỹ])", "phó giáo sư "),
        (r"(?i)\bts\.?\s+(?=[A-ZÀ-Ỹ])", "tiến sĩ "),
        (r"(?i)\bths\.?\s+(?=[A-ZÀ-Ỹ])", "thạc sĩ "),
        // Common miscellaneous
        (r"(?i)\bv\.v\.\.\.", "vân vân"),
        (r"(?i)\bv\.v\.", "vân vân"),
        (r"(?i)\bv\.v\b", "vân vân"),
        (r"(?i)\bk/c\b", "khoảng cách"),
    ];

    for (pattern, replacement) in abbrevs {
        if let Ok(re) = Regex::new(pattern) {
            res = re.replace_all(&res, *replacement).to_string();
        }
    }

    res
}

/// Normalize Vietnamese text for Text-to-Speech synthesis.
///
/// Converts numbers, currencies, percentages, dates, units, and abbreviations.
/// Substrings inside tags like `[ngắt 0.7]` are strictly left unchanged.
pub fn normalize_vietnamese_text(text: &str) -> String {
    if text.trim().is_empty() {
        return text.to_string();
    }

    // Step 1: Protect `[ngắt ...]` tags
    let mut protected_tags: Vec<String> = Vec::new();
    let mut text_with_placeholders = TAG_REGEX
        .replace_all(text, |caps: &Captures| {
            let tag = caps[0].to_string();
            let idx = protected_tags.len();
            protected_tags.push(tag);
            format!("__STUDIO_TAG_{idx}__")
        })
        .to_string();

    // Step 2: Abbreviations
    text_with_placeholders = normalize_abbreviations(&text_with_placeholders);

    // Step 3: Dates (dd/mm/yyyy then dd/mm)
    text_with_placeholders = DATE_DMY_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let day = parse_number_to_vietnamese(&caps[1]);
            let month = parse_number_to_vietnamese(&caps[2]);
            let year = parse_number_to_vietnamese(&caps[3]);
            format!("{day} tháng {month} năm {year}")
        })
        .to_string();

    text_with_placeholders = DATE_DM_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let day = parse_number_to_vietnamese(&caps[1]);
            let month = parse_number_to_vietnamese(&caps[2]);
            format!("{day} tháng {month}")
        })
        .to_string();

    // Step 4: Time (3h30 -> ba giờ ba mươi phút; 3h -> ba giờ)
    text_with_placeholders = TIME_HM_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let h = parse_number_to_vietnamese(&caps[1]);
            let m = parse_number_to_vietnamese(&caps[2]);
            format!("{h} giờ {m} phút")
        })
        .to_string();

    text_with_placeholders = TIME_H_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let h = parse_number_to_vietnamese(&caps[1]);
            format!("{h} giờ")
        })
        .to_string();

    // Step 5: Percentages ("25%" -> "hai mươi lăm phần trăm")
    text_with_placeholders = PERCENT_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let num = parse_number_to_vietnamese(&caps[1]);
            format!("{num} phần trăm")
        })
        .to_string();

    // Step 6: Currencies ("50.000đ" -> "năm mươi nghìn đồng", "$50" -> "năm mươi đô la")
    text_with_placeholders = CURRENCY_VND_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let num = parse_number_to_vietnamese(&caps[1]);
            format!("{num} đồng")
        })
        .to_string();

    text_with_placeholders = CURRENCY_USD_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let raw_num = caps
                .get(1)
                .or_else(|| caps.get(2))
                .map(|m| m.as_str())
                .unwrap_or("");
            let num = parse_number_to_vietnamese(raw_num);
            format!("{num} đô la")
        })
        .to_string();

    // Step 7: Units (temperatures, speed, lengths, weights)
    text_with_placeholders = UNIT_TEMP_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let num = parse_number_to_vietnamese(&caps[1]);
            format!("{num} độ C")
        })
        .to_string();

    text_with_placeholders = UNIT_SPEED_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let num = parse_number_to_vietnamese(&caps[1]);
            format!("{num} ki-lô-mét trên giờ")
        })
        .to_string();

    text_with_placeholders = UNIT_KM_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let num = parse_number_to_vietnamese(&caps[1]);
            format!("{num} ki-lô-mét")
        })
        .to_string();

    text_with_placeholders = UNIT_KG_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let num = parse_number_to_vietnamese(&caps[1]);
            format!("{num} ki-lô-gam")
        })
        .to_string();

    text_with_placeholders = UNIT_CM_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let num = parse_number_to_vietnamese(&caps[1]);
            format!("{num} xen-ti-mét")
        })
        .to_string();

    text_with_placeholders = UNIT_MM_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let num = parse_number_to_vietnamese(&caps[1]);
            format!("{num} mi-li-mét")
        })
        .to_string();

    text_with_placeholders = UNIT_METER_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let num = parse_number_to_vietnamese(&caps[1]);
            format!("{num} mét")
        })
        .to_string();

    text_with_placeholders = UNIT_ML_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let num = parse_number_to_vietnamese(&caps[1]);
            format!("{num} mi-li-lít")
        })
        .to_string();

    text_with_placeholders = UNIT_LITER_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let num = parse_number_to_vietnamese(&caps[1]);
            format!("{num} lít")
        })
        .to_string();

    text_with_placeholders = UNIT_MINUTES_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let num = parse_number_to_vietnamese(&caps[1]);
            format!("{num} phút")
        })
        .to_string();

    text_with_placeholders = UNIT_SECONDS_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            let num = parse_number_to_vietnamese(&caps[1]);
            format!("{num} giây")
        })
        .to_string();

    // Step 8: Vietnamese thousands-separated numbers "1.500" -> "một nghìn năm trăm"
    text_with_placeholders = THOUSANDS_INT_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            parse_number_to_vietnamese(&caps[0])
        })
        .to_string();

    // Step 9: Comma decimal numbers "2,5" -> "hai phẩy năm"
    text_with_placeholders = COMMA_DECIMAL_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            parse_number_to_vietnamese(&caps[0])
        })
        .to_string();

    // Step 10: Remaining standalone integers
    text_with_placeholders = STANDALONE_INT_REGEX
        .replace_all(&text_with_placeholders, |caps: &Captures| {
            parse_number_to_vietnamese(&caps[0])
        })
        .to_string();

    // Step 11: Restore protected tags `[ngắt ...]`
    for (idx, tag) in protected_tags.iter().enumerate() {
        let placeholder = format!("__STUDIO_TAG_{idx}__");
        text_with_placeholders = text_with_placeholders.replace(&placeholder, tag);
    }

    text_with_placeholders
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_conversion_cases() {
        assert_eq!(integer_to_vietnamese(0), "không");
        assert_eq!(integer_to_vietnamese(1), "một");
        assert_eq!(integer_to_vietnamese(5), "năm");
        assert_eq!(integer_to_vietnamese(10), "mười");
        assert_eq!(integer_to_vietnamese(15), "mười lăm");
        assert_eq!(integer_to_vietnamese(21), "hai mươi mốt");
        assert_eq!(integer_to_vietnamese(25), "hai mươi lăm");
        assert_eq!(integer_to_vietnamese(100), "một trăm");
        assert_eq!(integer_to_vietnamese(105), "một trăm lẻ năm");
        assert_eq!(integer_to_vietnamese(115), "một trăm mười lăm");
        assert_eq!(integer_to_vietnamese(1000), "một nghìn");
        assert_eq!(integer_to_vietnamese(1500), "một nghìn năm trăm");
        assert_eq!(integer_to_vietnamese(1000000), "một triệu");
        assert_eq!(integer_to_vietnamese(1500000), "một triệu năm trăm nghìn");
        assert_eq!(integer_to_vietnamese(1000000000), "một tỷ");
    }

    #[test]
    fn test_normalization_15_cases() {
        // 1. Số nguyên kiểu nghìn VN
        assert_eq!(
            normalize_vietnamese_text("Dự án có 1.500 người tham gia."),
            "Dự án có một nghìn năm trăm người tham gia."
        );

        // 2. Số thập phân phẩy VN
        assert_eq!(
            normalize_vietnamese_text("Tốc độ tăng 2,5 lần."),
            "Tốc độ tăng hai phẩy năm lần."
        );

        // 3. Phần trăm
        assert_eq!(
            normalize_vietnamese_text("Tỷ lệ đạt 25% tổng số."),
            "Tỷ lệ đạt hai mươi lăm phần trăm tổng số."
        );

        // 4. Giờ rút gọn
        assert_eq!(
            normalize_vietnamese_text("Cuộc họp bắt đầu lúc 3h chiều."),
            "Cuộc họp bắt đầu lúc ba giờ chiều."
        );

        // 5. Giờ và phút rút gọn
        assert_eq!(
            normalize_vietnamese_text("Chuyến bay khởi hành lúc 3h30."),
            "Chuyến bay khởi hành lúc ba giờ ba mươi phút."
        );

        // 6. Viết tắt thành phố Hồ Chí Minh
        assert_eq!(
            normalize_vietnamese_text("Sự kiện diễn ra tại TP.HCM ngày mai."),
            "Sự kiện diễn ra tại thành phố Hồ Chí Minh ngày mai."
        );

        // 7. Vân vân (v.v.)
        assert_eq!(
            normalize_vietnamese_text("Sách, báo, tạp chí, v.v."),
            "Sách, báo, tạp chí, vân vân"
        );

        // 8. Ngày tháng ngắn dd/mm
        assert_eq!(
            normalize_vietnamese_text("Hạn chót là 24/10."),
            "Hạn chót là hai mươi bốn tháng mười."
        );

        // 9. Ngày tháng năm đầy đủ dd/mm/yyyy
        assert_eq!(
            normalize_vietnamese_text("Ngày thành lập là 24/10/2026."),
            "Ngày thành lập là hai mươi bốn tháng mười năm hai nghìn không trăm hai mươi sáu."
        );

        // 10. Khoảng cách km
        assert_eq!(
            normalize_vietnamese_text("Quãng đường dài 100 km."),
            "Quãng đường dài một trăm ki-lô-mét."
        );

        // 11. Khối lượng kg
        assert_eq!(
            normalize_vietnamese_text("Gói hàng nặng 50 kg."),
            "Gói hàng nặng năm mươi ki-lô-gam."
        );

        // 12. Nhiệt độ độ C
        assert_eq!(
            normalize_vietnamese_text("Hôm nay nhiệt độ đạt 37°C."),
            "Hôm nay nhiệt độ đạt ba mươi bảy độ C."
        );

        // 13. Viết tắt Việt Nam
        assert_eq!(
            normalize_vietnamese_text("Đội tuyển bóng đá VN chiến thắng."),
            "Đội tuyển bóng đá Việt Nam chiến thắng."
        );

        // 14. Tiền tệ VNĐ
        assert_eq!(
            normalize_vietnamese_text("Giá vé chỉ 50.000đ."),
            "Giá vé chỉ năm mươi nghìn đồng."
        );

        // 15. Tiền tệ USD
        assert_eq!(
            normalize_vietnamese_text("Chi phí là $100 hoặc 50 USD."),
            "Chi phí là một trăm đô la hoặc năm mươi đô la."
        );

        // 16. Bảo toàn thẻ [ngắt ...]
        assert_eq!(
            normalize_vietnamese_text("Xin chú ý [ngắt 0.7] đây là thông báo 1.500 người."),
            "Xin chú ý [ngắt 0.7] đây là thông báo một nghìn năm trăm người."
        );

        // 17. Thời gian phút và giây
        assert_eq!(
            normalize_vietnamese_text("Thời lượng còn 15 phút 30 giây."),
            "Thời lượng còn mười lăm phút ba mươi giây."
        );
    }
}
