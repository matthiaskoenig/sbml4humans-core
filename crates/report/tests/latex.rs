//! The latex of a syntax tree built by hand.
//!
//! Every expected string of this file is what the python implementation writes:
//! the ones which name an entry come from the math fixture
//! `testdata/oracles/maths.json`, whose 1,834 entries the gate of
//! `crates/oracles/tests/math.rs` compares, and the others are what
//! `sbml4humans.mathml.astnode_to_latex` answered for the tree built here, for
//! constructs the corpus holds none of. The trees are built by hand, since
//! `crates/report` does not depend on libsbml and so cannot read a MathML.

use sbml4humans_model::{Ast, AstKind, AstNumber, MAX_AST_DEPTH};
use sbml4humans_report::math::{MathError, latex, math};

/// An inner node of `kind` with `children`.
fn node(kind: AstKind, children: Vec<Ast>) -> Ast {
    Ast {
        kind,
        children,
        ..Ast::default()
    }
}

/// A node of `kind` which carries the name libsbml gives it of its own.
///
/// The latex of such a node is written by its kind: libsbml names a node of the
/// kind [`AstKind::FunctionAbs`] `abs` and one of the kind
/// [`AstKind::ConstantPi`] `pi`, which is no part of any latex.
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

/// A `cn` which carries the units of the document, which no latex renders.
fn integer_with_units(value: i64, units: &str) -> Ast {
    Ast {
        units: Some(units.to_owned()),
        ..integer(value)
    }
}

/// A `cn`, which libsbml writes with a `sep` where its `"%.15g"` carries an exponent.
fn real(value: f64) -> Ast {
    Ast {
        kind: AstKind::Real,
        number: Some(AstNumber::Real(value)),
        ..Ast::default()
    }
}

