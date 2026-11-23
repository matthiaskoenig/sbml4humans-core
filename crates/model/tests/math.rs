//! The syntax tree of the math of an element.
use sbml4humans_model::{Ast, AstKind, AstNumber, MAX_AST_DEPTH};

#[test]
fn a_node_carries_its_kind_its_payload_and_its_children() {
    let ast = Ast {
        kind: AstKind::Times,
        children: vec![
            Ast {
                kind: AstKind::Name,
                name: Some("k1".to_string()),
                ..Default::default()
            },
            Ast {
                kind: AstKind::Integer,
                number: Some(AstNumber::Integer(2)),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    assert_eq!(ast.children.len(), 2);
    assert_eq!(ast.children[1].number, Some(AstNumber::Integer(2)));
}

#[test]
fn every_kind_round_trips_through_its_code_and_an_unknown_one_is_other() {
    assert_eq!(AstKind::Other as i32, 0);
    for kind in AstKind::ALL {
        assert_eq!(AstKind::from_code(kind as i32), kind, "{kind:?}");
    }
    for code in [-7, 0, 9999] {
        assert_eq!(AstKind::from_code(code), AstKind::Other);
    }
}

#[test]
fn all_holds_every_kind_exactly_once() {
    assert_eq!(AstKind::ALL.len(), 81);
    for (i, a) in AstKind::ALL.iter().enumerate() {
        for b in AstKind::ALL.iter().skip(i + 1) {
            assert_ne!(a, b);
            assert_ne!(*a as i32, *b as i32);
        }
    }
}

#[test]
fn the_default_is_other_and_the_depth_bound_is_one_thousand() {
    assert_eq!(Ast::default().kind, AstKind::Other);
    assert_eq!(MAX_AST_DEPTH, 1000);
}
