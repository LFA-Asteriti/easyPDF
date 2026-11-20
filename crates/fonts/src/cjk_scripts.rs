//! The rule `build.rs` follows for CJK faces: which manifest script tags are CJK, which families
//! are embedded whole, and the size cap for the others. The build script includes this file with
//! `#[path]` and the crate's tests use it as a module, so the two can't drift.

/// The CJK families embedded whole in native builds.
pub const WHOLE_CJK_FAMILIES: [&str; 5] = ["Shippori Mincho", "BIZ UDPGothic", "BIZ UDMincho", "Noto Sans CJK SC", "LXGW WenKai Lite"];

/// Adobe-origin type designs forbidden by AGENTS.md, regardless of size or licence.
pub const BARRED_FAMILIES: [&str; 4] = ["Source Han", "Source Serif", "Source Sans", "Noto Serif CJK"];

/// A prefix also covers region and style suffixes in the optional font manifest.
pub fn is_barred_family(family: &str) -> bool {
    BARRED_FAMILIES.iter().any(|prefix| family.starts_with(prefix))
}

/// Any other CJK face over this size is left out.
pub const MAX_OTHER_CJK_BYTES: u64 = 4 << 20;

/// ISO 15924 codes of the Han, Japanese and Korean scripts, Hangul on its own included.
const CJK_SCRIPTS: [&str; 6] = ["Hani", "Hans", "Hant", "Jpan", "Kore", "Hang"];

/// Whether an easyPDF font manifest script tag is CJK. A tag can carry a region after the code,
/// so only the part before `-` is compared.
pub fn is_cjk_script(tag: &str) -> bool {
    let code = tag.trim().split('-').next().unwrap_or_default();
    CJK_SCRIPTS.contains(&code)
}

/// What `build.rs` does with one CJK face from a manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CjkSizeDecision {
    /// The face fits its cap (or is a Japanese family embedded whole).
    Embed,
    /// An optional face over its cap: leave it out and keep the rest.
    Skip,
}

/// The size cap that applies to `family` in this build.
pub fn cjk_cap_bytes(_family: &str, _wasm: bool) -> u64 {
    MAX_OTHER_CJK_BYTES
}

/// Decide whether a CJK face is embedded, skipped, or rejected.
///
/// The approved document/UI families are embedded whole. Every other CJK family uses its cap.
pub fn cjk_size_decision(family: &str, bytes: u64, wasm: bool) -> CjkSizeDecision {
    if is_barred_family(family) {
        return CjkSizeDecision::Skip;
    }
    if WHOLE_CJK_FAMILIES.contains(&family) {
        return CjkSizeDecision::Embed;
    }
    if bytes <= cjk_cap_bytes(family, wasm) { CjkSizeDecision::Embed } else { CjkSizeDecision::Skip }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forbidden_families_never_pass_the_font_size_gate() {
        for family in ["Source Han Sans", "Source Serif 4", "Source Sans 3", "Noto Serif CJK JP"] {
            assert_eq!(cjk_size_decision(family, 1024, false), CjkSizeDecision::Skip, "{family}");
        }
        assert!(!is_barred_family("BIZ UDPGothic"));
        assert!(!is_barred_family("Noto Sans CJK SC"));
    }

    /// Approved whole CJK families bypass the general size cap.
    #[test]
    fn approved_cjk_families_are_embedded_whole() {
        assert_eq!(cjk_size_decision("BIZ UDPGothic", u64::MAX, false), CjkSizeDecision::Embed);
        assert_eq!(cjk_size_decision("Noto Sans CJK SC", u64::MAX, false), CjkSizeDecision::Embed);
        assert_eq!(cjk_size_decision("LXGW WenKai Lite", u64::MAX, true), CjkSizeDecision::Embed);
        assert_eq!(cjk_size_decision("Other CJK", MAX_OTHER_CJK_BYTES + 1, false), CjkSizeDecision::Skip);
    }
}
