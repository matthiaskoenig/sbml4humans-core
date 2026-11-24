//! The syntax tree of the math of an element.
use sbml4humans_model::{Ast, AstKind, AstNumber, MAX_AST_DEPTH};

#[test]
fn a_node_carries_its_kind_its_payload_and_its_children() {
    let ast = Ast {
        kind: AstKind::Times,
        name: None,
        number: None,
        units: None,
        children: vec![
            Ast {
                kind: AstKind::Name,
                name: Some("k1".to_string()),
                number: None,
                units: None,
                children: Vec::new(),
            },
            Ast {
                kind: AstKind::Integer,
                name: None,
                number: Some(AstNumber::Integer(2)),
                units: None,
                children: Vec::new(),
            },
        ],
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
    assert_eq!(AstKind::ALL.len(), 84);
    for (i, a) in AstKind::ALL.iter().enumerate() {
        for b in AstKind::ALL.iter().skip(i + 1) {
            assert_ne!(a, b);
            assert_ne!(*a as i32, *b as i32);
        }
    }
}

#[test]
fn the_default_is_other_and_the_depth_bound_is_two_hundred_thousand() {
    assert_eq!(Ast::default().kind, AstKind::Other);
    assert_eq!(MAX_AST_DEPTH, 200_000);
}

/// A tree of this many levels, which is far past [`MAX_AST_DEPTH`] whatever the
/// bound is: the walks of this module cost no call frame per level, so what
/// they are tried on is a figure of its own and not one of the bound.
const FAR_PAST_THE_BOUND: usize = 2_000_000;

/// A chain of `levels` nodes, the one at the bottom a name.
///
/// The chain is built from the bottom up, so building it costs no call frame
/// per level either.
fn chain(levels: usize) -> Ast {
    let mut node = Ast {
        kind: AstKind::Name,
        name: Some("x".to_string()),
        number: None,
        units: None,
        children: Vec::new(),
    };
    for _ in 1..levels {
        node = Ast {
            kind: AstKind::FunctionAbs,
            name: None,
            number: None,
            units: None,
            children: vec![node],
        };
    }
    node
}

/// The depth of a tree, read without a call frame per level.
fn depth_of(ast: &Ast) -> usize {
    let mut deepest = 0;
    let mut stack = vec![(ast, 1usize)];
    while let Some((node, depth)) = stack.pop() {
        deepest = deepest.max(depth);
        stack.extend(node.children.iter().map(|child| (child, depth + 1)));
    }
    deepest
}

/// A sink which counts the bytes written to it and keeps none of them.
struct Counter(usize);

impl std::fmt::Write for Counter {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.0 += text.len();
        Ok(())
    }
}

#[test]
fn a_tree_far_deeper_than_the_bound_is_dropped_without_overflowing_the_stack() {
    // The drop glue the compiler writes for a `Vec<Ast>` field follows the
    // children on the call stack, one frame per level, and a destructor is the
    // one place where nothing can catch a stack which runs out: the process
    // aborts, and a wasm instance traps. `Drop` of `Ast` is therefore written
    // and not derived, and this is what it is written for: twice the bound,
    // dropped on the 2 MiB stack a test thread gets, where the glue of a derive
    // overflows after a few thousand levels.
    let deep = chain(FAR_PAST_THE_BOUND);
    assert_eq!(depth_of(&deep), FAR_PAST_THE_BOUND);
    drop(deep);
}

#[test]
fn a_deep_tree_is_cloned_compared_and_written_without_overflowing_the_stack() {
    // `Clone`, `PartialEq` and `Debug` are written for the same reason `Drop`
    // is: each of the three walks the children, and a derive walks them on the
    // call stack.
    let levels = FAR_PAST_THE_BOUND / 2;
    let deep = chain(levels);
    let copy = deep.clone();
    assert_eq!(depth_of(&copy), levels);
    assert!(copy == deep, "a clone equals its original");
    let mut shallower = chain(levels - 1);
    assert!(copy != shallower, "and a chain of one level less does not");
    // the text is the one a derive writes, which is pinned on a small tree by
    // `the_debug_text_of_a_tree_is_the_one_a_derive_writes`
    let mut counter = Counter(0);
    std::fmt::write(&mut counter, format_args!("{deep:?}")).expect("the tree is written");
    assert!(counter.0 > levels, "every level is written");
    // the comparison of two chains which differ at the very bottom walks both
    // of them whole
    shallower = chain(levels);
    assert!(copy == shallower, "two chains of the same shape are equal");
}

#[test]
fn the_debug_text_of_a_tree_is_the_one_a_derive_writes() {
    let ast = Ast {
        kind: AstKind::Times,
        name: None,
        number: None,
        units: None,
        children: vec![
            Ast {
                kind: AstKind::Name,
                name: Some("k1".to_string()),
                number: None,
                units: None,
                children: Vec::new(),
            },
            Ast {
                kind: AstKind::Integer,
                name: None,
                number: Some(AstNumber::Integer(2)),
                units: None,
                children: Vec::new(),
            },
        ],
    };
    let expected = concat!(
        "Ast { kind: Times, name: None, number: None, units: None, children: [",
        "Ast { kind: Name, name: Some(\"k1\"), number: None, units: None, children: [] }, ",
        "Ast { kind: Integer, name: None, number: Some(Integer(2)), units: None, children: [] }",
        "] }"
    );
    assert_eq!(format!("{ast:?}"), expected);
    // the alternate form writes that same text, which the walk of one stack is
    // what it costs
    assert_eq!(format!("{ast:#?}"), expected);
}
