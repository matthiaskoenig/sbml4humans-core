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
//! [`sbml4humans_model::AstKind::FunctionAbs`] carries the name `abs` and one of the kind
//! [`sbml4humans_model::AstKind::ConstantPi`] carries `pi`, and a writer which fell back to the
//! name would be right on those and wrong wherever libsbml names a node
//! differently from the way a formula or a latex writes it. A csymbol is
//! typeset by the text the document gave it, so one which spells the time
//! csymbol `t` is written ` t` and not ` \mathrm{time}`.
//!
//! No writer panics on any tree, whatever a document holds: this crate is built
//! for WebAssembly, where a panic traps the instance and takes the report of
//! every other document of the process with it.

mod latex;
mod references;

pub use latex::latex;
pub use references::free_symbols;
pub use sbml4humans_model::formatting::{MathError, formula};

use latex::{MAX_MATHML_DEPTH, holds_a_childless_lambda, mathml_depth};

use crate::types::Math;
use sbml4humans_model::Ast;

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
/// nested deeper than [`sbml4humans_model::MAX_AST_DEPTH`] and [`MathError::UnknownKind`] for one
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

#[cfg(test)]
mod tests {
    use super::*;
    use sbml4humans_model::{AstKind, AstNumber, MAX_AST_DEPTH};

    /// An inner node of `kind` with `children`.
    fn node(kind: AstKind, children: Vec<Ast>) -> Ast {
        Ast {
            kind,
            children,
            name: None,
            number: None,
            units: None,
        }
    }

    /// A `ci` which names `id`.
    fn name(id: &str) -> Ast {
        Ast {
            kind: AstKind::Name,
            name: Some(id.to_owned()),
            number: None,
            units: None,
            children: Vec::new(),
        }
    }

