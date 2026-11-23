//! The Level 3 formula of a syntax tree built by hand.
//!
//! Every expected string of this file is the `formula` of an entry of the math
//! fixture `testdata/oracles/maths.json`, which holds what the python
//! implementation writes; the gate of `crates/oracles/tests/math.rs` compares
//! every one of its 1,834 entries. The trees are built here by hand, since
//! `crates/report` does not depend on libsbml and so cannot read a MathML,
//! and each test names the entry its string comes from.

use sbml4humans_model::{Ast, AstKind, AstNumber, MAX_AST_DEPTH};
use sbml4humans_report::math::{MathError, formula};

/// An inner node of `kind` with `children`.
fn node(kind: AstKind, children: Vec<Ast>) -> Ast {
    Ast {
        kind,
        children,
        ..Ast::default()
    }
}

/// A node of `kind` which carries the name libsbml gives it of its own, which no writer reads.
fn named(kind: AstKind, libsbml_name: &str, children: Vec<Ast>) -> Ast {
    Ast {
        kind,
        name: Some(libsbml_name.to_owned()),
        children,
        ..Ast::default()
    }
}

/// A `ci` which names `id`.
fn name(id: &str) -> Ast {
    Ast {
        kind: AstKind::Name,
        name: Some(id.to_owned()),
        ..Ast::default()
    }
}

/// The csymbol time, which libsbml answers the name `time` for.
fn time() -> Ast {
    csymbol(AstKind::NameTime, "time")
}

/// A csymbol of `kind` whose text the document wrote, which may be any text.
fn csymbol(kind: AstKind, text: &str) -> Ast {
    Ast {
        kind,
        name: Some(text.to_owned()),
        ..Ast::default()
    }
}

/// A `cn type="integer"`.
fn integer(value: i64) -> Ast {
    Ast {
        kind: AstKind::Integer,
        number: Some(AstNumber::Integer(value)),
        ..Ast::default()
    }
}

/// A `cn`.
fn real(value: f64) -> Ast {
    Ast {
        kind: AstKind::Real,
        number: Some(AstNumber::Real(value)),
        ..Ast::default()
    }
}

/// A `cn type="e-notation"`.
fn real_e(mantissa: f64, exponent: i64) -> Ast {
    Ast {
        kind: AstKind::RealE,
        number: Some(AstNumber::RealE { mantissa, exponent }),
        ..Ast::default()
    }
}

/// `number` with the `sbml:units` `units`.
fn with_units(number: Ast, units: &str) -> Ast {
    Ast {
        units: Some(units.to_owned()),
        ..number
    }
}

/// The formula of `ast`, which every test of this file expects to be written.
fn written(ast: &Ast) -> String {
    formula(ast).expect("the formula of a tree this test builds is written")
}

// comp1 * (kf_0 * B - kr_0 * BL)
#[test]
fn a_product_of_a_name_and_a_sum_parenthesises_the_sum() {
    let product = |left: &str, right: &str| node(AstKind::Times, vec![name(left), name(right)]);
    let ast = node(
        AstKind::Times,
        vec![
            name("comp1"),
            node(
                AstKind::Minus,
                vec![product("kf_0", "B"), product("kr_0", "BL")],
            ),
        ],
    );
    assert_eq!(written(&ast), "comp1 * (kf_0 * B - kr_0 * BL)");
}

// C * VM1 * (C + Kc)^-1
#[test]
fn a_power_of_a_sum_parenthesises_its_base() {
    let ast = node(
        AstKind::Times,
        vec![
            name("C"),
            name("VM1"),
            node(
                AstKind::Power,
                vec![
                    node(AstKind::Plus, vec![name("C"), name("Kc")]),
                    integer(-1),
                ],
            ),
        ],
    );
    assert_eq!(written(&ast), "C * VM1 * (C + Kc)^-1");
}

