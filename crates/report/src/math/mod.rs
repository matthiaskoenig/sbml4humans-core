//! The writers of the math of an element.
//!
//! [`formula`] writes the Level 3 formula of a syntax tree and [`latex`] its
//! latex, the two fields of the math of the report, which [`math`] writes
//! together. Both fields of a math are what the python implementation writes for
//! the same document, which the gate of `crates/oracles` compares entry by entry
//! against the whole corpus. The latex of a math is absent where the python
//! implementation writes none, which [`math`] names: there are two maths it
//! cannot typeset and reports as their formula alone.
//!
//! A writer reads the `name` of a node for a symbol, for a call of a function of
//! the model and for a csymbol, which are the three the MathML carries the text
//! of, and keys everything else off the kind of the node: libsbml answers its
//! own built-in name for every other node, so a node of the kind
//! [`AstKind::FunctionAbs`] carries the name `abs` and one of the kind
//! [`AstKind::ConstantPi`] carries `pi`, and a writer which fell back to the
//! name would be right on those and wrong wherever libsbml names a node
//! differently from the way a formula or a latex writes it. A csymbol is
//! typeset by the text the document gave it, so one which spells the time
//! csymbol `t` is written ` t` and not ` \mathrm{time}`.
//!
//! No writer panics on any tree, whatever a document holds: this crate is built
//! for WebAssembly, where a panic traps the instance and takes the report of
//! every other document of the process with it.

mod formula;
mod latex;

pub use formula::formula;
pub use latex::latex;

use latex::{MAX_MATHML_DEPTH, holds_a_childless_lambda, mathml_depth};

use crate::types::Math;
use sbml4humans_model::{Ast, AstKind, MAX_AST_DEPTH};

/// The math of an element: its latex and its Level 3 formula.
///
/// The latex is absent exactly where the python implementation writes `null`
/// for it, which `math_info` of `mathml.py` (`mathml.py:309-332`) does for the
/// two maths it cannot typeset, logging a warning and reporting the infix
/// formula alone:
///
/// - a tree which holds a lambda without children anywhere in it, which the
///   mathml writer of libsbml dies on ([`holds_a_childless_lambda`]);
/// - a tree whose mathml is deeper than [`MAX_MATHML_DEPTH`] elements, which the
///   parser of the stylesheet pipeline refuses ([`mathml_depth`]).
///
/// The formula is written for any tree at any depth, so the math of an element
/// is reported either way and the report of a document is never lost over one
/// math.
///
/// # Errors
///
/// What [`latex`] and [`formula`] answer: [`MathError::TooDeep`] for a tree
/// nested deeper than [`MAX_AST_DEPTH`] and [`MathError::UnknownKind`] for one
/// which holds a node of a kind the libsbml shim does not map.
pub fn math(ast: &Ast) -> Result<Math, MathError> {
    // the formula first, as the python implementation writes it first: a tree
    // which no writer of this module writes is refused by it and never reaches
    // the two rules below
    let formula = formula(ast)?;
    let latex = if holds_a_childless_lambda(ast) || mathml_depth(ast)? > MAX_MATHML_DEPTH {
        None
    } else {
        Some(latex(ast)?)
    };
    Ok(Math { latex, formula })
}

