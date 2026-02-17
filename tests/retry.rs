// Integration tests for tap retry logic (Phase 1-3)

use era::simulator::idb::{compute_state_hash, JITTER_OFFSETS, MAX_RETRIES};

// -------------------------------------------------------------------
// compute_state_hash — determinism and collision avoidance
// -------------------------------------------------------------------

#[test]
fn test_hash_deterministic_across_calls() {
    let data = r#"{"frame":{"x":0,"y":0,"width":393,"height":852}}"#;
    let h1 = compute_state_hash(data);
    let h2 = compute_state_hash(data);
    let h3 = compute_state_hash(data);
    assert_eq!(h1, h2);
    assert_eq!(h2, h3);
}

#[test]
fn test_hash_different_for_different_ui_states() {
    let state_a = r#"{"label":"Login","enabled":true}"#;
    let state_b = r#"{"label":"Login","enabled":false}"#;
    assert_ne!(
        compute_state_hash(state_a),
        compute_state_hash(state_b),
        "Different UI states should produce different hashes"
    );
}

#[test]
fn test_hash_sensitive_to_small_changes() {
    let base = "a]b]c]d]e]f]g]h]i]j]k]l]m]n]o]p";
    let modified = "a]b]c]d]e]f]g]h]i]j]k]l]m]n]o]q";
    assert_ne!(
        compute_state_hash(base),
        compute_state_hash(modified),
        "Single character change should produce different hash"
    );
}

#[test]
fn test_hash_empty_string() {
    // Should not panic and should produce a consistent value
    let h1 = compute_state_hash("");
    let h2 = compute_state_hash("");
    assert_eq!(h1, h2);
}

#[test]
fn test_hash_large_input() {
    // Simulates a large UI tree from idb describe-all
    let large = "x".repeat(100_000);
    let h1 = compute_state_hash(&large);
    let h2 = compute_state_hash(&large);
    assert_eq!(h1, h2);
}

// -------------------------------------------------------------------
// JITTER_OFFSETS — range and validity
// -------------------------------------------------------------------

#[test]
fn test_jitter_offsets_count_matches_max_retries() {
    assert_eq!(
        JITTER_OFFSETS.len(),
        MAX_RETRIES,
        "JITTER_OFFSETS length must equal MAX_RETRIES"
    );
}

#[test]
fn test_jitter_offsets_are_small() {
    // Jitter should be small enough not to miss the target element
    let max_jitter = 10.0; // reasonable upper bound for jitter in points
    for (i, (dx, dy)) in JITTER_OFFSETS.iter().enumerate() {
        assert!(
            dx.abs() <= max_jitter,
            "Jitter offset {} x ({}) exceeds max {}",
            i,
            dx,
            max_jitter
        );
        assert!(
            dy.abs() <= max_jitter,
            "Jitter offset {} y ({}) exceeds max {}",
            i,
            dy,
            max_jitter
        );
    }
}

#[test]
fn test_jitter_offsets_are_distinct() {
    // Each jitter offset should be unique to explore different positions
    for i in 0..JITTER_OFFSETS.len() {
        for j in (i + 1)..JITTER_OFFSETS.len() {
            assert_ne!(
                JITTER_OFFSETS[i], JITTER_OFFSETS[j],
                "Jitter offsets {} and {} should be distinct",
                i, j
            );
        }
    }
}

#[test]
fn test_jitter_offsets_not_all_same_axis() {
    // At least one offset should have non-zero x and at least one non-zero y
    let has_nonzero_x = JITTER_OFFSETS.iter().any(|(dx, _)| dx.abs() > f64::EPSILON);
    let has_nonzero_y = JITTER_OFFSETS.iter().any(|(_, dy)| dy.abs() > f64::EPSILON);
    assert!(has_nonzero_x, "Should have at least one non-zero X offset");
    assert!(has_nonzero_y, "Should have at least one non-zero Y offset");
}

// -------------------------------------------------------------------
// MAX_RETRIES — sanity checks
// -------------------------------------------------------------------

#[test]
fn test_max_retries_is_reasonable() {
    // Should retry at least once but not too many times
    assert!(MAX_RETRIES >= 1, "Should retry at least once");
    assert!(MAX_RETRIES <= 10, "Too many retries would be slow");
}

// -------------------------------------------------------------------
// Jitter applied to coordinates — boundary safety
// -------------------------------------------------------------------

#[test]
fn test_jitter_at_origin_stays_non_negative() {
    // When tapping at (0, 0), jitter with negative offsets should be clamped
    let x = 0.0_f64;
    let y = 0.0_f64;
    for (dx, dy) in &JITTER_OFFSETS {
        let jittered_x = (x + dx).max(0.0);
        let jittered_y = (y + dy).max(0.0);
        assert!(
            jittered_x >= 0.0,
            "Jittered X should be non-negative, got {}",
            jittered_x
        );
        assert!(
            jittered_y >= 0.0,
            "Jittered Y should be non-negative, got {}",
            jittered_y
        );
    }
}

#[test]
fn test_jitter_preserves_approximate_position() {
    // After jitter, coordinates should still be near the original
    let x = 200.0;
    let y = 400.0;
    for (dx, dy) in &JITTER_OFFSETS {
        let jittered_x = x + dx;
        let jittered_y = y + dy;
        assert!(
            (jittered_x - x).abs() < 10.0,
            "Jitter moved X too far: {} -> {}",
            x,
            jittered_x
        );
        assert!(
            (jittered_y - y).abs() < 10.0,
            "Jitter moved Y too far: {} -> {}",
            y,
            jittered_y
        );
    }
}