// 10 mg + 2 mg * sin(time / 1 hr)
#[test]
fn a_number_with_units_prints_them() {
    let ast = node(
        AstKind::Plus,
        vec![
            with_units(integer(10), "mg"),
            node(
                AstKind::Times,
                vec![
                    with_units(real(2.0), "mg"),
                    node(
                        AstKind::FunctionSin,
                        vec![node(
                            AstKind::Divide,
                            vec![time(), with_units(integer(1), "hr")],
                        )],
                    ),
                ],
            ),
        ],
    );
    assert_eq!(written(&ast), "10 mg + 2 mg * sin(time / 1 hr)");
}

// the same tree, with the name libsbml gives every built-in node of its own on
// it, and with the time csymbol named `t` rather than `time`, which is what
// libsbml answers for a document which writes
//
//     <csymbol definitionURL="http://www.sbml.org/sbml/symbols/time"> t </csymbol>
//
// and which libsbml still writes as `time`. A writer which fell back to the
// name of a node would look right on the names libsbml gives a built-in node of
// its own and would write `t` here, so the name of every node of this tree
// misleads and none of them may make a difference.
#[test]
fn a_built_in_node_is_written_from_its_kind_and_not_from_the_name_of_the_document() {
    let ast = named(
        AstKind::Plus,
        "plus",
        vec![
            with_units(integer(10), "mg"),
            named(
                AstKind::Times,
                "times",
                vec![
                    with_units(real(2.0), "mg"),
                    named(
                        AstKind::FunctionSin,
                        "sin",
                        vec![named(
                            AstKind::Divide,
                            "divide",
                            vec![
                                csymbol(AstKind::NameTime, "t"),
                                with_units(integer(1), "hr"),
                            ],
                        )],
                    ),
                ],
            ),
        ],
    );
    assert_eq!(written(&ast), "10 mg + 2 mg * sin(time / 1 hr)");
    // the three other csymbols a document may name as it likes, read from
    // libsbml for the same definitionURL with the text beside each:
    //
    //   NA  -> avogadro * x        lag -> delay(b, c)        dt -> rateOf(S)
    let avogadro = node(
        AstKind::Times,
        vec![csymbol(AstKind::NameAvogadro, "NA"), name("x")],
    );
    assert_eq!(written(&avogadro), "avogadro * x");
    let delay = named(AstKind::FunctionDelay, "lag", vec![name("b"), name("c")]);
    assert_eq!(written(&delay), "delay(b, c)");
    let rate_of = named(AstKind::FunctionRateOf, "dt", vec![name("S")]);
    assert_eq!(written(&rate_of), "rateOf(S)");
}

// the otherwise branch of piecewise(1.55, C < 3100, 0.2, C > 100000, C * -1.35e-5 + 1.55)
#[test]
fn a_number_in_exponent_form_prints_as_written() {
    let ast = node(
        AstKind::Plus,
        vec![
            node(AstKind::Times, vec![name("C"), real_e(-1.35, -5)]),
            real(1.55),
        ],
    );
    assert_eq!(written(&ast), "C * -1.35e-5 + 1.55");
}

// piecewise(1.55, C < 3100, 0.2, C > 100000, C * -1.35e-5 + 1.55)
#[test]
fn a_piecewise_prints_its_children_in_order() {
    let ast = node(
        AstKind::FunctionPiecewise,
        vec![
            real(1.55),
            node(AstKind::RelationalLt, vec![name("C"), real(3100.0)]),
            real(0.2),
            node(AstKind::RelationalGt, vec![name("C"), real(100000.0)]),
            node(
                AstKind::Plus,
                vec![
                    node(AstKind::Times, vec![name("C"), real_e(-1.35, -5)]),
                    real(1.55),
                ],
            ),
        ],
    );
    assert_eq!(
        written(&ast),
        "piecewise(1.55, C < 3100, 0.2, C > 100000, C * -1.35e-5 + 1.55)"
    );
}

// lambda(x, y, piecewise(x, x > y, y))
#[test]
fn a_lambda_prints_its_variables_and_its_body() {
    let ast = named(
        AstKind::Lambda,
        "lambda",
        vec![
            name("x"),
            name("y"),
            node(
                AstKind::FunctionPiecewise,
                vec![
                    name("x"),
                    node(AstKind::RelationalGt, vec![name("x"), name("y")]),
                    name("y"),
                ],
            ),
        ],
    );
    assert_eq!(written(&ast), "lambda(x, y, piecewise(x, x > y, y))");
}