/// Why the math of an element could not be written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MathError {
    /// The syntax tree is nested deeper than [`MAX_AST_DEPTH`].
    #[error("the math is nested deeper than {MAX_AST_DEPTH} levels")]
    TooDeep,
    /// The tree holds a node of a kind no writer knows, which only
    /// [`AstKind::Other`] is: the kind the libsbml shim answers for a type it
    /// does not map, a node of a package this crate does not write.
    #[error("the math holds a node of the kind {0:?}, which no writer writes")]
    UnknownKind(AstKind),
    /// A node lacks a child a writer of its kind reads.
    ///
    /// Every writer asks for a child of a node whose shape it has established,
    /// so a tree the libsbml shim filled never answers this; it is here so that
    /// no writer indexes into the children of a node and panics instead.
    #[error("a node of the math lacks a child its writer reads")]
    MissingChild,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbml4humans_model::AstNumber;

    /// An inner node of `kind` with `children`.
    fn node(kind: AstKind, children: Vec<Ast>) -> Ast {
        Ast {
            kind,
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

    /// A number node of `kind` which holds `number`.
    fn number(kind: AstKind, number: AstNumber) -> Ast {
        Ast {
            kind,
            number: Some(number),
            ..Ast::default()
        }
    }

    /// A `cn type="integer"`.
    fn integer(value: i64) -> Ast {
        number(AstKind::Integer, AstNumber::Integer(value))
    }

    /// A `cn` which holds the real `value`.
    fn real(value: f64) -> Ast {
        number(AstKind::Real, AstNumber::Real(value))
    }

    /// A chain of `n` nested quotients over a `ci`.
    ///
    /// libsbml writes one `apply` per quotient and one `ci` for the leaf, so the
    /// mathml document of such a chain is `n + 2` elements deep, the `<math>`
    /// root counted. The controller measured the bound with this shape against
    /// the python implementation: a chain of 254 is written with its latex and
    /// one of 255 without it.
    fn nested_applies(n: usize) -> Ast {
        let mut ast = name("x");
        for _ in 0..n {
            ast = node(AstKind::Divide, vec![name("a"), ast]);
        }
        ast
    }

    /// The deepest chain of [`nested_applies`] whose mathml lxml still parses.
    const AT_THE_BOUND: usize = 254;

    /// One level more, which the python implementation writes without a latex.
    const ABOVE_THE_BOUND: usize = 255;

    /// A sum whose second operand is a lambda without children.
    ///
    /// The shape crashes the mathml writer of libsbml wherever it stands, so the
    /// python implementation refuses the whole tree over it.
    fn lambda_without_children_inside_a_sum() -> Ast {
        node(
            AstKind::Plus,
            vec![name("a"), node(AstKind::Lambda, Vec::new())],
        )
    }

    #[test]
    fn a_math_which_holds_a_lambda_without_children_is_written_without_its_latex() {
        let math = math(&lambda_without_children_inside_a_sum()).unwrap();
        assert_eq!(math.latex, None);
        assert!(!math.formula.is_empty());
    }

    #[test]
    fn a_math_whose_mathml_is_too_deep_for_the_stylesheet_is_written_without_its_latex() {
        assert_eq!(math(&nested_applies(ABOVE_THE_BOUND)).unwrap().latex, None);
        assert!(math(&nested_applies(AT_THE_BOUND)).unwrap().latex.is_some());
    }

    // A piecewise writes two elements above its children, so the bound of a
    // chain which stands inside one is two levels lower than the bound of the
    // bare chain. Both thresholds were measured against the python
    // implementation, which answers a latex for a chain of 252 inside a
    // piecewise and none for one of 253.
    #[test]
    fn the_bound_counts_the_elements_of_the_shape_around_a_chain() {
        let inside = |n| {
            node(
                AstKind::FunctionPiecewise,
                vec![nested_applies(n), name("c")],
            )
        };
        assert!(math(&inside(252)).unwrap().latex.is_some());
        assert_eq!(math(&inside(253)).unwrap().latex, None);
    }

    // Every depth of this table is the depth of the mathml
    // `libsbml.writeMathMLToString` writes for the same tree, measured with
    // `lxml` against libsbml 5.21.2; the shapes which are not a plain apply
    // were measured at the bound as well, through `sbml4humans.mathml.math_info`.
    #[test]
    fn the_mathml_depth_of_a_shape_is_the_one_libsbml_writes() {
        let x = || name("x");
        let unknown = || node(AstKind::Unknown, Vec::new());
        let cases: Vec<(&str, Ast, usize)> = vec![
            // the token elements, which carry text and no element
            ("ci", x(), 2),
            ("cn integer", integer(1), 2),
            ("cn real", real(1.5), 2),
            ("cn real which needs an exponent", real(1e300), 3),
            ("csymbol time", node(AstKind::NameTime, Vec::new()), 2),
            ("pi", node(AstKind::ConstantPi, Vec::new()), 2),
            // the numbers libsbml writes more than one element of
            (
                "cn in exponent form",
                number(
                    AstKind::RealE,
                    AstNumber::RealE {
                        mantissa: 1.0,
                        exponent: 10,
                    },
                ),
                3,
            ),
            (
                "cn rational",
                number(
                    AstKind::Rational,
                    AstNumber::Rational {
                        numerator: 3,
                        denominator: 4,
                    },
                ),
                3,
            ),
            ("infinity", real(f64::INFINITY), 2),
            ("notanumber", real(f64::NAN), 2),
            ("the negated infinity", real(f64::NEG_INFINITY), 3),
            // an apply, which writes its operator element whatever its operands
            ("quotient", node(AstKind::Divide, vec![integer(1), x()]), 3),
            ("sum of three", node(AstKind::Plus, vec![x(), x(), x()]), 3),
            ("absolute value", node(AstKind::FunctionAbs, vec![x()]), 3),
            (
                "absolute value of nothing",
                node(AstKind::FunctionAbs, vec![unknown()]),
                3,
            ),
            (
                "product of nothing",
                node(AstKind::Times, vec![unknown()]),
                3,
            ),
            ("function call", node(AstKind::Function, vec![x(), x()]), 3),
            (
                "function call of no argument",
                node(AstKind::Function, Vec::new()),
                3,
            ),
            (
                "delay",
                node(AstKind::FunctionDelay, vec![x(), integer(1)]),
                3,
            ),
            (
                "relation of three",
                node(AstKind::RelationalLt, vec![x(), x(), x()]),
                3,
            ),
            // the middle child of an operator of three is written as no element
            (
                "difference of three",
                node(
                    AstKind::Minus,
                    vec![x(), node(AstKind::Plus, vec![x(), x()]), x()],
                ),
                3,
            ),
            // a sum or a product of at most two children unrolls a child of its
            // own kind into itself, and one of three does not
            (
                "sum over a sum",
                node(
                    AstKind::Plus,
                    vec![x(), node(AstKind::Plus, vec![x(), x()])],
                ),
                3,
            ),
            (
                "sum of three over a sum",
                node(
                    AstKind::Plus,
                    vec![x(), x(), node(AstKind::Plus, vec![x(), x()])],
                ),
                4,
            ),
            (
                "sum over a product",
                node(
                    AstKind::Plus,
                    vec![x(), node(AstKind::Times, vec![x(), x()])],
                ),
                4,
            ),
            // the logbase and the degree wrappers, which libsbml writes for a
            // node of more than one child and then also around nothing
            (
                "logarithm to a base",
                node(AstKind::FunctionLog, vec![integer(10), x()]),
                4,
            ),
            (
                "logarithm of one child",
                node(AstKind::FunctionLog, vec![x()]),
                3,
            ),
            (
                "logarithm of no base",
                node(AstKind::FunctionLog, vec![unknown(), x()]),
                3,
            ),
            (
                "root of a degree",
                node(AstKind::FunctionRoot, vec![integer(3), x()]),
                4,
            ),
            (
                "root of one child",
                node(AstKind::FunctionRoot, vec![x()]),
                3,
            ),
            (
                "root of no degree",
                node(AstKind::FunctionRoot, vec![unknown(), x()]),
                3,
            ),
            // the piece and otherwise wrappers of a piecewise
            (
                "piecewise of a condition",
                node(
                    AstKind::FunctionPiecewise,
                    vec![x(), node(AstKind::RelationalGt, vec![x(), integer(1)])],
                ),
                5,
            ),
            (
                "piecewise of two pieces and an otherwise",
                node(AstKind::FunctionPiecewise, vec![x(), x(), x(), x(), x()]),
                4,
            ),
            (
                "piecewise of nothing",
                node(AstKind::FunctionPiecewise, Vec::new()),
                2,
            ),
            (
                "piecewise of an otherwise of nothing",
                node(AstKind::FunctionPiecewise, vec![unknown()]),
                3,
            ),
            // the bvar wrappers of a lambda, which stand around every child but
            // the last
            (
                "lambda of two bound variables",
                node(
                    AstKind::Lambda,
                    vec![x(), x(), node(AstKind::Plus, vec![x(), x()])],
                ),
                4,
            ),
            (
                "lambda of a body alone",
                node(AstKind::Lambda, vec![x()]),
                3,
            ),
            (
                "lambda of nothing",
                node(AstKind::Lambda, vec![unknown()]),
                2,
            ),
            (
                "lambda of a bound variable of nothing",
                node(AstKind::Lambda, vec![unknown(), x()]),
                3,
            ),
            // a node of a math libsbml could not read is written as no element
            // at all, and nothing of what stands below it either
            (
                "a chain below an unknown node",
                node(
                    AstKind::Times,
                    vec![x(), node(AstKind::Unknown, vec![nested_applies(400)])],
                ),
                3,
            ),
        ];
        let mut differences = Vec::new();
        for (label, ast, expected) in cases {
            match mathml_depth(&ast) {
                Ok(ours) if ours == expected => {}
                Ok(ours) => {
                    differences.push(format!("{label}: ours {ours} but libsbml {expected}"))
                }
                Err(error) => {
                    differences.push(format!("{label}: ours {error} but libsbml {expected}"))
                }
            }
        }
        assert!(differences.is_empty(), "{}", differences.join("\n"));
    }
}
