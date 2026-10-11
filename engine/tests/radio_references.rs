use ranterra_engine::{
    model::RadioKind::{UMa, UMi},
    radio::path_loss,
};
#[test]
fn ns3_347_published_numeric_reference_points() {
    // Numerical reference facts from ns-3.47 SIGNET Lab/University of Padova tests.
    // No ns-3 implementation code is embedded. See docs/radio-validation.md.
    // fc=5 GHz, hUT=1.5 m, hBS=25 m UMa / 10 m UMi, shadowing disabled.
    let rows = [
        (UMa, true, 10., 72.9380),
        (UMa, true, 100., 86.2362),
        (UMa, true, 1000., 109.7252),
        (UMa, true, 5000., 137.6794),
        (UMa, false, 10., 82.5131),
        (UMa, false, 100., 106.1356),
        (UMa, false, 1000., 144.7641),
        (UMa, false, 5000., 172.0753),
        (UMi, true, 10., 69.8591),
        (UMi, true, 100., 88.4122),
        (UMi, true, 1000., 119.3114),
        (UMi, true, 5000., 147.2696),
        (UMi, false, 10., 76.7563),
        (UMi, false, 100., 107.9432),
        (UMi, false, 1000., 143.1886),
        (UMi, false, 5000., 167.8617),
    ];
    for (model, los, distance, expected) in rows {
        let actual = path_loss(model, los, distance, 5.).unwrap();
        assert!(
            (actual - expected).abs() < 0.0001,
            "{model:?} LOS={los} d={distance}: {actual} != {expected}"
        );
    }
}
#[test]
fn breakpoint_continuity_and_extreme_valid_frequencies() {
    for model in [UMi, UMa] {
        let h = if model == UMi { 10.0 } else { 25.0 };
        for frequency in [0.5, 3.5, 100.0] {
            let breakpoint = 4.0 * (h - 1.0) * 0.5 * frequency * 1e9 / 3e8;
            if (10.0..5000.0).contains(&breakpoint) {
                let lo = path_loss(model, true, breakpoint - 1e-5, frequency).unwrap();
                let hi = path_loss(model, true, breakpoint + 1e-5, frequency).unwrap();
                assert!((hi - lo).abs() < 0.0001);
            }
            assert!(path_loss(model, false, 10.0, frequency)
                .unwrap()
                .is_finite());
            assert!(path_loss(model, false, 5000.0, frequency)
                .unwrap()
                .is_finite());
        }
    }
}
