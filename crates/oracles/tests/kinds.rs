//! Every kind the libsbml shim maps writes the formula and the latex python writes.
//!
//! The gate of `tests/math.rs` compares the two writers over the whole math corpus, but the
//! corpus is a corpus: 39 of the 84 kinds appear in it and 45 do not (44 of them mapped, the 45th
//! is `Other`), among them 23 of the 24 trigonometric and hyperbolic kinds, every constant, the
//! rational, the `Power` of a Level 1 formula and the
//! whole `\sqrt[n]{}` branch. This table covers the rest.
//!
//! Each row is a tree built by hand, the formula `libsbml.formulaToL3String` writes of it and the
//! latex `sbml4humans.mathml.astnode_to_latex` writes of it. Both expectations were generated once
//! by a probe against the python implementation, over trees built with libsbml itself and read
//! back the way the shim reads them, and are committed as literals: this test needs no libsbml at
//! all and so says nothing about the fill side, which `crates/libsbml/tests/math.rs` covers with a
//! kind table of its own.
//!
//! The table holds one row per mapped kind and thirty-five rows more, for the shapes of a construct
//! whose branch depends on them: a difference of one operand, a logarithm of the base ten and of
//! one child, a root of the degree two and of one child, a call of no argument and of one, and the
//! twenty-six of `AstKind::Unknown`, the node libsbml answers of a math it could not read. That
//! kind earns twenty-six rows because libsbml writes no mathml element of it at all, so the latex
//! of a tree which holds one is the latex of the tree without it, and every construct which counts
//! its operands writes a different shape for the operand that is gone. The logarithm and the root
//! are the two constructs that are no row of it, and `latex.rs` says why.
//!
//! The non-breaking spaces of the cases environment are written as `\u{a0}` in the rows which were
//! added with the unknown node, as `latex.rs` writes them, rather than as the literal characters of
//! the rows before them, so that a reader can see them.

use sbml4humans_model::{Ast, AstKind, AstNumber};
use sbml4humans_report::{formula, latex};

/// A node of a tree a row of the table builds.
struct Node {
    /// The kind of the node.
    kind: AstKind,
    /// The name libsbml answers of it, [`None`] where it answers none.
    name: Option<&'static str>,
    /// The number it holds, [`None`] where it is no number.
    number: Option<AstNumber>,
    /// Its children.
    children: &'static [Node],
}

/// A node of `kind` over `children`, which libsbml answers no name of.
const fn over(kind: AstKind, children: &'static [Node]) -> Node {
    Node {
        kind,
        name: None,
        number: None,
        children,
    }
}

/// A node of `kind` over `children`, which libsbml answers `name` of.
const fn call(kind: AstKind, name: &'static str, children: &'static [Node]) -> Node {
    Node {
        kind,
        name: Some(name),
        number: None,
        children,
    }
}

/// A number node of `kind` which holds `number`.
const fn number(kind: AstKind, number: AstNumber) -> Node {
    Node {
        kind,
        name: None,
        number: Some(number),
        children: &[],
    }
}

/// The syntax tree of `node`, as the shim would have filled it.
fn to_ast(node: &Node) -> Ast {
    Ast {
        kind: node.kind,
        name: node.name.map(str::to_owned),
        number: node.number.clone(),
        children: node.children.iter().map(to_ast).collect(),
        ..Ast::default()
    }
}

/// The two children every unknown node of the table carries, `x` and `y`.
const UNKNOWN_CHILDREN: &[Node] = &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])];

/// The unknown node of two children which stands in the rows of that kind.
const UNKNOWN: Node = over(AstKind::Unknown, UNKNOWN_CHILDREN);

/// The name `a`, the known operand beside the unknown one in those rows.
const A: Node = call(AstKind::Name, "a", &[]);

/// The condition `a > 0` of the piece of the two piecewise rows of the unknown node.
const PIECEWISE_CONDITION: Node = call(
    AstKind::RelationalGt,
    "gt",
    &[
        call(AstKind::Name, "a", &[]),
        number(AstKind::Integer, AstNumber::Integer(0)),
    ],
);