// (Cdc20 == 0) && (Cdh1 == 0)
#[test]
fn a_relation_within_a_logical_operator_is_parenthesised() {
    let zero = |id: &str| node(AstKind::RelationalEq, vec![name(id), integer(0)]);
    let ast = node(AstKind::LogicalAnd, vec![zero("Cdc20"), zero("Cdh1")]);
    assert_eq!(written(&ast), "(Cdc20 == 0) && (Cdh1 == 0)");
}

// ((CycD == 0) && (CycE == 0) && (CycA == 0) && (CycB == 0)) || ((p27 >= 1) &&
// (!((CycE >= 1) && (CycA >= 1))) && (CycB == 0) && (CycD == 0))
//
// the negation sits in the middle of its conjunction and is wrapped, where the
// one of ((E2F >= 1) && (Rb == 0) && (Cdc20 == 0) && !((Cdh1 >= 1) && (UbcH10
// >= 1))) || ... is the last child of its conjunction and keeps its parentheses:
// the right operand of a logical operator is unambiguous without them.
#[test]
fn a_negation_within_a_conjunction_is_wrapped_unless_it_is_its_last_child() {
    let at_least = |id: &str| node(AstKind::RelationalGeq, vec![name(id), integer(1)]);
    let is_zero = |id: &str| node(AstKind::RelationalEq, vec![name(id), integer(0)]);
    let negation = |left: &str, right: &str| {
        node(
            AstKind::LogicalNot,
            vec![node(
                AstKind::LogicalAnd,
                vec![at_least(left), at_least(right)],
            )],
        )
    };
    let middle = node(
        AstKind::LogicalOr,
        vec![
            node(
                AstKind::LogicalAnd,
                vec![
                    is_zero("CycD"),
                    is_zero("CycE"),
                    is_zero("CycA"),
                    is_zero("CycB"),
                ],
            ),
            node(
                AstKind::LogicalAnd,
                vec![
                    at_least("p27"),
                    negation("CycE", "CycA"),
                    is_zero("CycB"),
                    is_zero("CycD"),
                ],
            ),
        ],
    );
    assert_eq!(
        written(&middle),
        "((CycD == 0) && (CycE == 0) && (CycA == 0) && (CycB == 0)) || ((p27 >= 1) && \
         (!((CycE >= 1) && (CycA >= 1))) && (CycB == 0) && (CycD == 0))"
    );
    let last = node(
        AstKind::LogicalAnd,
        vec![
            at_least("E2F"),
            is_zero("Rb"),
            is_zero("Cdc20"),
            negation("Cdh1", "UbcH10"),
        ],
    );
    assert_eq!(
        written(&last),
        "(E2F >= 1) && (Rb == 0) && (Cdc20 == 0) && !((Cdh1 >= 1) && (UbcH10 >= 1))"
    );
}

// Ri_icg / Mr_icg / (Cve_icg + 1e-12 mM)
#[test]
fn a_quotient_keeps_its_left_quotient_bare_and_wraps_its_right_sum() {
    let ast = node(
        AstKind::Divide,
        vec![
            node(AstKind::Divide, vec![name("Ri_icg"), name("Mr_icg")]),
            node(
                AstKind::Plus,
                vec![name("Cve_icg"), with_units(real_e(1.0, -12), "mM")],
            ),
        ],
    );
    assert_eq!(written(&ast), "Ri_icg / Mr_icg / (Cve_icg + 1e-12 mM)");
}

// max(0 pM, epi - x_epi2)
#[test]
fn a_function_of_a_package_prints_its_arguments_bare() {
    let ast = named(
        AstKind::FunctionMax,
        "max",
        vec![
            with_units(integer(0), "pM"),
            node(AstKind::Minus, vec![name("epi"), name("x_epi2")]),
        ],
    );
    assert_eq!(written(&ast), "max(0 pM, epi - x_epi2)");
}

// lognormal(0, 1, 0, 10)
#[test]
fn a_draw_of_a_distribution_prints_by_its_name() {
    let ast = named(
        AstKind::DistribLognormal,
        "lognormal",
        vec![integer(0), integer(1), integer(0), integer(10)],
    );
    assert_eq!(written(&ast), "lognormal(0, 1, 0, 10)");
}

