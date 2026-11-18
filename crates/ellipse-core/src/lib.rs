//! Ellipse proprietary application core.
//! This crate is intentionally platform-independent and has no web or OS dependencies.

pub mod artwork;
pub mod home;
pub mod notes;
/// Our functional features are never enabled based on a client-side boolean alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessLevel {
    Free,
    Pro,
    Lifetime,
}

/// Stable feature identifiers; the backend is the authority for paid entitlements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    Notes,
    LocalFiles,
    CustomThemes,
    AdvancedLayouts,
    PremiumPlayers,
}

/// A preview-only gate used for showing unavailable features in the UI.
/// Real paid access must be authorized by a verified, unexpired server entitlement.
pub fn feature_available_in_preview(feature: Feature, level: AccessLevel) -> bool {
    match feature {
        Feature::Notes | Feature::LocalFiles => true,
        Feature::CustomThemes | Feature::AdvancedLayouts | Feature::PremiumPlayers => {
            matches!(level, AccessLevel::Pro | AccessLevel::Lifetime)
        }
    }
}

/// Bound note sizes to protect low-memory devices from accidental oversized entries.
pub const MAX_NOTE_BYTES: usize = 256 * 1024;

pub fn valid_note_bytes(note: &str) -> bool {
    note.len() <= MAX_NOTE_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_access_retains_essential_offline_notes() {
        assert!(feature_available_in_preview(
            Feature::Notes,
            AccessLevel::Free
        ));
        assert!(!feature_available_in_preview(
            Feature::PremiumPlayers,
            AccessLevel::Free
        ));
    }

    #[test]
    fn paid_access_is_only_a_preview_hint() {
        assert!(feature_available_in_preview(
            Feature::PremiumPlayers,
            AccessLevel::Pro
        ));
        assert!(feature_available_in_preview(
            Feature::AdvancedLayouts,
            AccessLevel::Lifetime
        ));
    }

    #[test]
    fn note_size_is_bounded() {
        assert!(valid_note_bytes("small note"));
        assert!(!valid_note_bytes(&"x".repeat(MAX_NOTE_BYTES + 1)));
    }
}
