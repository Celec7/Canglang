use canglang_app::core::position::{Move, Position};

#[test]
fn from_iccs_h2_e2_parses_coordinates_correctly() {
    let mv = Move::from_iccs("h2e2").unwrap();

    // h2：列为 7（'h'-'a'），行为 7（9 - 2）
    assert_eq!(mv.from, Position::new(7, 7));
    // e2：列为 4（'e'-'a'），行为 7（9 - 2）
    assert_eq!(mv.to, Position::new(7, 4));

    assert_eq!(mv.to_iccs(), "h2e2");
}

#[test]
fn from_iccs_black_knight_h9_g7() {
    let mv = Move::from_iccs("h9g7").unwrap();

    // h9：列为 7，行为 0（9 - 9）
    assert_eq!(mv.from, Position::new(0, 7));
    // g7：列为 6，行为 2（9 - 7）
    assert_eq!(mv.to, Position::new(2, 6));

    assert_eq!(mv.to_iccs(), "h9g7");
}

#[test]
fn roundtrip_iccs_matches_expected() {
    let cases = [
        ("a0a9", 9, 0, 0, 0),
        ("i0i9", 9, 8, 0, 8),
        ("b2d4", 7, 1, 5, 3),
    ];

    for (iccs, from_row, from_col, to_row, to_col) in cases {
        let mv = Move::from_iccs(iccs).unwrap();
        assert_eq!(mv.from.row, from_row);
        assert_eq!(mv.from.col, from_col);
        assert_eq!(mv.to.row, to_row);
        assert_eq!(mv.to.col, to_col);
        assert_eq!(mv.to_iccs(), iccs);
    }
}

#[test]
fn from_iccs_invalid_iccs_returns_error() {
    let invalid = ["", "   ", "h2e", "h2e22", "j2e2", "h2j2", "haea", "0a0a"];

    for iccs in invalid {
        assert!(
            Move::from_iccs(iccs).is_err(),
            "should reject invalid ICCS: {iccs}"
        );
    }
}

#[test]
fn position_index_conversion_is_bijective() {
    for idx in 0..90 {
        let pos = Position::from_index(idx);
        assert_eq!(pos.to_index(), idx);
        assert!(pos.is_valid());
    }

    assert!(!Position::new(9, 9).is_valid());
    assert!(!Position::new(10, 0).is_valid());
}