// f_glyglc * GP_Vmax * GP_fmax * exp((-log10(2 dimensionless) / GPp_ki_glc) * glc)
#[test]
fn a_logarithm_to_base_ten_prints_as_log10_of_its_one_argument() {
    let ast = node(
        AstKind::Times,
        vec![
            name("f_glyglc"),
            name("GP_Vmax"),
            name("GP_fmax"),
            node(
                AstKind::FunctionExp,
                vec![node(
                    AstKind::Times,
                    vec![
                        node(
                            AstKind::Divide,
                            vec![
                                node(
                                    AstKind::Minus,
                                    vec![named(
                                        AstKind::FunctionLog,
                                        "log",
                                        vec![integer(10), with_units(integer(2), "dimensionless")],
                                    )],
                                ),
                                name("GPp_ki_glc"),
                            ],
                        ),
                        name("glc"),
                    ],
                )],
            ),
        ],
    );
    assert_eq!(
        written(&ast),
        "f_glyglc * GP_Vmax * GP_fmax * exp((-log10(2 dimensionless) / GPp_ki_glc) * glc)"
    );
}

// compartment_0000004 * rP * delay(M, parameter_0000009)^m
#[test]
fn a_delay_prints_as_a_function_of_its_value_and_its_time() {
    let ast = node(
        AstKind::Times,
        vec![
            name("compartment_0000004"),
            name("rP"),
            node(
                AstKind::Power,
                vec![
                    named(
                        AstKind::FunctionDelay,
                        "delay",
                        vec![name("M"), name("parameter_0000009")],
                    ),
                    name("m"),
                ],
            ),
        ],
    );
    assert_eq!(
        written(&ast),
        "compartment_0000004 * rP * delay(M, parameter_0000009)^m"
    );
}

// The maths below reach a rule of the writer which no entry of the fixture
// does, so their expected strings are not of the fixture: each is what libsbml
// itself writes, read with
//
//     libsbml.formulaToL3String(libsbml.parseL3Formula("<the formula>"))
//
// of python-libsbml 5.21.2, the version the fixture was harvested with, which
// writes the same string back for every one of them. The formula each was read
// for stands above its assertion, so a reader can run the line again.

#[test]
fn a_modulo_written_as_a_piecewise_prints_with_a_percent() {
    // a % b, which libsbml holds as piecewise(a - b * ceil(a / b), xor(a < 0, b
    // < 0), a - b * floor(a / b)): the math of Level 3 Version 2 has no node for
    // a modulo, and libsbml reads that pattern back as one
    let quotient = || node(AstKind::Divide, vec![name("a"), name("b")]);
    let difference = |rounding: AstKind| {
        node(
            AstKind::Minus,
            vec![
                name("a"),
                node(
                    AstKind::Times,
                    vec![name("b"), node(rounding, vec![quotient()])],
                ),
            ],
        )
    };
    let below_zero = |id: &str| node(AstKind::RelationalLt, vec![name(id), integer(0)]);
    let modulo = || {
        node(
            AstKind::FunctionPiecewise,
            vec![
                difference(AstKind::FunctionCeiling),
                node(AstKind::LogicalXor, vec![below_zero("a"), below_zero("b")]),
                difference(AstKind::FunctionFloor),
            ],
        )
    };
    assert_eq!(written(&modulo()), "a % b");
    // (a % b) + c
    let sum = node(AstKind::Plus, vec![modulo(), name("c")]);
    assert_eq!(written(&sum), "(a % b) + c");
    // piecewise(a - b * ceil(a / b), xor(a < 0, b < 1), a - b * floor(a / b)),
    // which is the pattern with one operand changed and stays a piecewise
    let mut other = modulo();
    other.children[1] = node(
        AstKind::LogicalXor,
        vec![
            below_zero("a"),
            node(AstKind::RelationalLt, vec![name("b"), integer(1)]),
        ],
    );
    assert_eq!(
        written(&other),
        "piecewise(a - b * ceil(a / b), xor((a < 0), (b < 1)), a - b * floor(a / b))"
    );
}

