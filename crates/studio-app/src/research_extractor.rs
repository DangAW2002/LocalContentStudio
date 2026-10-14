use regex::Regex;
use sha2::{Digest, Sha256};
use studio_core::research::{EvidenceSpan, ResearchSnapshot};

pub struct ResearchExtractor;

impl ResearchExtractor {
    pub const MAX_BODY_BYTES: usize = 10 * 1024 * 1024; // 10 MB maximum

    pub fn extract_from_html(html: &str, canonical_url: &str) -> ResearchSnapshot {
        let title = Self::extract_title(html).unwrap_or_else(|| "Untitled Source".to_string());
        let description = Self::extract_meta_description(html);
        let headings = Self::extract_headings(html);
        let (main_text, evidence_spans) = Self::extract_text_and_spans(html);

        let mut hasher = Sha256::new();
        hasher.update(canonical_url.as_bytes());
        hasher.update(html.as_bytes());
        let source_hash = hex::encode(hasher.finalize());

        ResearchSnapshot {
            title,
            description,
            canonical_url: canonical_url.to_string(),
            main_text,
            headings,
            evidence_spans,
            source_hash,
        }
    }

    fn extract_title(html: &str) -> Option<String> {
        let title_re = Regex::new(r"(?is)<title[^>]*>(.*?)</title>").ok()?;
        if let Some(caps) = title_re.captures(html) {
            let t = caps.get(1)?.as_str().trim();
            if !t.is_empty() {
                return Some(Self::strip_tags_and_unescape(t));
            }
        }

        // Fallback to og:title
        let og_re =
            Regex::new(r#"(?is)<meta[^>]+property=["']og:title["'][^>]+content=["']([^"']+)["']"#)
                .ok()?;
        if let Some(caps) = og_re.captures(html) {
            return Some(Self::strip_tags_and_unescape(caps.get(1)?.as_str().trim()));
        }

        None
    }

    fn extract_meta_description(html: &str) -> Option<String> {
        let og_desc = Regex::new(
            r#"(?is)<meta[^>]+property=["']og:description["'][^>]+content=["']([^"']+)["']"#,
        )
        .ok()?;
        if let Some(caps) = og_desc.captures(html) {
            return Some(Self::strip_tags_and_unescape(caps.get(1)?.as_str().trim()));
        }

        let meta_desc =
            Regex::new(r#"(?is)<meta[^>]+name=["']description["'][^>]+content=["']([^"']+)["']"#)
                .ok()?;
        if let Some(caps) = meta_desc.captures(html) {
            return Some(Self::strip_tags_and_unescape(caps.get(1)?.as_str().trim()));
        }

        None
    }

    fn extract_headings(html: &str) -> Vec<String> {
        let mut headings = Vec::new();
        let h_re = Regex::new(r"(?is)<h[1-3][^>]*>(.*?)</h[1-3]>").unwrap();
        for caps in h_re.captures_iter(html) {
            if let Some(m) = caps.get(1) {
                let clean = Self::strip_tags_and_unescape(m.as_str());
                if !clean.is_empty() && clean.len() < 300 {
                    headings.push(clean);
                }
            }
        }
        headings
    }

    fn extract_text_and_spans(html: &str) -> (String, Vec<EvidenceSpan>) {
        // Strip scripts and styles first
        let script_re = Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
        let no_scripts = script_re.replace_all(html, " ");
        let style_re = Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
        let clean_html = style_re.replace_all(&no_scripts, " ");

        // Extract paragraphs
        let p_re = Regex::new(r"(?is)<p[^>]*>(.*?)</p>").unwrap();
        let mut paragraphs = Vec::new();
        let mut spans = Vec::new();

        for caps in p_re.captures_iter(&clean_html) {
            if let Some(m) = caps.get(1) {
                let text = Self::strip_tags_and_unescape(m.as_str());
                if text.len() > 20 {
                    spans.push(EvidenceSpan {
                        text: text.clone(),
                        section: "body".into(),
                        confidence: 0.95,
                    });
                    paragraphs.push(text);
                }
            }
        }

        let main_text = paragraphs.join("\n\n");
        (main_text, spans)
    }

    fn strip_tags_and_unescape(text: &str) -> String {
        let tag_re = Regex::new(r"<[^>]+>").unwrap();
        let stripped = tag_re.replace_all(text, "");
        stripped
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&nbsp;", " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}