/// One tree per kind, with the formula and the latex python writes of it.
const CASES: &[(Node, &str, &str)] = &[
    // the unknown node alone: libsbml writes its children as a parenthesised list in a
    // formula and writes no mathml element of it at all, so its latex is the one space
    // every latex of the report begins with and nothing else
    (over(AstKind::Unknown, &[]), "()", " "),
    (
        over(AstKind::Unknown, &[call(AstKind::Name, "x", &[])]),
        "(x)",
        " ",
    ),
    (UNKNOWN, "(x, y)", " "),
    (
        over(AstKind::Unknown, &[UNKNOWN, call(AstKind::Name, "z", &[])]),
        "((x, y), z)",
        " ",
    ),
    // the unknown node as an operand: the formula writes it, the latex is the latex of the
    // construct without that operand, which is a different shape for every one of them
    (
        over(
            AstKind::Plus,
            &[
                over(AstKind::Plus, &[call(AstKind::Name, "a", &[]), UNKNOWN]),
                call(AstKind::Name, "b", &[]),
            ],
        ),
        "a + (x, y) + b",
        " \\mathit{a}+\\mathit{b}",
    ),
    (
        over(AstKind::Divide, &[UNKNOWN, call(AstKind::Name, "z", &[])]),
        "(x, y) / z",
        " \\frac{\\mathit{z}}{}",
    ),
    (
        call(
            AstKind::FunctionPower,
            "power",
            &[UNKNOWN, call(AstKind::Name, "z", &[])],
        ),
        "(x, y)^z",
        " {\\mathit{z}}^{}",
    ),
    (
        call(AstKind::FunctionAbs, "abs", &[UNKNOWN]),
        "abs((x, y))",
        " \\left|\\right|",
    ),
    (
        call(AstKind::Function, "f", &[UNKNOWN]),
        "f((x, y))",
        " \\mathit{f}\\left(\\right)",
    ),
    // the left operand of the difference is a sum of one written operand and the right one a
    // sum of two, so the string values of the two differ and the right one is parenthesised
    (
        over(
            AstKind::Minus,
            &[
                over(AstKind::Plus, &[call(AstKind::Name, "a", &[]), UNKNOWN]),
                over(
                    AstKind::Plus,
                    &[call(AstKind::Name, "a", &[]), call(AstKind::Name, "b", &[])],
                ),
            ],
        ),
        "a + (x, y) - (a + b)",
        " \\mathit{a}-(\\mathit{a}+\\mathit{b})",
    ),
    // a pair of children of a piecewise is a piece and a trailing single child is the
    // otherwise, whatever the two are, so an unknown node leaves its slot empty and the
    // children of its piece which are written move up into the slots before it
    (
        call(
            AstKind::FunctionPiecewise,
            "piecewise",
            &[UNKNOWN, PIECEWISE_CONDITION, call(AstKind::Name, "b", &[])],
        ),
        "piecewise((x, y), a > 0, b)",
        " \\begin{cases} \\mathit{a}>0& \\text{\u{a0} if \u{a0}} & \\\\ \\mathit{b}& \
         {\\text{\u{a0} otherwise}}\\end{cases}",
    ),
    (
        call(
            AstKind::FunctionPiecewise,
            "piecewise",
            &[call(AstKind::Name, "a", &[]), PIECEWISE_CONDITION, UNKNOWN],
        ),
        "piecewise(a, a > 0, (x, y))",
        " \\begin{cases} \\mathit{a}& \\text{\u{a0} if \u{a0}} & \\mathit{a}>0\\\\ & \
         {\\text{\u{a0} otherwise}}\\end{cases}",
    ),
    // one row per further branch of the latex writer which an operand that is gone reaches,
    // and the slot the unknown node stands in decides nothing: a quotient and a power are
    // the same whichever of their two operands it is
    (
        call(AstKind::FunctionSin, "sin", &[UNKNOWN]),
        "sin((x, y))",
        " \\mathrm{sin}",
    ),
    (
        call(AstKind::FunctionLn, "ln", &[UNKNOWN]),
        "ln((x, y))",
        " \\mathrm{ln}",
    ),
    (
        call(AstKind::FunctionFloor, "floor", &[UNKNOWN]),
        "floor((x, y))",
        " \\lfloor \\rfloor ",
    ),
    (
        call(AstKind::FunctionExp, "exp", &[UNKNOWN]),
        "exp((x, y))",
        " {e}^{}",
    ),
    (
        call(AstKind::FunctionMax, "max", &[UNKNOWN, A]),
        "max((x, y), a)",
        " \\mathrm{max}\\left(\\mathit{a}\\right)",
    ),
    (
        call(AstKind::LogicalAnd, "and", &[UNKNOWN, A]),
        "(x, y) && a",
        " \\mathit{a}",
    ),
    (
        call(AstKind::LogicalNot, "not", &[UNKNOWN]),
        "!(x, y)",
        " \\neg ",
    ),
    (
        call(AstKind::LogicalImplies, "implies", &[UNKNOWN, A]),
        "implies((x, y), a)",
        " \\mathit{a}\\Rightarrow ",
    ),
    (
        call(AstKind::FunctionDelay, "delay", &[UNKNOWN, A]),
        "delay((x, y), a)",
        " \\mathrm{delay}\\left(\\mathit{a}\\right)",
    ),
    (
        call(AstKind::FunctionRateOf, "rateOf", &[UNKNOWN]),
        "rateOf((x, y))",
        " \\mathrm{rateOf}\\left(\\right)",
    ),
    (
        over(AstKind::Times, &[A, UNKNOWN]),
        "a * (x, y)",
        " \\mathit{a}",
    ),
    (
        over(AstKind::Minus, &[A, UNKNOWN]),
        "a - (x, y)",
        " -\\mathit{a}",
    ),
    // a difference of no written operand at all, which ctopff writes as a call of its name
    (
        over(AstKind::Minus, &[UNKNOWN]),
        "-(x, y)",
        " \\mathrm{minus}\\left(\\right)",
    ),
    (
        call(AstKind::FunctionPower, "power", &[A, UNKNOWN]),
        "a^(x, y)",
        " {\\mathit{a}}^{}",
    ),
    (
        over(AstKind::Divide, &[A, UNKNOWN]),
        "a / (x, y)",
        " \\frac{\\mathit{a}}{}",
    ),
    (call(AstKind::Name, "x", &[]), "x", " \\mathit{x}"),
    (
        call(AstKind::NameTime, "time", &[]),
        "time",
        " \\mathrm{time}",
    ),
    (
        call(AstKind::NameAvogadro, "avogadro", &[]),
        "avogadro",
        " \\mathrm{avogadro}",
    ),
    (number(AstKind::Integer, AstNumber::Integer(2)), "2", " 2"),
    (number(AstKind::Real, AstNumber::Real(1.5)), "1.5", " 1.5"),
    (
        number(
            AstKind::RealE,
            AstNumber::RealE {
                mantissa: 1.5,
                exponent: 3,
            },
        ),
        "1.5e3",
        " 1.5\\cdot {10}^{3}",
    ),
    (
        number(
            AstKind::Rational,
            AstNumber::Rational {
                numerator: 3,
                denominator: 4,
            },
        ),
        "(3/4)",
        " 3/4",
    ),
    (
        call(AstKind::ConstantE, "exponentiale", &[]),
        "exponentiale",
        " e",
    ),
    (
        call(AstKind::ConstantFalse, "false", &[]),
        "false",
        " \\mathrm{false}",
    ),
    (call(AstKind::ConstantPi, "pi", &[]), "pi", " \\pi "),
    (
        call(AstKind::ConstantTrue, "true", &[]),
        "true",
        " \\mathrm{true}",
    ),
    (
        over(
            AstKind::Plus,
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "x + y",
        " \\mathit{x}+\\mathit{y}",
    ),
    (
        over(
            AstKind::Minus,
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "x - y",
        " \\mathit{x}-\\mathit{y}",
    ),
    (
        over(AstKind::Minus, &[call(AstKind::Name, "x", &[])]),
        "-x",
        " -\\mathit{x}",
    ),
    (
        over(
            AstKind::Times,
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "x * y",
        " \\mathit{x}\\cdot \\mathit{y}",
    ),
    (
        over(
            AstKind::Divide,
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "x / y",
        " \\frac{\\mathit{x}}{\\mathit{y}}",
    ),
    (
        over(
            AstKind::Power,
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "x^y",
        " {\\mathit{x}}^{\\mathit{y}}",
    ),
    (
        call(
            AstKind::Function,
            "f",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "f(x, y)",
        " \\mathit{f}\\left(\\mathit{x},\\mathit{y}\\right)",
    ),
    (
        call(AstKind::Function, "f", &[]),
        "f()",
        " \\mathit{f}\\left(\\right)",
    ),
    (
        call(
            AstKind::CsymbolFunction,
            "myfunc",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "myfunc(x, y)",
        " \\mathrm{myfunc}\\left(\\mathit{x},\\mathit{y}\\right)",
    ),
    // a name of one character stands in an `mi` which the stylesheet writes as it is, where
    // a longer one is wrapped in a `\mathrm`, and the arity decides nothing of its own
    (
        call(AstKind::CsymbolFunction, "f", &[]),
        "f()",
        " f\\left(\\right)",
    ),
    (
        call(
            AstKind::CsymbolFunction,
            "f",
            &[call(AstKind::Name, "x0", &[])],
        ),
        "f(x0)",
        " f\\left(\\mathit{x0}\\right)",
    ),
    (
        call(
            AstKind::CsymbolFunction,
            "f",
            &[
                call(AstKind::Name, "x0", &[]),
                call(AstKind::Name, "x1", &[]),
            ],
        ),
        "f(x0, x1)",
        " f\\left(\\mathit{x0},\\mathit{x1}\\right)",
    ),
    (
        call(
            AstKind::FunctionAbs,
            "abs",
            &[call(AstKind::Name, "x", &[])],
        ),
        "abs(x)",
        " \\left|\\mathit{x}\\right|",
    ),
    (
        call(
            AstKind::FunctionArccos,
            "arccos",
            &[call(AstKind::Name, "x", &[])],
        ),
        "acos(x)",
        " \\mathrm{arccos}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionArccosh,
            "arccosh",
            &[call(AstKind::Name, "x", &[])],
        ),
        "arccosh(x)",
        " \\mathrm{arccosh}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionArccot,
            "arccot",
            &[call(AstKind::Name, "x", &[])],
        ),
        "arccot(x)",
        " \\mathrm{arccot}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionArccoth,
            "arccoth",
            &[call(AstKind::Name, "x", &[])],
        ),
        "arccoth(x)",
        " \\mathrm{arccoth}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionArccsc,
            "arccsc",
            &[call(AstKind::Name, "x", &[])],
        ),
        "arccsc(x)",
        " \\mathrm{arccsc}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionArccsch,
            "arccsch",
            &[call(AstKind::Name, "x", &[])],
        ),
        "arccsch(x)",
        " \\mathrm{arccsch}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionArcsec,
            "arcsec",
            &[call(AstKind::Name, "x", &[])],
        ),
        "arcsec(x)",
        " \\mathrm{arcsec}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionArcsech,
            "arcsech",
            &[call(AstKind::Name, "x", &[])],
        ),
        "arcsech(x)",
        " \\mathrm{arcsech}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionArcsin,
            "arcsin",
            &[call(AstKind::Name, "x", &[])],
        ),
        "asin(x)",
        " \\mathrm{arcsin}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionArcsinh,
            "arcsinh",
            &[call(AstKind::Name, "x", &[])],
        ),
        "arcsinh(x)",
        " \\mathrm{arcsinh}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionArctan,
            "arctan",
            &[call(AstKind::Name, "x", &[])],
        ),
        "atan(x)",
        " \\mathrm{arctan}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionArctanh,
            "arctanh",
            &[call(AstKind::Name, "x", &[])],
        ),
        "arctanh(x)",
        " \\mathrm{arctanh}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionCeiling,
            "ceiling",
            &[call(AstKind::Name, "x", &[])],
        ),
        "ceil(x)",
        " \\lceil \\mathit{x}\\rceil ",
    ),
    (
        call(
            AstKind::FunctionCos,
            "cos",
            &[call(AstKind::Name, "x", &[])],
        ),
        "cos(x)",
        " \\mathrm{cos}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionCosh,
            "cosh",
            &[call(AstKind::Name, "x", &[])],
        ),
        "cosh(x)",
        " \\mathrm{cosh}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionCot,
            "cot",
            &[call(AstKind::Name, "x", &[])],
        ),
        "cot(x)",
        " \\mathrm{cot}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionCoth,
            "coth",
            &[call(AstKind::Name, "x", &[])],
        ),
        "coth(x)",
        " \\mathrm{coth}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionCsc,
            "csc",
            &[call(AstKind::Name, "x", &[])],
        ),
        "csc(x)",
        " \\mathrm{csc}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionCsch,
            "csch",
            &[call(AstKind::Name, "x", &[])],
        ),
        "csch(x)",
        " \\mathrm{csch}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionDelay,
            "delay",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "delay(x, y)",
        " \\mathrm{delay}\\left(\\mathit{x},\\mathit{y}\\right)",
    ),
    (
        call(
            AstKind::FunctionExp,
            "exp",
            &[call(AstKind::Name, "x", &[])],
        ),
        "exp(x)",
        " {e}^{\\mathit{x}}",
    ),
    (
        call(
            AstKind::FunctionFactorial,
            "factorial",
            &[call(AstKind::Name, "x", &[])],
        ),
        "factorial(x)",
        " \\mathit{x}!",
    ),
    (
        call(
            AstKind::FunctionFloor,
            "floor",
            &[call(AstKind::Name, "x", &[])],
        ),
        "floor(x)",
        " \\lfloor \\mathit{x}\\rfloor ",
    ),
    (
        call(AstKind::FunctionLn, "ln", &[call(AstKind::Name, "x", &[])]),
        "ln(x)",
        " \\mathrm{ln}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionLog,
            "log",
            &[
                number(AstKind::Integer, AstNumber::Integer(2)),
                call(AstKind::Name, "x", &[]),
            ],
        ),
        "log(2, x)",
        " {\\mathrm{log}}_{2}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionLog,
            "log",
            &[
                number(AstKind::Integer, AstNumber::Integer(10)),
                call(AstKind::Name, "x", &[]),
            ],
        ),
        "log10(x)",
        " \\mathrm{log}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionLog,
            "log",
            &[call(AstKind::Name, "x", &[])],
        ),
        "log(x)",
        " \\mathrm{log}",
    ),
    (
        call(
            AstKind::FunctionMax,
            "max",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "max(x, y)",
        " \\mathrm{max}(\\mathit{x},\\mathit{y})",
    ),
    (
        call(
            AstKind::FunctionMin,
            "min",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "min(x, y)",
        " \\mathrm{min}(\\mathit{x},\\mathit{y})",
    ),
    (
        call(
            AstKind::FunctionPiecewise,
            "piecewise",
            &[
                call(AstKind::Name, "x", &[]),
                call(AstKind::Name, "y", &[]),
                call(AstKind::Name, "z", &[]),
            ],
        ),
        "piecewise(x, y, z)",
        " \\begin{cases} \\mathit{x}& \\text{  if  } & \\mathit{y}\\\\ \\mathit{z}& {\\text{  otherwise}}\\end{cases}",
    ),
    (
        call(
            AstKind::FunctionPower,
            "power",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "x^y",
        " {\\mathit{x}}^{\\mathit{y}}",
    ),
    (
        call(
            AstKind::FunctionQuotient,
            "quotient",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "quotient(x, y)",
        " \\lfloor \\mathit{x}/\\mathit{y}\\rfloor ",
    ),
    (
        call(
            AstKind::FunctionRateOf,
            "rateOf",
            &[call(AstKind::Name, "x", &[])],
        ),
        "rateOf(x)",
        " \\mathrm{rateOf}\\left(\\mathit{x}\\right)",
    ),
    (
        call(
            AstKind::FunctionRem,
            "rem",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "rem(x, y)",
        " \\mathrm{rem}(\\mathit{x},\\mathit{y})",
    ),
    (
        call(
            AstKind::FunctionRoot,
            "root",
            &[
                number(AstKind::Integer, AstNumber::Integer(3)),
                call(AstKind::Name, "x", &[]),
            ],
        ),
        "root(3, x)",
        " \\sqrt[3]{\\mathit{x}}",
    ),
    (
        call(
            AstKind::FunctionRoot,
            "root",
            &[
                number(AstKind::Integer, AstNumber::Integer(2)),
                call(AstKind::Name, "x", &[]),
            ],
        ),
        "sqrt(x)",
        " \\sqrt{\\mathit{x}}",
    ),
    (
        call(
            AstKind::FunctionRoot,
            "root",
            &[call(AstKind::Name, "x", &[])],
        ),
        "root(x)",
        " \\sqrt{\\mathit{x}}",
    ),
    (
        call(
            AstKind::FunctionSec,
            "sec",
            &[call(AstKind::Name, "x", &[])],
        ),
        "sec(x)",
        " \\mathrm{sec}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionSech,
            "sech",
            &[call(AstKind::Name, "x", &[])],
        ),
        "sech(x)",
        " \\mathrm{sech}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionSin,
            "sin",
            &[call(AstKind::Name, "x", &[])],
        ),
        "sin(x)",
        " \\mathrm{sin}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionSinh,
            "sinh",
            &[call(AstKind::Name, "x", &[])],
        ),
        "sinh(x)",
        " \\mathrm{sinh}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionTan,
            "tan",
            &[call(AstKind::Name, "x", &[])],
        ),
        "tan(x)",
        " \\mathrm{tan}, \\mathit{x}",
    ),
    (
        call(
            AstKind::FunctionTanh,
            "tanh",
            &[call(AstKind::Name, "x", &[])],
        ),
        "tanh(x)",
        " \\mathrm{tanh}, \\mathit{x}",
    ),
    (
        call(
            AstKind::Lambda,
            "lambda",
            &[
                call(AstKind::Name, "x", &[]),
                call(AstKind::Name, "y", &[]),
                call(AstKind::Name, "z", &[]),
            ],
        ),
        "lambda(x, y, z)",
        " \\lambda(\\mathit{x}, \\mathit{y}) =\\left(\\mathit{z}\\right)",
    ),
    (
        call(
            AstKind::RelationalEq,
            "eq",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "x == y",
        " \\mathit{x}=\\mathit{y}",
    ),
    (
        call(
            AstKind::RelationalGeq,
            "geq",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "x >= y",
        " \\mathit{x}\\ge \\mathit{y}",
    ),
    (
        call(
            AstKind::RelationalGt,
            "gt",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "x > y",
        " \\mathit{x}>\\mathit{y}",
    ),
    (
        call(
            AstKind::RelationalLeq,
            "leq",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "x <= y",
        " \\mathit{x}\\le \\mathit{y}",
    ),
    (
        call(
            AstKind::RelationalLt,
            "lt",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "x < y",
        " \\mathit{x}<\\mathit{y}",
    ),
    (
        call(
            AstKind::RelationalNeq,
            "neq",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "x != y",
        " \\mathit{x}\\ne \\mathit{y}",
    ),
    (
        call(
            AstKind::LogicalAnd,
            "and",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "x && y",
        " \\mathit{x}\\wedge \\mathit{y}",
    ),
    (
        call(AstKind::LogicalNot, "not", &[call(AstKind::Name, "x", &[])]),
        "!x",
        " \\neg \\mathit{x}",
    ),
    (
        call(
            AstKind::LogicalOr,
            "or",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "x || y",
        " \\mathit{x}\\vee \\mathit{y}",
    ),
    (
        call(
            AstKind::LogicalXor,
            "xor",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "xor(x, y)",
        " \\mathit{x}\\oplus \\mathit{y}",
    ),
    (
        call(
            AstKind::LogicalImplies,
            "implies",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "implies(x, y)",
        " \\mathit{x}\\Rightarrow \\mathit{y}",
    ),
    (
        call(
            AstKind::DistribBernoulli,
            "bernoulli",
            &[call(AstKind::Name, "x", &[])],
        ),
        "bernoulli(x)",
        " \\mathrm{bernoulli}\\left(\\mathit{x}\\right)",
    ),
    (
        call(
            AstKind::DistribBinomial,
            "binomial",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "binomial(x, y)",
        " \\mathrm{binomial}\\left(\\mathit{x},\\mathit{y}\\right)",
    ),
    (
        call(
            AstKind::DistribCauchy,
            "cauchy",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "cauchy(x, y)",
        " \\mathrm{cauchy}\\left(\\mathit{x},\\mathit{y}\\right)",
    ),
    (
        call(
            AstKind::DistribChisquare,
            "chisquare",
            &[call(AstKind::Name, "x", &[])],
        ),
        "chisquare(x)",
        " \\mathrm{chisquare}\\left(\\mathit{x}\\right)",
    ),
    (
        call(
            AstKind::DistribExponential,
            "exponential",
            &[call(AstKind::Name, "x", &[])],
        ),
        "exponential(x)",
        " \\mathrm{exponential}\\left(\\mathit{x}\\right)",
    ),
    (
        call(
            AstKind::DistribGamma,
            "gamma",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "gamma(x, y)",
        " \\mathrm{gamma}\\left(\\mathit{x},\\mathit{y}\\right)",
    ),
    (
        call(
            AstKind::DistribLaplace,
            "laplace",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "laplace(x, y)",
        " \\mathrm{laplace}\\left(\\mathit{x},\\mathit{y}\\right)",
    ),
    (
        call(
            AstKind::DistribLognormal,
            "lognormal",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "lognormal(x, y)",
        " \\mathrm{lognormal}\\left(\\mathit{x},\\mathit{y}\\right)",
    ),
    (
        call(
            AstKind::DistribNormal,
            "normal",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "normal(x, y)",
        " \\mathrm{normal}\\left(\\mathit{x},\\mathit{y}\\right)",
    ),
    (
        call(
            AstKind::DistribPoisson,
            "poisson",
            &[call(AstKind::Name, "x", &[])],
        ),
        "poisson(x)",
        " \\mathrm{poisson}\\left(\\mathit{x}\\right)",
    ),
    (
        call(
            AstKind::DistribRayleigh,
            "rayleigh",
            &[call(AstKind::Name, "x", &[])],
        ),
        "rayleigh(x)",
        " \\mathrm{rayleigh}\\left(\\mathit{x}\\right)",
    ),
    (
        call(
            AstKind::DistribUniform,
            "uniform",
            &[call(AstKind::Name, "x", &[]), call(AstKind::Name, "y", &[])],
        ),
        "uniform(x, y)",
        " \\mathrm{uniform}\\left(\\mathit{x},\\mathit{y}\\right)",
    ),
];

#[test]
fn every_kind_writes_the_formula_and_the_latex_of_the_python_implementation() {
    let mut differences = Vec::new();
    for (node, expected_formula, expected_latex) in CASES {
        let ast = to_ast(node);
        match formula(&ast) {
            Ok(ours) if ours == *expected_formula => {}
            Ok(ours) => differences.push(format!(
                "{:?}: formula {ours:?} but python {expected_formula:?}",
                node.kind
            )),
            Err(error) => differences.push(format!(
                "{:?}: formula {error} but python {expected_formula:?}",
                node.kind
            )),
        }
        match latex(&ast) {
            Ok(ours) if ours == *expected_latex => {}
            Ok(ours) => differences.push(format!(
                "{:?}: latex {ours:?} but python {expected_latex:?}",
                node.kind
            )),
            Err(error) => differences.push(format!(
                "{:?}: latex {error} but python {expected_latex:?}",
                node.kind
            )),
        }
    }
    assert!(
        differences.is_empty(),
        "{} of the {} rows of the kind table write something else than python:\n{}",
        differences.len(),
        CASES.len(),
        differences.join("\n")
    );
}

#[test]
fn the_kind_table_holds_every_kind_the_shim_maps() {
    // `Other` is the kind of a type the shim does not name, which both writers
    // refuse rather than write, so it has no row
    for kind in AstKind::ALL {
        if kind == AstKind::Other {
            continue;
        }
        assert!(
            CASES.iter().any(|(node, _, _)| node.kind == kind),
            "{kind:?} has no row in the kind table"
        );
    }
}