#[test]
fn a_rational_prints_as_a_quotient_within_parentheses() {
    // (3/4) mg
    let rational = |numerator: i64, denominator: i64| Ast {
        kind: AstKind::Rational,
        number: Some(AstNumber::Rational {
            numerator,
            denominator,
        }),
        ..Ast::default()
    };
    assert_eq!(written(&with_units(rational(3, 4), "mg")), "(3/4) mg");
    // (-3/4)
    assert_eq!(written(&rational(-3, 4)), "(-3/4)");
    // a * (3/4)
    let product = node(AstKind::Times, vec![name("a"), rational(3, 4)]);
    assert_eq!(written(&product), "a * (3/4)");
}

#[test]
fn a_root_of_degree_two_prints_as_sqrt_and_every_other_degree_as_root() {
    // sqrt(x)
    let root = |degree: i64| {
        named(
            AstKind::FunctionRoot,
            "root",
            vec![integer(degree), name("x")],
        )
    };
    assert_eq!(written(&root(2)), "sqrt(x)");
    // root(3, x)
    assert_eq!(written(&root(3)), "root(3, x)");
}

#[test]
fn a_node_of_the_wrong_number_of_children_prints_as_a_call_of_its_name() {
    // plus(a), times(a), minus(a, b, c), and(a), f(x, y)
    assert_eq!(written(&node(AstKind::Plus, vec![name("a")])), "plus(a)");
    assert_eq!(written(&node(AstKind::Times, vec![name("a")])), "times(a)");
    assert_eq!(
        written(&node(AstKind::Minus, vec![name("a"), name("b"), name("c")])),
        "minus(a, b, c)"
    );
    assert_eq!(
        written(&named(AstKind::LogicalAnd, "and", vec![name("a")])),
        "and(a)"
    );
    // a call of a function of the model is the one node a formula names by the
    // name of the document, which the fixture holds none of
    assert_eq!(
        written(&Ast {
            kind: AstKind::Function,
            name: Some("f".to_owned()),
            children: vec![name("x"), name("y")],
            ..Ast::default()
        }),
        "f(x, y)"
    );
}

#[test]
fn an_exclusive_or_and_an_implication_print_as_calls_of_wrapped_operands() {
    // xor((a < 0), b)
    let exclusive = named(
        AstKind::LogicalXor,
        "xor",
        vec![
            node(AstKind::RelationalLt, vec![name("a"), integer(0)]),
            name("b"),
        ],
    );
    assert_eq!(written(&exclusive), "xor((a < 0), b)");
    // implies((a && b), c)
    let implication = named(
        AstKind::LogicalImplies,
        "implies",
        vec![
            node(AstKind::LogicalAnd, vec![name("a"), name("b")]),
            name("c"),
        ],
    );
    assert_eq!(written(&implication), "implies((a && b), c)");
}

#[test]
fn a_unary_operand_of_a_power_is_wrapped_on_its_left_and_bare_on_its_right() {
    let negative = |id: &str| node(AstKind::Minus, vec![name(id)]);
    // (-a)^b
    let base = node(AstKind::Power, vec![negative("a"), name("b")]);
    assert_eq!(written(&base), "(-a)^b");
    // a^-b
    let exponent = node(AstKind::Power, vec![name("a"), negative("b")]);
    assert_eq!(written(&exponent), "a^-b");
    // --a: a nested negative sign is not collapsed, which is the default of the
    // Level 3 parser settings python writes with
    assert_eq!(written(&node(AstKind::Minus, vec![negative("a")])), "--a");
    // -(!a) and !(-a) are always wrapped
    let not = node(AstKind::LogicalNot, vec![name("a")]);
    assert_eq!(written(&node(AstKind::Minus, vec![not])), "-(!a)");
    assert_eq!(
        written(&node(AstKind::LogicalNot, vec![negative("a")])),
        "!(-a)"
    );
    // a^(b^c) and a^b^c: a power associates to the left, so the right operand of
    // one is wrapped and the left is not
    let power = |left: Ast, right: Ast| node(AstKind::Power, vec![left, right]);
    assert_eq!(
        written(&power(name("a"), power(name("b"), name("c")))),
        "a^(b^c)"
    );
    assert_eq!(
        written(&power(power(name("a"), name("b")), name("c"))),
        "a^b^c"
    );
}

