//! Kit suggestion via case and diacritics-insensitive keyword matching.

use deunicode::deunicode;

use crate::model::Catalog;

/// Normalize a string for matching: trim, lowercase, and strip diacritics.
fn normalize(s: &str) -> String {
    deunicode(s).to_lowercase()
}

/// Suggest matching kits based on input prompt or project description text.
///
/// Returns a sorted list of `(kit_id, score)` tuples in descending order of score.
/// Score corresponds to how many keyword matches hit in the normalized text.
pub fn suggest_kits(catalog: &Catalog, text: &str) -> Vec<(String, usize)> {
    let norm_text = normalize(text);
    let mut scores = Vec::new();

    for (kid, kit) in &catalog.kits {
        let mut score = 0;

        // Check kit_id itself
        let norm_kid = normalize(kid);
        if norm_text.contains(&norm_kid) {
            score += 3;
        }

        // Check kit title
        let norm_title = normalize(&kit.title);
        for word in norm_title.split_whitespace() {
            if word.len() >= 3 && norm_text.contains(word) {
                score += 1;
            }
        }

        // Check explicit keywords
        for kw in &kit.keywords {
            let norm_kw = normalize(kw);
            if !norm_kw.is_empty() && norm_text.contains(&norm_kw) {
                score += 2;
            }
        }

        if score > 0 {
            scores.push((kid.clone(), score));
        }
    }

    scores.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    scores
}