/// A `cn type="rational"`.
fn rational(numerator: i64, denominator: i64) -> Ast {
    Ast {
        kind: AstKind::Rational,
        number: Some(AstNumber::Rational {
            numerator,
            denominator,
        }),
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

/// The latex of `ast`, which no tree of these tests fails to write.
fn written(ast: &Ast) -> String {
    latex(ast).expect("the tree is written")
}

#[test]
fn a_name_with_one_underscore_becomes_a_subscript() {
    // the `kf_0` of the entry `comp1 * (kf_0 * B - kr_0 * BL)` of the fixture,
    // which is a sub-expression of that entry and no entry of its own
    assert_eq!(written(&name("kf_0")), " \\mathit{kf_{0}}");
}

#[test]
fn a_name_with_two_underscores_keeps_them_escaped() {
    // the entry `LI__flux_P_in` of the fixture: the heuristic reads a subscript
    // out of one underscore between two alphanumeric runs and of nothing else
    assert_eq!(
        written(&name("LI__flux_P_in")),
        " \\mathit{LI\\_\\_flux\\_P\\_in}"
    );
}

#[test]
fn a_name_which_is_a_greek_word_becomes_its_command() {
    // the entry `kappa` of the fixture
    assert_eq!(written(&name("kappa")), " \\mathit{\\kappa}");
}

#[test]
fn a_product_joins_with_cdot_and_parenthesises_a_sum() {
    // the entry `comp1 * (kf_0 * B - kr_0 * BL)` of the fixture
    let difference = node(
        AstKind::Minus,
        vec![
            node(AstKind::Times, vec![name("kf_0"), name("B")]),
            node(AstKind::Times, vec![name("kr_0"), name("BL")]),
        ],
    );
    let ast = node(AstKind::Times, vec![name("comp1"), difference]);
    assert_eq!(
        written(&ast),
        " \\mathit{comp1}\\cdot (\\mathit{kf_{0}}\\cdot \\mathit{B}-\\mathit{kr_{0}}\\cdot \\mathit{BL})"
    );
}

#[test]
fn a_power_wraps_a_quotient_base_in_left_right() {
    // the base of the entry `cell * pM * (k4prime + k4 * (M / CT)^2)` of the
    // fixture: a parenthesis which holds no operator of its own is stretched
    let quotient = node(AstKind::Divide, vec![name("M"), name("CT")]);
    let ast = node(AstKind::Power, vec![quotient, integer(2)]);
    assert_eq!(
        written(&ast),
        " {\\left(\\frac{\\mathit{M}}{\\mathit{CT}}\\right)}^{2}"
    );
}

#[test]
fn a_piecewise_is_a_cases_environment() {
    // the piecewise of the entry `lambda(x, y, piecewise(x, x > y, y))` of the fixture
    let ast = named(
        AstKind::FunctionPiecewise,
        "piecewise",
        vec![
            name("x"),
            named(AstKind::RelationalGt, "gt", vec![name("x"), name("y")]),
            name("y"),
        ],
    );
    assert_eq!(
        written(&ast),
        " \\begin{cases} \\mathit{x}& \\text{\u{a0} if \u{a0}} & \\mathit{x}>\\mathit{y}\
         \\\\ \\mathit{y}& {\\text{\u{a0} otherwise}}\\end{cases}"
    );
}

#[test]
fn a_lambda_names_its_variables_before_the_equals_sign() {
    // the entry `lambda(x, y, piecewise(x, x > y, y))` of the fixture: the three
    // replacements of the lambda turn `\lambda \mathit{x}\mathit{y}.` into
    // `\lambda(\mathit{x}, \mathit{y}) =`
    let piecewise = named(
        AstKind::FunctionPiecewise,
        "piecewise",
        vec![
            name("x"),
            named(AstKind::RelationalGt, "gt", vec![name("x"), name("y")]),
            name("y"),
        ],
    );
    let ast = named(
        AstKind::Lambda,
        "lambda",
        vec![name("x"), name("y"), piecewise],
    );
    assert_eq!(
        written(&ast),
        " \\lambda(\\mathit{x}, \\mathit{y}) =\\begin{cases} \\mathit{x}& \\text{\u{a0} if \u{a0}} \
         & \\mathit{x}>\\mathit{y}\\\\ \\mathit{y}& {\\text{\u{a0} otherwise}}\\end{cases}"
    );
}

#[test]
fn a_csymbol_is_typeset_by_the_text_the_document_gave_it() {
    // every document of the corpus spells the time csymbol `time`, which is the
    // entry `time > 2` of the fixture; libsbml answers the text of the document
    // as the name of the node, and the latex comes from that text and not from
    // the kind, so a document which writes `t` is typeset `t`. python answers
    // ` t` and ` \mathrm{time}` for the two.
    assert_eq!(
        written(&csymbol(AstKind::NameTime, "time")),
        " \\mathrm{time}"
    );
    assert_eq!(written(&csymbol(AstKind::NameTime, "t")), " t");
    assert_eq!(
        written(&csymbol(AstKind::NameAvogadro, "NA")),
        " \\mathrm{NA}"
    );
}

#[test]
fn a_function_of_a_plain_name_prints_without_parentheses_and_the_comma_fires() {
    // the corpus holds no such call. The stylesheets write `\mathrm{sin}` and
    // its argument with nothing between, and the global replacement of the
    // lambda turns the `}\mathit` of that into `}, \mathit`: python answers
    // ` \mathrm{sin}, \mathit{x}`, which a writer with a local rule for the
    // lambda would never produce.
    let ast = named(AstKind::FunctionSin, "sin", vec![name("x")]);
    assert_eq!(written(&ast), " \\mathrm{sin}, \\mathit{x}");
    // an argument which is an application is parenthesised and the replacement
    // does not fire
    let product = node(AstKind::Times, vec![name("x"), name("y")]);
    let call = named(AstKind::FunctionSin, "sin", vec![product]);
    assert_eq!(
        written(&call),
        " \\mathrm{sin}\\left(\\mathit{x}\\cdot \\mathit{y}\\right)"
    );
}

#[test]
fn an_exponential_is_a_power_of_e_and_a_logarithm_of_one_token_has_no_parentheses() {
    // the entries `2 m3 * exp(time / 1 second)` and `ln(2) / tau_mRNA` of the fixture
    let quotient = node(
        AstKind::Divide,
        vec![
            csymbol(AstKind::NameTime, "time"),
            integer_with_units(1, "second"),
        ],
    );
    let exponential = named(AstKind::FunctionExp, "exp", vec![quotient]);
    let ast = node(
        AstKind::Times,
        vec![integer_with_units(2, "m3"), exponential],
    );
    assert_eq!(written(&ast), " 2\\cdot {e}^{\\frac{\\mathrm{time}}{1}}");

    let logarithm = named(AstKind::FunctionLn, "ln", vec![integer(2)]);
    let ast = node(AstKind::Divide, vec![logarithm, name("tau_mRNA")]);
    assert_eq!(
        written(&ast),
        " \\frac{\\mathrm{ln}2}{\\mathit{tau_{mRNA}}}"
    );
}

#[test]
fn a_call_of_a_function_of_the_model_is_italic_and_a_csymbol_upright() {
    // the entry `max(0 pM, epi - x_epi2)` of the fixture, whose `max` is a
    // function of the model and no maximum of the extended math
    let ast = named(
        AstKind::Function,
        "max",
        vec![
            integer_with_units(0, "pM"),
            node(AstKind::Minus, vec![name("epi"), name("x_epi2")]),
        ],
    );
    assert_eq!(
        written(&ast),
        " \\mathit{max}\\left(0,\\mathit{epi}-\\mathit{x_{epi2}}\\right)"
    );
    // python answers ` \mathrm{rateOf}\left(\mathit{a}\right)` for the csymbol
    let rate = named(AstKind::FunctionRateOf, "rateOf", vec![name("a")]);
    assert_eq!(
        written(&rate),
        " \\mathrm{rateOf}\\left(\\mathit{a}\\right)"
    );
}

#[test]
fn a_number_in_exponent_form_is_a_product_with_a_power_of_ten() {
    // the `1e-6` of the entry which spells out the pattern of a liver model
    assert_eq!(written(&real_e(1.0, -6)), " 1\\cdot {10}^{-6}");
}

#[test]
fn the_right_operand_of_a_difference_is_parenthesised_by_its_text() {
    // the templates tell the two operands of a difference apart by the string
    // value of their mathml, which is the text of everything below them: two
    // operands which differ in their operator alone have the same string value,
    // and the right one of them is written without the parentheses it needs.
    // python answers ` \mathit{a}\cdot \mathit{b}-\mathit{a}+\mathit{b}` for the
    // first of these and ` \mathit{a}-(\mathit{b}+\mathit{c})` for the second.
    let product = node(AstKind::Times, vec![name("a"), name("b")]);
    let sum = node(AstKind::Plus, vec![name("a"), name("b")]);
    let ast = node(AstKind::Minus, vec![product, sum]);
    assert_eq!(
        written(&ast),
        " \\mathit{a}\\cdot \\mathit{b}-\\mathit{a}+\\mathit{b}"
    );

    let sum = node(AstKind::Plus, vec![name("b"), name("c")]);
    let ast = node(AstKind::Minus, vec![name("a"), sum]);
    assert_eq!(written(&ast), " \\mathit{a}-(\\mathit{b}+\\mathit{c})");
}

#[test]
fn the_operators_of_a_relation_carry_the_trailing_space_of_their_command() {
    // python answers these for the two relations the corpus writes as a command
    let geq = named(AstKind::RelationalGeq, "geq", vec![name("a"), name("b")]);
    assert_eq!(written(&geq), " \\mathit{a}\\ge \\mathit{b}");
    let leq = named(AstKind::RelationalLeq, "leq", vec![name("a"), name("b")]);
    assert_eq!(written(&leq), " \\mathit{a}\\le \\mathit{b}");
    let xor = named(AstKind::LogicalXor, "xor", vec![name("a"), name("b")]);
    assert_eq!(written(&xor), " \\mathit{a}\\oplus \\mathit{b}");
}

#[test]
fn an_absolute_value_is_a_pair_of_stretched_bars() {
    // the entry which spells out the pattern of a liver model holds
    // `\left|\mathit{LI\_\_pattern}-0\right|`
    let difference = node(AstKind::Minus, vec![name("a"), name("b")]);
    let ast = named(AstKind::FunctionAbs, "abs", vec![difference]);
    assert_eq!(written(&ast), " \\left|\\mathit{a}-\\mathit{b}\\right|");
}

#[test]
fn the_constants_and_the_roots_are_what_python_writes() {
    // the corpus holds none of these; python answers each of them
    assert_eq!(written(&named(AstKind::ConstantPi, "pi", vec![])), " \\pi ");
    assert_eq!(
        written(&named(AstKind::ConstantE, "exponentiale", vec![])),
        " e"
    );
    assert_eq!(
        written(&named(AstKind::ConstantTrue, "true", vec![])),
        " \\mathrm{true}"
    );
    let root = named(AstKind::FunctionRoot, "root", vec![integer(2), name("a")]);
    assert_eq!(written(&root), " \\sqrt{\\mathit{a}}");
    let cube = named(AstKind::FunctionRoot, "root", vec![integer(3), name("a")]);
    assert_eq!(written(&cube), " \\sqrt[3]{\\mathit{a}}");
    let floor = named(AstKind::FunctionFloor, "floor", vec![name("a")]);
    assert_eq!(written(&floor), " \\lfloor \\mathit{a}\\rfloor ");
    let negation = named(AstKind::LogicalNot, "not", vec![name("a")]);
    assert_eq!(written(&negation), " \\neg \\mathit{a}");
    let maximum = named(AstKind::FunctionMax, "max", vec![name("a"), name("b")]);
    assert_eq!(written(&maximum), " \\mathrm{max}(\\mathit{a},\\mathit{b})");
}

#[test]
fn a_child_a_node_lacks_is_written_as_nothing() {
    // the stylesheets select the element of an operand by its position and write
    // nothing where a document gave none; python answers ` \frac{\mathit{a}}{}`
    let ast = node(AstKind::Divide, vec![name("a")]);
    assert_eq!(written(&ast), " \\frac{\\mathit{a}}{}");
}

#[test]
fn the_math_of_a_tree_carries_both_fields() {
    let ast = node(AstKind::Times, vec![name("a"), integer(2)]);
    let both = math(&ast).expect("the tree is written");
    assert_eq!(both.latex.as_deref(), Some(" \\mathit{a}\\cdot 2"));
    assert_eq!(both.formula, "a * 2");
}

#[test]
fn a_tree_deeper_than_the_bound_is_an_error() {
    let mut ast = name("x");
    for _ in 1..MAX_AST_DEPTH {
        ast = named(AstKind::FunctionAbs, "abs", vec![ast]);
    }
    assert!(latex(&ast).is_ok(), "a tree of the bound is written");
    let deeper = named(AstKind::FunctionAbs, "abs", vec![ast]);
    assert_eq!(latex(&deeper), Err(MathError::TooDeep));
}

#[test]
fn an_unknown_kind_is_an_error_and_never_a_panic() {
    let ast = node(
        AstKind::Times,
        vec![name("x"), node(AstKind::Other, vec![])],
    );
    assert_eq!(
        latex(&ast),
        Err(MathError::UnknownKind(AstKind::Other)),
        "a writer which wrote a node of an unmapped kind would write a latex which is not the one \
         of the document"
    );
}

#[test]
fn a_chain_of_any_construct_to_the_bound_is_written_without_overflowing_the_stack() {
    // a panic traps a WebAssembly instance, and so does the abort of a stack
    // which ran out: the walk has to carry the deepest tree the bound allows
    // whatever it is made of. The chain is built once with its tail on the left
    // and once on the right, because a sum and a product are written as one
    // operator over a whole chain of their own kind rather than one per level.
    let kinds = [
        AstKind::Minus,
        AstKind::Plus,
        AstKind::Times,
        AstKind::Divide,
        AstKind::Power,
        AstKind::FunctionAbs,
        AstKind::FunctionSin,
        AstKind::FunctionPiecewise,
        AstKind::Lambda,
    ];
    for kind in kinds {
        for tail_on_the_right in [false, true] {
            let mut ast = name("x");
            for _ in 1..MAX_AST_DEPTH {
                let children = if tail_on_the_right {
                    vec![name("y"), ast]
                } else {
                    vec![ast, name("y")]
                };
                ast = node(kind, children);
            }
            assert!(
                latex(&ast).is_ok(),
                "a chain of {kind:?} of the bound is written"
            );
        }
    }
}

#[test]
fn a_chain_of_differences_whose_operands_are_compared_is_written_without_overflowing_the_stack() {
    // this is the shape which asks for the comparison of two string values at
    // every level of itself and with the chain on the side which is read first:
    // the right operand of each difference is an application, so it is written
    // by the template which asks `. != ../*[2]`, and the left operand it is
    // compared against is the whole chain below. A comparison which built that
    // value would walk the chain again at every level, on top of the frames the
    // latex walk already stands on.
    // the sum of the innermost difference hangs two levels below it, so a chain
    // of this many differences is a tree of exactly the bound
    let mut ast = name("x");
    for _ in 2..MAX_AST_DEPTH {
        let sum = node(AstKind::Plus, vec![name("a"), name("b")]);
        ast = node(AstKind::Minus, vec![ast, sum]);
    }
    assert!(latex(&ast).is_ok(), "a chain of the bound is written");
    let deeper = node(
        AstKind::Minus,
        vec![ast, node(AstKind::Plus, vec![name("a"), name("b")])],
    );
    assert_eq!(latex(&deeper), Err(MathError::TooDeep));
}

#[test]
fn a_negative_number_in_exponent_form_keeps_its_sign_in_a_sum() {
    // the plus template of ctopff writes a minus of its own before an operand
    // which is a negative `cn`, and the number of that operand negated, but only
    // for a `cn` without a `sep`: `self::m:cn[not(m:sep) and (number(.) < 0)]`.
    // A plain `cn` whose `"%.15g"` carries an exponent is written with one, so it
    // takes the ordinary branch and keeps the minus of its own mantissa. python
    // answers each of these; the corpus holds no negative number of that shape,
    // so the gate is silent on it.
    let sum = node(AstKind::Plus, vec![name("a"), real(-1e-6)]);
    assert_eq!(written(&sum), " \\mathit{a}+-1\\cdot {10}^{-6}");
    // a negative number which needs no exponent does take that branch
    let sum = node(AstKind::Plus, vec![name("a"), real(-0.5)]);
    assert_eq!(written(&sum), " \\mathit{a}-0.5");

    // the same holds where the number is the first factor of a product, whose
    // sign the template pulls out
    let product = node(AstKind::Times, vec![real(-1e-6), name("b")]);
    let sum = node(AstKind::Plus, vec![name("a"), product]);
    assert_eq!(
        written(&sum),
        " \\mathit{a}+-1\\cdot {10}^{-6}\\cdot \\mathit{b}"
    );

    // and in the parentheses of a sum of one operand, which hold an operator of
    // their own only where the template wrote that minus
    let sum = node(AstKind::Plus, vec![real(-1e-6)]);
    let product = node(AstKind::Times, vec![name("x"), sum]);
    assert_eq!(
        written(&product),
        " \\mathit{x}\\cdot \\left(-1\\cdot {10}^{-6}\\right)"
    );
    let sum = node(AstKind::Plus, vec![real(-0.5)]);
    let product = node(AstKind::Times, vec![name("x"), sum]);
    assert_eq!(written(&product), " \\mathit{x}\\cdot (-0.5)");
}

#[test]
fn a_number_whose_value_is_not_finite_is_written_as_the_element_libsbml_writes() {
    // `writeCN` of libsbml branches on the value of every node `isReal()`
    // covers, which is `AST_REAL`, `AST_REAL_E` and `AST_RATIONAL`, and writes a
    // `notanumber`, an `infinity` or an apply of a minus and an infinity rather
    // than a `cn`. A rational of a zero denominator is such a node and needs
    // nothing but plain content MathML to reach. python answers each of these.
    assert_eq!(written(&rational(3, 0)), " \\infty ");
    assert_eq!(written(&rational(-3, 0)), " -\\infty ");
    assert_eq!(written(&rational(0, 0)), " \\mathrm{NaN}");

    // libsbml writes a number in exponent form whose value is the positive
    // infinity as a `cn` all the same, which is the one asymmetry of the branch
    assert_eq!(written(&real_e(-1.0, 400)), " -\\infty ");
    assert_eq!(written(&real_e(1.0, 400)), " 1\\cdot {10}^{400}");

    // and the element stands wherever the number stood
    let sum = node(AstKind::Plus, vec![name("x"), rational(3, 0)]);
    assert_eq!(written(&sum), " \\mathit{x}+\\infty ");
    let product = node(AstKind::Times, vec![name("x"), rational(-3, 0)]);
    assert_eq!(written(&product), " \\mathit{x}\\cdot -\\infty ");
    let power = node(AstKind::Power, vec![rational(-3, 0), integer(2)]);
    assert_eq!(written(&power), " {(-\\infty )}^{2}");
    // the argument is an application, so the template parenthesises it
    let call = named(AstKind::FunctionSin, "sin", vec![rational(-3, 0)]);
    assert_eq!(written(&call), " \\mathrm{sin}\\left(-\\infty \\right)");
    let logarithm = named(
        AstKind::FunctionLog,
        "log",
        vec![rational(-3, 0), name("x")],
    );
    assert_eq!(
        written(&logarithm),
        " {\\mathrm{log}}_{-\\infty }, \\mathit{x}"
    );
}

#[test]
fn a_negative_infinity_is_a_negated_operand_of_a_sum() {
    // the plus template asks `self::m:apply[count(*)=2 and *[1][self::m:minus]]`,
    // which the apply libsbml writes for a negative infinity satisfies although
    // the node carries no child at all. python answers each of these; the Level 1
    // formula `a + -INF` is one way a document reaches it.
    let sum = node(AstKind::Plus, vec![name("a"), real(f64::NEG_INFINITY)]);
    assert_eq!(written(&sum), " \\mathit{a}-\\infty ");
    let sum = node(AstKind::Plus, vec![name("a"), real_e(-1.0, 400)]);
    assert_eq!(written(&sum), " \\mathit{a}-\\infty ");
    let sum = node(AstKind::Plus, vec![name("a"), rational(-3, 0)]);
    assert_eq!(written(&sum), " \\mathit{a}-\\infty ");
    // the positive infinity and the not a number are ordinary operands
    let sum = node(AstKind::Plus, vec![name("a"), real(f64::INFINITY)]);
    assert_eq!(written(&sum), " \\mathit{a}+\\infty ");
    let sum = node(AstKind::Plus, vec![name("a"), real(f64::NAN)]);
    assert_eq!(written(&sum), " \\mathit{a}+\\mathrm{NaN}");
    // a difference writes the sign of its right operand as it stands
    let difference = node(AstKind::Minus, vec![name("a"), real_e(-1.0, 400)]);
    assert_eq!(written(&difference), " \\mathit{a}--\\infty ");
}

#[test]
fn the_apply_of_a_negative_infinity_is_written_inline_and_its_text_is_three_spaces() {
    // `writeCN` writes that apply with `setAutoIndent(false)` and a space around
    // each of its two elements, so its string value is three spaces where the
    // same apply spelled out by a document carries the indentation of its level.
    // The two therefore differ, and the difference below is parenthesised where
    // one of the same shape on both sides would not be. python answers
    // ` \mathit{a}\cdot -\infty -(\mathit{a}-\infty )`.
    let negated = node(AstKind::Minus, vec![real(f64::INFINITY)]);
    let product = node(AstKind::Times, vec![name("a"), real(f64::NEG_INFINITY)]);
    let sum = node(AstKind::Plus, vec![name("a"), negated]);
    let difference = node(AstKind::Minus, vec![product, sum]);
    assert_eq!(
        written(&difference),
        " \\mathit{a}\\cdot -\\infty -(\\mathit{a}-\\infty )"
    );
}

#[test]
fn the_largest_int_is_written_in_exponent_notation_like_libxml2() {
    // `xmlXPathFormatNumber` writes a value as an integer only where
    // `number > INT_MIN && number < INT_MAX`, so the largest `int` itself is
    // written by the branch above it. python answers
    // ` \mathit{a}-2.147483647e+09` for this sum, whose operand the plus
    // template negates and writes with the XPath number format.
    let sum = node(AstKind::Plus, vec![name("a"), integer(-2_147_483_647)]);
    assert_eq!(written(&sum), " \\mathit{a}-2.147483647e+09");
    let sum = node(AstKind::Plus, vec![name("a"), integer(-2_147_483_646)]);
    assert_eq!(written(&sum), " \\mathit{a}-2147483646");
}
