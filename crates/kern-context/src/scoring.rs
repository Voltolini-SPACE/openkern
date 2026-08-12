//! Deterministic scoring primitives (G8.4).
//!
//! No model is involved. Tokenization splits on non-alphanumerics and on `camelCase` /
//! `snake_case` boundaries so a query term like `head` matches a symbol `resolve_head`.
//!
//! Scores are ratios of small counts, so `usize as f64` casts are intended and safe here.
#![allow(clippy::cast_precision_loss)]

/// Split text into lowercase tokens, breaking `snake_case` and `camelCase`.
#[must_use]
pub fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            if ch.is_uppercase() && prev_lower && !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            cur.push(ch.to_ascii_lowercase());
            prev_lower = ch.is_lowercase() || ch.is_numeric();
        } else {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev_lower = false;
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out.retain(|t| t.len() >= 2);
    out
}

/// Fraction of query tokens present in `text` (0.0..=1.0).
#[must_use]
pub fn lexical_overlap(query_tokens: &[String], text: &str) -> f64 {
    if query_tokens.is_empty() {
        return 0.0;
    }
    let hay: std::collections::BTreeSet<String> = tokenize(text).into_iter().collect();
    let matched = query_tokens.iter().filter(|q| hay.contains(*q)).count();
    matched as f64 / query_tokens.len() as f64
}

/// Symbol-name match strength: exact token equality scores highest; partial token overlap
/// scores proportionally.
#[must_use]
pub fn name_match(query_tokens: &[String], name: &str) -> f64 {
    if query_tokens.is_empty() {
        return 0.0;
    }
    let name_lower = name.to_lowercase();
    if query_tokens.contains(&name_lower) {
        return 1.0;
    }
    let name_tokens: std::collections::BTreeSet<String> = tokenize(name).into_iter().collect();
    if name_tokens.is_empty() {
        return 0.0;
    }
    let matched = query_tokens
        .iter()
        .filter(|q| name_tokens.contains(*q))
        .count();
    // proportion of query tokens found in the name, scaled below an exact match
    0.9 * (matched as f64 / query_tokens.len() as f64)
}

/// Path proximity: shared leading path components over the longer path length.
#[must_use]
pub fn path_proximity(seed_paths: &[String], path: &str) -> f64 {
    if seed_paths.is_empty() {
        return 0.0;
    }
    let target: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let mut best = 0.0_f64;
    for seed in seed_paths {
        let sp: Vec<&str> = seed.split('/').filter(|s| !s.is_empty()).collect();
        let shared = sp
            .iter()
            .zip(target.iter())
            .take_while(|(a, b)| a == b)
            .count();
        let denom = sp.len().max(target.len()).max(1);
        best = best.max(shared as f64 / denom as f64);
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_splits_case_and_snake() {
        assert_eq!(tokenize("resolve_head"), vec!["resolve", "head"]);
        assert_eq!(
            tokenize("RepositoryIdentity"),
            vec!["repository", "identity"]
        );
        assert!(!tokenize("HTTPServer2").is_empty());
    }

    #[test]
    fn name_match_exact_beats_partial() {
        let q = tokenize("head");
        assert!((name_match(&q, "head") - 1.0).abs() < 1e-9);
        let partial = name_match(&tokenize("resolve head"), "resolve_head");
        assert!(partial > 0.0 && partial < 1.0);
        assert!(name_match(&tokenize("head"), "budget").abs() < 1e-9);
    }

    #[test]
    fn lexical_overlap_counts_terms() {
        let q = tokenize("mission budget");
        assert!((lexical_overlap(&q, "the mission budget is bounded") - 1.0).abs() < 1e-9);
        assert!((lexical_overlap(&q, "only mission here") - 0.5).abs() < 1e-9);
    }

    #[test]
    fn path_proximity_prefix() {
        let seeds = vec!["crates/kern-git/src/lib.rs".to_string()];
        let close = path_proximity(&seeds, "crates/kern-git/src/profile.rs");
        let far = path_proximity(&seeds, "docs/GAPS.md");
        assert!(close > far);
    }
}