    /// A number node of `kind` which holds `number`.
    fn number(kind: AstKind, number: AstNumber) -> Ast {
        Ast {
            kind,
            number: Some(number),
            name: None,
            units: None,
            children: Vec::new(),
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

    /// A chain of `levels` levels of `kind`, its tail the right child of each
    /// node and a `ci` at the bottom.
    fn chain(kind: AstKind, levels: usize) -> Ast {
        let mut ast = name("x");
        for _ in 1..levels {
            ast = node(kind, vec![name("a"), ast]);
        }
        ast
    }

    /// A chain of `n` nested quotients over a `ci`.
    ///
    /// libsbml writes one `apply` per quotient and one `ci` for the leaf, so the
    /// mathml document of such a chain is `n + 2` elements deep, the `<math>`
    /// root counted. The controller measured the bound with this shape against
    /// the python implementation: a chain of 254 is written with its latex and
    /// one of 255 without it.
    fn nested_applies(n: usize) -> Ast {
        chain(AstKind::Divide, n + 1)
    }

    /// The stack `wasm32-unknown-unknown` passes, its `stack-size=1048576`.
    ///
    /// The tests which walk a tree at the cap of the model run on a thread of
    /// exactly this size, so that what they measure is the budget of the target
    /// this crate is built for and not the two megabytes a test thread is
    /// given. A stack which runs out aborts the process rather than failing the
    /// test, so such a test goes red by taking the whole run down with it.
    const WASM_STACK: usize = 1024 * 1024;

    /// Runs `work` on a thread of exactly [`WASM_STACK`].
    fn on_a_wasm_stack(work: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(WASM_STACK)
            .spawn(work)
            .expect("a thread of the pinned stack starts")
            .join()
            .expect("the walk of the tree did not panic");
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

    // The guard walks the whole tree, so the shape takes the latex of the math
    // it is in wherever it stands. The formula is the one python writes for the
    // same tree, which is the point of the rule: the math is still reported.
    #[test]
    fn a_childless_lambda_deep_in_a_tree_takes_the_latex_of_the_whole_math() {
        let mut ast = node(AstKind::Lambda, Vec::new());
        for _ in 0..8 {
            ast = node(AstKind::Times, vec![name("a"), ast]);
        }
        let math = math(&ast).unwrap();
        assert_eq!(math.latex, None);
        assert_eq!(
            math.formula,
            "a * (a * (a * (a * (a * (a * (a * (a * lambda())))))))"
        );
    }

    // A lambda is the math of every function definition, so the rule has to fire
    // on the childless shape alone: the latex is the one python writes for this
    // tree, `\lambda(` and `) =` the replacements of `cmathml_to_latex`.
    #[test]
    fn a_lambda_which_has_children_keeps_its_latex() {
        let ast = node(
            AstKind::Lambda,
            vec![
                name("x"),
                name("y"),
                node(AstKind::Plus, vec![name("x"), name("y")]),
            ],
        );
        let math = math(&ast).unwrap();
        assert_eq!(
            math.latex.as_deref(),
            Some(" \\lambda(\\mathit{x}, \\mathit{y}) =\\left(\\mathit{x}+\\mathit{y}\\right)")
        );
        assert_eq!(math.formula, "lambda(x, y, x + y)");
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

    // A chain at the cap of the model is the deepest tree a document can hold
    // and the shape most likely to run a stack out, and a stack which runs out
    // traps a WebAssembly instance. The three kinds are the ones the walks treat
    // differently: libsbml writes one element per level of a chain of quotients,
    // and it unrolls a chain of sums or of products into one apply, which
    // `operands` does on a stack of its own.
    //
    // Measured on this thread in a debug build, bisected in steps of 1 KiB:
    // all three run within 16 KiB, which is the floor of the method and not a
    // figure of the walk, where the recursive walks of the slice before needed
    // 352 KiB and 463 KiB for the same shapes at a tenth of this depth. The
    // table of `latex.rs` carries the before and the after.
    #[test]
    fn the_math_of_a_chain_at_the_cap_is_written_within_the_stack_of_wasm() {
        on_a_wasm_stack(|| {
            for kind in [AstKind::Divide, AstKind::Plus, AstKind::Times] {
                let written = math(&chain(kind, MAX_AST_DEPTH)).expect("a tree of the cap");
                assert!(!written.formula.is_empty(), "{kind:?}");
                // the mathml of a chain of quotients is one element per level
                // and so as deep as the chain, far past the bound of the
                // stylesheet pipeline, while the mathml of a chain which
                // unrolls is three elements whatever its length
                assert_eq!(written.latex.is_some(), kind != AstKind::Divide, "{kind:?}");
                assert_eq!(
                    math(&chain(kind, MAX_AST_DEPTH + 1)).err(),
                    Some(MathError::TooDeep),
                    "{kind:?}"
                );
            }
        });
    }

    /// The deepest tree the depth rule leaves the latex writer, `level` building
    /// one level of it over the tail it is handed.
    ///
    /// Its mathml is exactly as deep as the bound allows, so the latex is
    /// written, and its levels past that are a chain of sums which libsbml
    /// unrolls into one apply. A chain of sums is two elements, the `apply` and
    /// its operands, so three with the `<math>` root: this many levels above one
    /// make the document exactly as deep as the bound allows.
    fn deepest_latex_shape(level: impl Fn(Ast) -> Ast) -> Ast {
        let levels = MAX_MATHML_DEPTH - 3;
        let mut ast = chain(AstKind::Plus, MAX_AST_DEPTH - levels);
        for _ in 0..levels {
            ast = level(ast);
        }
        // the shape stops being the worst case if either number drifts
        assert_eq!(mathml_depth(&ast).unwrap(), MAX_MATHML_DEPTH);
        assert_eq!(depth_of(&ast), MAX_AST_DEPTH);
        ast
    }

    /// One level of a chain of quotients, the tail its divisor.
    fn quotient_over(tail: Ast) -> Ast {
        node(AstKind::Divide, vec![name("a"), tail])
    }

    /// One level of a chain of differences whose two operands are compared.
    ///
    /// The right operand is an apply, so the template which tells the operands
    /// of a difference apart by their string values fires at every level, and
    /// the value it is compared against is the whole chain below. That made it
    /// the costliest level of the walk while the walk recursed: a chain of a
    /// thousand of it needed 1,401 KiB through [`latex`] alone, more than the
    /// stack of wasm, where a chain of this shape at the bound now costs no
    /// call frame at all.
    fn difference_over(tail: Ast) -> Ast {
        node(
            AstKind::Minus,
            vec![tail, node(AstKind::Plus, vec![name("a"), name("b")])],
        )
    }

    // The deepest latex walk the depth rule leaves reachable, in both of the
    // shapes which reach it: a chain of quotients, which is the plain walk, and
    // a chain of differences, which compares a string value at every level and
    // did cost more per level for it. Each of the two is 253 wrappers over a
    // chain of sums at the bound, so the tree below the wrappers is
    // `MAX_AST_DEPTH` levels deep.
    //
    // Measured on this thread in a debug build, bisected in steps of 1 KiB:
    // both run within 16 KiB, the floor of the method. The same two shapes cost
    // 684 KiB and 704 KiB when the walk recursed per level, and a bare chain of
    // a thousand quotients or differences cost 1,335 KiB and 1,401 KiB through
    // `latex` alone, which did not fit in the megabyte of wasm at all. Nothing
    // of this crate follows a tree on the call stack any more, so neither
    // figure has an heir.
    #[test]
    fn the_deepest_latex_the_bound_allows_is_written_within_the_stack_of_wasm() {
        for shape in [
            deepest_latex_shape(quotient_over),
            deepest_latex_shape(difference_over),
        ] {
            on_a_wasm_stack(move || {
                assert!(math(&shape).expect("a tree of the cap").latex.is_some());
            });
        }
    }

    /// How many levels deep the syntax tree `ast` is, without recursing.
    fn depth_of(ast: &Ast) -> usize {
        let mut deepest = 0;
        let mut stack = vec![(ast, 1)];
        while let Some((node, level)) = stack.pop() {
            deepest = deepest.max(level);
            stack.extend(node.children.iter().map(|child| (child, level + 1)));
        }
        deepest
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