#[test]
fn a_relation_of_three_operands_prints_as_a_chain() {
    // a == b == c
    let ast = node(AstKind::RelationalEq, vec![name("a"), name("b"), name("c")]);
    assert_eq!(written(&ast), "a == b == c");
}

#[test]
fn a_number_prints_the_fifteen_significant_digits_of_the_float_format_of_libsbml() {
    // 1.5e-07, 1.5e+21, 0.333333333333333, 1.23456789012346e+15, 123456789012345
    assert_eq!(written(&real(1.5e-7)), "1.5e-07");
    assert_eq!(written(&real(1.5e21)), "1.5e+21");
    assert_eq!(written(&real(1.0 / 3.0)), "0.333333333333333");
    assert_eq!(written(&real(1234567890123456.0)), "1.23456789012346e+15");
    assert_eq!(written(&real(123456789012345.0)), "123456789012345");
    // -0: libsbml reads the high byte of the double and answers that every
    // negative value whose seven high exponent bits are zero is a negative zero
    assert_eq!(written(&real(-0.0)), "-0");
    assert_eq!(written(&real(-2.38558290553944e-305)), "-0");
    assert_eq!(written(&real(-2.5e-300)), "-2.5e-300");
    // INF, -INF, NaN: the three branches libsbml takes before it formats a
    // value at all, read with formulaToL3String(parseL3Formula("INF")) and so on
    assert_eq!(written(&real(f64::INFINITY)), "INF");
    assert_eq!(written(&real(f64::NEG_INFINITY)), "-INF");
    assert_eq!(written(&real(f64::NAN)), "NaN");
    // a * INF and a + NaN: they are written the same below an operator
    assert_eq!(
        written(&node(AstKind::Times, vec![name("a"), real(f64::INFINITY)])),
        "a * INF"
    );
    assert_eq!(
        written(&node(AstKind::Plus, vec![name("a"), real(f64::NAN)])),
        "a + NaN"
    );
}

#[test]
fn a_number_in_exponent_form_keeps_its_form_only_while_its_mantissa_reads_as_itself() {
    // 99999999999999e3 and 1e+17: a mantissa of 1e14 or more is written as the
    // value of the node instead
    assert_eq!(written(&real_e(99999999999999.0, 3)), "99999999999999e3");
    assert_eq!(written(&real_e(1e14, 3)), "1e+17");
    // 0.0001e3 and 0.099: so is a mantissa below a ten thousandth
    assert_eq!(written(&real_e(1e-4, 3)), "0.0001e3");
    assert_eq!(written(&real_e(9.9e-5, 3)), "0.099");
    // 0 and 2e0
    assert_eq!(written(&real_e(0.0, 3)), "0");
    assert_eq!(written(&real_e(2.0, 0)), "2e0");
    // 1.5e1294967296: libsbml writes the exponent with the "%d" of C, which
    // reads the low 32 bits of the long it is given
    assert_eq!(written(&real_e(1.5, -3_000_000_000)), "1.5e1294967296");
}

#[test]
fn a_tree_deeper_than_the_bound_is_an_error() {
    let mut ast = name("x");
    for _ in 1..MAX_AST_DEPTH {
        ast = node(AstKind::FunctionAbs, vec![ast]);
    }
    assert!(formula(&ast).is_ok(), "a tree of the bound is written");
    let deeper = node(AstKind::FunctionAbs, vec![ast]);
    assert_eq!(formula(&deeper), Err(MathError::TooDeep));
}

#[test]
fn a_kind_the_shim_does_not_name_is_an_error() {
    let ast = node(
        AstKind::Times,
        vec![name("x"), node(AstKind::Other, vec![])],
    );
    assert_eq!(
        formula(&ast),
        Err(MathError::UnknownKind(AstKind::Other)),
        "a writer which wrote a node of an unmapped kind would write a formula which is not the one \
         of the document"
    );
}
