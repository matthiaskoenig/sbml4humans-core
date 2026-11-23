//! The latex of a syntax tree.
//!
//! The python implementation writes no latex of its own
//! (`backend/sbml4humans/mathml.py`): it writes the content MathML of the tree
//! with `libsbml.writeMathMLToString`, converts it to presentation MathML with
//! `ctopff.xsl`, converts that to latex with the `xsltml` stylesheet set, and
//! then runs ten string replacements and two heuristics over the whole result.
//! This module is a port of that pipeline, in the same three steps:
//!
//! 1. [`emit`] writes what the two stylesheets write of a tree, `$ ` and `$`
//!    around it included. It follows the shape of the stylesheets rather than
//!    the shape of a latex writer: the parameter `p` is the `$p` of ctopff, the
//!    precedence of the parent which decides a parenthesis, and [`open`] is the
//!    `\left`/`\right` rule of `tokens.xsl`, which depends on whether another
//!    operator stands between a pair of parentheses.
//! 2. [`replacements`] is `cmathml_to_latex` of the python module, verbatim and
//!    in its order. The replacements are global: `}\mathit` becomes
//!    `}, \mathit` **wherever** it stands, not only in the lambda it was written
//!    for, so `sin(x)` of a plain name is ` \mathrm{sin}, \mathit{x}`. Running
//!    them over the output of step 1, rather than writing their effect into the
//!    writer, is what reproduces those accidents without guessing where they
//!    fire.
//! 3. [`fix_mathit_symbols`] is `_fix_mathit_symbols`: a name of one underscore
//!    between two alphanumeric runs becomes a subscript, and a name which is one
//!    of the greek words becomes its command.
//!
//! The writer therefore has to answer one question only: what the stylesheets
//! write. Two things of libsbml are part of that answer, since the stylesheets
//! read the MathML libsbml writes and not the tree:
//!
//! - The text of a number is the one `writeCN` of libsbml writes, which is the
//!   `"%.15g"` of its value and, where that carries an exponent, a mantissa and
//!   an exponent apart ([`number_texts`]).
//! - The children of an `apply` are the ones `writeOperatorArgs` writes, which
//!   for an operator are its left and its right child alone and for a plus or a
//!   times of at most two children unrolls a child of the same kind
//!   ([`operands`]).
//!
//! The name of a node is read for a symbol, for a call of a function of the
//! model and for a csymbol, which are the three the MathML carries the text of:
//! a csymbol is typeset by its text, so a document which spells the time
//! csymbol `t` is written ` t` and not ` \mathrm{time}`. Every other node is
//! written by its kind.
//!
//! Nothing here panics on any tree: a child a node lacks is written as the empty
//! string, which is what the stylesheets write of a missing element, and a tree
//! past [`MAX_AST_DEPTH`] is [`MathError::TooDeep`].
//!
//! The two maths the python implementation writes no latex of at all are here as
//! well, since both are questions about the mathml libsbml writes and not about
//! the latex: [`holds_a_childless_lambda`] is the shape its writer dies on and
//! [`mathml_depth`] is the depth [`MAX_MATHML_DEPTH`] bounds. `math` of the
//! parent module asks them before it asks for a latex.
//!
//! # The stack
//!
//! A math may be nested to that bound and the walk follows it on the stack, so
//! every function of the walk writes into one string rather than building its
//! own and hands each construct to a function of its own, and the string value
//! of an element is produced with a stack of its own ([`StringValue`]) rather
//! than by a second recursion. The two chain tests of `tests/latex.rs` carry the
//! deepest tree the bound allows; the stack a chain of a thousand levels needs,
//! measured by bisecting the size of the thread it is written on, is
//!
//! | shape | stack |
//! | --- | --- |
//! | `minus(chain, plus(a, b))`, which compares the string values at every level | 1,401 KiB |
//! | a chain of `abs`, which is the plain walk | 1,369 KiB |
//! | a chain of `plus`, which is unrolled into one operator | 415 KiB |
//!
//! **Those are a debug native build on the 2 MiB stack a test thread gets.**
//! The target this constraint exists for is `wasm32-unknown-unknown`, whose
//! stack is 1 MiB (`stack-size=1048576` in its target spec) and which traps on
//! an overflow. **1,401 KiB does not fit in 1 MiB**, so a wasm build of these
//! frame sizes would trap on the deepest tree the bound allows **where [`latex`]
//! is called for it**. `math` of the parent module no longer is: it asks
//! [`mathml_depth`] first, and the mathml of a chain of a thousand quotients is
//! a thousand elements, far past [`MAX_MATHML_DEPTH`], so the python
//! implementation writes no latex for such a tree and neither does this one.
//! What a tree at the cap costs through `math`, measured the same way, is
//!
//! | shape | stack | is a latex written |
//! | --- | --- | --- |
//! | 253 quotients over a chain of sums, the deepest mathml the bound allows | 704 KiB | yes |
//! | a chain of a thousand sums or products, unrolled into one operator | 512 KiB | yes |
//! | a chain of a thousand quotients, or of differences, at 1,408 KiB through [`latex`] | 384 KiB | no |
//! | a chain of a thousand of `abs`, of a piecewise or of a lambda | 320 KiB | no |
//!
//! so the deepest latex walk which is still reachable fits, with room to spare,
//! in the megabyte of wasm in a debug build. A release build has smaller frames
//! than a debug one, by how much nobody has measured, and nothing in the
//! workspace builds or runs wasm yet, so these are still not wasm
//! measurements: a slice which raises [`MAX_MATHML_DEPTH`], lets a caller reach
//! [`latex`] without the depth rule, or runs on a smaller stack has to measure
//! again. The two tests of the parent module which carry a tree at the cap run
//! on a thread pinned to that megabyte, so a change which grows a frame of the
//! walk is a red run and not a surprise on the target.
//!
//! # The string value
//!
//! [`StringValue`] emulates the XPath string value of the mathml libsbml writes,
//! the indentation included. It was checked against `lxml`'s `itertext()` over
//! the mathml of all 1,832 maths of the corpus in the fixture while this was
//! written: 0 differed. That check is not committed, and of the committed tests
//! the gate of `crates/oracles/tests/math.rs` covers it only in part: 11 of
//! those 1,832 hold a difference whose right operand asks for the comparison and in
//! all 11 the two values differ, so **no entry of the corpus covers the case
//! where they are equal**. That case is covered by
//! `the_right_operand_of_a_difference_is_parenthesised_by_its_text` of
//! `crates/report/tests/latex.rs` alone, whose two expectations were probed
//! against python.

use super::MathError;
use sbml4humans_model::{Ast, AstKind, AstNumber, MAX_AST_DEPTH};
use std::borrow::Cow;

/// The latex of `ast`, as the python implementation writes it.
///
/// The string begins with the space which is left where the `$` of the inline
/// math of `mmltex.xsl` was removed, which every latex of the report carries.
///
/// # Errors
///
/// [`MathError::TooDeep`] for a tree nested deeper than [`MAX_AST_DEPTH`] and
/// [`MathError::UnknownKind`] for a tree which holds a node of a kind the
/// libsbml shim does not map, whose latex would not be the one of the document.
pub fn latex(ast: &Ast) -> Result<String, MathError> {
    let mut out = String::from("$ ");
    emit(ast, 0, None, 0, 1, &mut out)?;
    out.push('$');
    Ok(fix_mathit_symbols(&replacements(&out)))
}

/* ------------------------------------------------------- the replacements */

/// `cmathml_to_latex` of the python module: its ten replacements, in its order.
///
/// The order is part of the result. The three replacements of the piecewise run
/// before the three of the lambda, so the `{ccc}` of the column alignment of the
/// array is gone by the time `}\mathit` is replaced and never becomes
/// `{ccc}, \mathit`; and the two which strip a `\left(`/`\right)` pair around a
/// piecewise run before the two which do not, so a piecewise which stands as the
/// body of a lambda loses the parentheses of the `mfenced` around it.
fn replacements(text: &str) -> String {
    text.replace('$', "")
        .replace("\\hfill", "")
        .replace("\\multicolumn{2}{c}", "")
        .replace("\\left(\\{\\begin{array}{ccc}", "\\begin{cases} ")
        .replace("\\end{array}\\right)", "\\end{cases}")
        .replace("\\{\\begin{array}{ccc}", "\\begin{cases} ")
        .replace("\\end{array}", "\\end{cases}")
        .replace("}\\mathit", "}, \\mathit")
        .replace("\\lambda ", "\\lambda(")
        .replace("}.", "}) =")
}

/// The greek words a name is typeset as a command for, which `mathml.py` lists.
///
/// The small lambda is not among them: it marks the definition of a function.
const GREEK_SYMBOLS: [&str; 29] = [
    "alpha", "beta", "gamma", "Gamma", "delta", "Delta", "epsilon", "zeta", "eta", "theta", "iota",
    "kappa", "Lambda", "mu", "nu", "omicron", "pi", "rho", "sigma", "tau", "upsilon", "Upsilon",
    "phi", "Phi", "chi", "psi", "Psi", "omega", "Omega",
];

/// `_fix_mathit_symbols` of the python module: the subscripts and the greek words.
///
/// A `\mathit{a\_b}` of one escaped underscore between two alphanumeric runs
/// becomes `\mathit{a_{b}}`, so `kf_0` reads as a subscript while `LI__pattern`,
/// which holds two, keeps its escaped underscores. A `\mathit{alpha}` becomes
/// `\mathit{\alpha}` afterwards, so a name which was turned into a subscript is
/// no longer a candidate. The python implementation replaces each name it found
/// everywhere in the string at once, which is the same result as walking the
/// names one by one: the two rules are mutually exclusive and neither makes a
/// name the other matches.
fn fix_mathit_symbols(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("\\mathit{") {
        let (before, from) = rest.split_at(start);
        out.push_str(before);
        let body = from.get("\\mathit{".len()..).unwrap_or_default();
        match body.find('}') {
            Some(end) => {
                let name = body.get(..end).unwrap_or_default();
                out.push_str("\\mathit{");
                out.push_str(&fix_name(name));
                out.push('}');
                rest = body.get(end + 1..).unwrap_or_default();
            }
            // a `\mathit{` which is never closed, which no token of the writer
            // produces: nothing of it is a name, so it is copied as it stands
            None => {
                out.push_str(from);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// The body of a `\mathit{...}` after the two heuristics of `_fix_mathit_symbols`.
fn fix_name(name: &str) -> String {
    if let Some((head, tail)) = name.split_once("\\_")
        && !head.is_empty()
        && !tail.is_empty()
        && head.chars().all(|c| c.is_ascii_alphanumeric())
        && tail.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return format!("{head}_{{{tail}}}");
    }
    if GREEK_SYMBOLS.contains(&name) {
        return format!("\\{name}");
    }
    name.to_owned()
}

/* ------------------------------------------------------------- the tokens */

/// Writes the text of a token element: `normalize-space` and then `replaceEntities`.
///
/// `entities.xsl` maps a few hundred characters to a command. Mapped here are
/// the ascii characters latex gives a meaning and the two characters ctopff
/// writes of a constant; every other character is written as it stands.
///
/// **This is a known non-parity case**, for the text of any token which is not
/// ascii, a `ci` as much as a csymbol: python answers ` \mathit{\alpha }` for a
/// `ci` whose text is one greek letter and this answers the letter itself. The
/// SId of a valid document is ascii, but `read_math` enforces no SId syntax and
/// a report of an invalid document is the point of the project. Closing it
/// means porting the table of `entities.xsl`, which no document of the corpus
/// needs.
fn write_token(text: &str, out: &mut String) {
    let mut first = true;
    for word in text.split_whitespace() {
        if !first {
            out.push_str("\\; ");
        }
        first = false;
        for character in word.chars() {
            match character {
                '{' | '}' | '$' | '#' | '&' | '%' | '_' => {
                    out.push('\\');
                    out.push(character);
                }
                '\\' => out.push_str("\\backslash "),
                '\'' => out.push_str("\\text{'}"),
                '\u{03c0}' => out.push_str("\\pi "),
                '\u{221e}' => out.push_str("\\infty "),
                _ => out.push(character),
            }
        }
    }
}

/// Writes an `mi` without a `mathvariant`, which `tokens.xsl` sets upright where
/// its text is longer than one character.
fn write_mi(text: &str, out: &mut String) {
    if text
        .split_whitespace()
        .flat_map(str::chars)
        .nth(1)
        .is_some()
    {
        out.push_str("\\mathrm{");
        write_token(text, out);
        out.push('}');
    } else {
        write_token(text, out);
    }
}

/// Writes an `mi mathvariant="italic"`, which is what ctopff writes of a `ci`.
fn write_mi_italic(text: &str, out: &mut String) {
    out.push_str("\\mathit{");
    write_token(text, out);
    out.push('}');
}

/// Writes an `mn`, which `tokens.xsl` sets upright where its text is no number.
fn write_mn(text: &str, out: &mut String) {
    if is_number(text) {
        write_token(text, out);
    } else {
        out.push_str("\\mathrm{");
        write_token(text, out);
        out.push('}');
    }
}

/// Whether `string(number(text))` of XPath is not `NaN`, which the `mn` template asks.
fn is_number(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty() && trimmed.parse::<f64>().is_ok_and(|value| !value.is_nan())
}

/* ------------------------------------------------------------ the numbers */

/// The text pieces of the `cn` libsbml writes for a number node.
///
/// One piece for an integer and for a real which needs no exponent, two for a
/// rational and for a real in exponent form, which libsbml separates with a
/// `sep` element. A real whose `"%.15g"` carries an exponent is written in
/// exponent form as well, which is why the kind of the node does not decide this
/// on its own.
fn number_texts(node: &Ast) -> (String, Option<String>) {
    match node.number {
        Some(AstNumber::Integer(integer)) => (integer.to_string(), None),
        Some(AstNumber::Rational {
            numerator,
            denominator,
        }) => (numerator.to_string(), Some(denominator.to_string())),
        Some(AstNumber::RealE { mantissa, exponent }) => exponent_texts(mantissa, exponent),
        Some(AstNumber::Real(real)) => real_texts(real),
        // libsbml holds the value of a number node which was never given one as
        // the real zero
        None => real_texts(0.0),
    }
}

/// The text pieces of `writeDouble` of libsbml for `value`.
fn real_texts(value: f64) -> (String, Option<String>) {
    let text = significant_15(value);
    match text.split_once('e') {
        Some((mantissa, exponent)) => exponent_texts(
            mantissa.parse::<f64>().unwrap_or_default(),
            exponent.parse::<i64>().unwrap_or_default(),
        ),
        None => (text, None),
    }
}

/// The two text pieces of `writeENotation` of libsbml for a mantissa and an exponent.
///
/// A mantissa whose own `"%.15g"` carries an exponent adds it to the exponent of
/// the node, which is the special case the function of libsbml exists for.
fn exponent_texts(mantissa: f64, exponent: i64) -> (String, Option<String>) {
    let text = significant_15(mantissa);
    match text.split_once('e') {
        Some((head, tail)) => (
            head.to_owned(),
            Some(
                exponent
                    .saturating_add(tail.parse::<i64>().unwrap_or_default())
                    .to_string(),
            ),
        ),
        None => (text, Some(exponent.to_string())),
    }
}

/// `value` as the `"%.15g"` of C writes it, the float format of libsbml.
fn significant_15(value: f64) -> String {
    super::formula::significant_15(value)
}

/// `string(number)` of XPath, as libxml2 writes it (`xmlXPathFormatNumber`).
///
/// This is the one place a number is not written by libsbml: the plus template
/// of ctopff negates a `cn` which is below zero with `-(.)`, and the string of
/// that number is written by the XPath engine. libxml2 writes a value which is
/// an `int` as that integer, a value beyond a thousand million or below a
/// hundred thousandth in exponent notation with fifteen significant digits, and
/// every other value as a fixed decimal of fifteen significant digits; in each
/// case the trailing zeros of the fraction, and a bare point, are removed.
fn xpath_number(value: f64) -> String {
    /// The significant digits libxml2 writes, its `DBL_DIG`.
    const DIGITS: i32 = 15;
    /// Above this a value is written in exponent notation, its `UPPER_DOUBLE`.
    const UPPER: f64 = 1e9;
    /// Below this a value is written in exponent notation, its `LOWER_DOUBLE`.
    const LOWER: f64 = 1e-5;

    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value < 0.0 { "-Infinity" } else { "Infinity" }.to_owned();
    }
    // `xmlXPathFormatNumber` writes an integer only for a value strictly within
    // the range of an `int`, so the two ends of that range are written by the
    // branch below rather than as themselves
    if value > f64::from(i32::MIN)
        && value < f64::from(i32::MAX)
        && value == f64::from(value as i32)
    {
        return (value as i32).to_string();
    }
    let magnitude = value.abs();
    if magnitude > UPPER || magnitude < LOWER {
        let scientific = format!("{:.*e}", (DIGITS - 1) as usize, value);
        let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
        let exponent: i64 = exponent.parse().unwrap_or_default();
        let sign = if exponent < 0 { '-' } else { '+' };
        return format!(
            "{}e{sign}{:02}",
            without_trailing_zeros(mantissa),
            exponent.unsigned_abs()
        );
    }
    let integer_place = magnitude.log10() as i32;
    let fraction_place = if integer_place > 0 {
        DIGITS - integer_place - 1
    } else {
        DIGITS - integer_place
    };
    let fixed = format!("{value:.*}", usize::try_from(fraction_place).unwrap_or(0));
    without_trailing_zeros(&fixed).to_owned()
}

/// `formatted` without the trailing zeros of its fraction and without a bare point.
fn without_trailing_zeros(formatted: &str) -> &str {
    if formatted.contains('.') {
        formatted.trim_end_matches('0').trim_end_matches('.')
    } else {
        formatted
    }
}

/* --------------------------------------------------- the mathml of a node */

/// The operand elements libsbml writes under the `apply` of `node`.
///
/// `writeOperatorArgs` writes the left and the right child of an operator and
/// nothing between them, so a difference, a quotient or a power of three
/// children loses its middle one; and for a plus or a times of at most two
/// children it unrolls a child of the same kind, so `a + (b + c)` is written as
/// one sum of three operands. `writeFunctionLog` writes the base of a logarithm
/// in a `logbase` and `writeFunctionRoot` the degree of a root in a `degree`,
/// both of them only where the node carries more than one child, and then the
/// right child as the argument: a logarithm of one child carries no argument at
/// all, and a root of one writes it without a degree. Every other node is
/// written with all its children.
///
/// A child of the kind [`AstKind::Unknown`] is written as no element at all, so
/// it is no operand, and the construct above it writes the shape of one operand
/// fewer: a quotient of a known and an unknown child is `\frac{a}{}` whichever
/// of the two is unknown, a difference of them is a negation, and an absolute
/// value of one is a pair of bars around nothing. It is dropped after the
/// positions are read, since libsbml picks the left and the right child of an
/// operator by their place in the tree and not by what they are.
///
/// The logarithm and the root are the two constructs this is not faithful to,
/// and both only for a tree which holds an unknown node, which no valid
/// document does. libsbml writes the first child of either in a `logbase` or a
/// `degree` element which it writes whether or not that child writes anything,
/// so the operand is not gone there but empty, and the stylesheets then write
/// what they write of an element they did not expect: measured against 5.21.2
/// and the python implementation, `log(unknown, a)` is
/// ` {\mathrm{log}}_{}, \mathit{a}`, `log(3, unknown)` writes the base twice as
/// ` {\mathrm{log}}_{3}3`, `root(unknown, a)` is ` \sqrt[]{\mathit{a}}` and
/// `root(3, unknown)` is the literal ` \text{exception 25:}` of the error branch
/// of `ctopff.xsl`. Reaching those four needs the two wrappers in the model of
/// this writer, and the fourth of them is python writing no math at all.
///
/// # Errors
///
/// [`MathError::TooDeep`] where the unrolling would follow a chain of sums or
/// products deeper than [`MAX_AST_DEPTH`].
fn operands(node: &Ast, depth: usize) -> Result<Vec<&Ast>, MathError> {
    if depth > MAX_AST_DEPTH {
        return Err(MathError::TooDeep);
    }
    let children = &node.children;
    let written = match node.kind {
        AstKind::Plus | AstKind::Times if children.len() <= 2 => {
            let mut out = Vec::new();
            for child in children {
                if child.kind == node.kind {
                    out.append(&mut operands(child, depth + 1)?);
                } else {
                    out.push(child);
                }
            }
            out
        }
        AstKind::Minus | AstKind::Divide | AstKind::Power | AstKind::FunctionRoot => {
            first_and_last(children, true)
        }
        AstKind::FunctionLog => first_and_last(children, false),
        _ => children.iter().collect(),
    };
    Ok(written
        .into_iter()
        .filter(|child| child.kind != AstKind::Unknown)
        .collect())
}

/// The first and the last of `children`, the first alone where there is one and
/// `alone` says libsbml writes it.
fn first_and_last(children: &[Ast], alone: bool) -> Vec<&Ast> {
    let mut out = Vec::new();
    if let Some(first) = children.first()
        && (children.len() > 1 || alone)
    {
        out.push(first);
    }
    if children.len() > 1
        && let Some(last) = children.last()
    {
        out.push(last);
    }
    out
}

/// Whether libsbml writes `node` as an `apply`, which several templates of ctopff test.
///
/// A trigonometric function and a logarithm parenthesise an argument which is an
/// `apply` and write one which is not without parentheses, and a power and an
/// exponential wrap themselves where their parent is a power and they carry one.
/// A real which is no number at all is written as an element of its own, and the
/// negative infinity as a negated one, which is the one number that is an apply.
fn is_apply(node: &Ast) -> bool {
    match node.kind {
        AstKind::Real | AstKind::RealE | AstKind::Rational => {
            special_real(node) == Some(SpecialReal::NegativeInfinity)
        }
        AstKind::Integer
        | AstKind::Name
        | AstKind::NameTime
        | AstKind::NameAvogadro
        | AstKind::ConstantE
        | AstKind::ConstantFalse
        | AstKind::ConstantPi
        | AstKind::ConstantTrue
        | AstKind::Lambda
        | AstKind::FunctionPiecewise
        | AstKind::Other => false,
        _ => true,
    }
}

/// A real libsbml writes as an element of its own rather than as a `cn`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpecialReal {
    /// `notanumber`.
    NotANumber,
    /// `infinity`.
    Infinity,
    /// `apply` of `minus` and `infinity`.
    NegativeInfinity,
}

/// Which element libsbml writes for `node`, [`None`] for every node it writes a `cn` for.
///
/// `writeCN` branches on the **value** of a node, not on its kind, and it does
/// so for every node `ASTNode::isReal()` covers, which is `AST_REAL`,
/// `AST_REAL_E` and `AST_RATIONAL`. So a rational of a zero denominator, which
/// plain content MathML can write, is an `infinity` or a `notanumber`, and a
/// number in exponent form whose value overflows is the apply of a minus and an
/// infinity. The one asymmetry is `writeCN`'s own: a node of the kind
/// `AST_REAL_E` whose value is the positive infinity is excluded from the
/// `infinity` branch and written as a `cn` all the same.
fn special_real(node: &Ast) -> Option<SpecialReal> {
    let value = real_value(node)?;
    if value.is_nan() {
        Some(SpecialReal::NotANumber)
    } else if value == f64::INFINITY && node.kind != AstKind::RealE {
        Some(SpecialReal::Infinity)
    } else if value == f64::NEG_INFINITY {
        Some(SpecialReal::NegativeInfinity)
    } else {
        None
    }
}

/// The value `ASTNode::getReal()` answers for `node`, [`None`] for a node
/// `ASTNode::isReal()` does not cover.
///
/// An integer is not among them, so `writeCN` always writes it as a `cn`.
fn real_value(node: &Ast) -> Option<f64> {
    if !matches!(
        node.kind,
        AstKind::Real | AstKind::RealE | AstKind::Rational
    ) {
        return None;
    }
    Some(match node.number {
        Some(AstNumber::Real(value)) => value,
        Some(AstNumber::RealE { mantissa, exponent }) => mantissa * 10f64.powf(exponent as f64),
        Some(AstNumber::Rational {
            numerator,
            denominator,
        }) => numerator as f64 / denominator as f64,
        // The shim tags the number by the AST type, so these two arms are
        // unreachable from it and defensive only: an integer answers itself
        // as a real, and a node with no number answers 0.0.
        Some(AstNumber::Integer(integer)) => integer as f64,
        None => 0.0,
    })
}

/* ------------------------------------------ the maths python writes no latex of */

/// The deepest mathml document the python implementation converts, the `<math>`
/// root counted as its first element.
///
/// `_convert_cmathml` of `mathml.py` parses the mathml libsbml wrote with
/// `lxml.etree.fromstring`, which refuses a document deeper than this with
/// `Excessive depth in document: 256, use XML_PARSE_HUGE option`; `math_info`
/// (`mathml.py:309-332`) catches that `LxmlError`, logs a warning and reports
/// the math without its latex, as its formula alone.
pub(super) const MAX_MATHML_DEPTH: usize = 256;

/// Whether `ast` holds a lambda without children anywhere in it.
///
/// `unwritable_mathml` of `mathml.py` (`mathml.py:194-223`): `writeLambda` of
/// libsbml reads the number of bound variables as the number of children less
/// one, which wraps to `UINT_MAX` for a childless node, and then dereferences
/// the null child it asks for, so `writeMathMLToString` ends the process with
/// SIGSEGV (sbmlteam/libsbml#521). The python implementation therefore never
/// hands such a tree to the writer and reports the math without its latex.
///
/// The shape can sit anywhere inside a larger math, so the whole tree is
/// walked, the children of a node of the kind [`AstKind::Unknown`] included:
/// the walk of the python guard reads the syntax tree and not what the writer
/// writes of it, and a lambda without children below an unknown node answers
/// the guard there as well (measured against 5.21.2).
///
/// The walk keeps its own stack, so a tree of any depth costs no call frames.
pub(super) fn holds_a_childless_lambda(ast: &Ast) -> bool {
    let mut stack = vec![ast];
    while let Some(node) = stack.pop() {
        if node.kind == AstKind::Lambda && node.children.is_empty() {
            return true;
        }
        stack.extend(node.children.iter());
    }
    false
}

/// How many elements deep the mathml document libsbml writes for `ast` is, the
/// `<math>` root counted as its first element.
///
/// The depth is read off the tree rather than by writing the mathml, so the
/// mapping from a node to the elements it writes is the whole of this function.
/// It is the mapping of `ASTNode::write` of libsbml, the elements it writes
/// above its children and the wrapper each child stands in:
///
/// The last column says how each row is known. **At the bound** is the strongest:
/// the shape was put at the bound of the pipeline and `sbml4humans.mathml.math_info`
/// was asked which side of it answers a latex, so the row is python's own answer.
/// **With lxml** is the depth of the mathml `writeMathMLToString` wrote for the
/// shape, read with lxml, without python's threshold behind it. Every row was
/// measured one way or the other against libsbml 5.21.2; nothing here is
/// reasoning about what libsbml ought to write.
///
/// | kind | elements | the branch below the top element | how it is known |
/// | --- | --- | --- | --- |
/// | [`AstKind::Unknown`] | none at all, nor anything of its children | - | at the bound |
/// | a `ci`, a `csymbol` or a constant | the one element | - | at the bound |
/// | an `Integer`, and a `Real` whose `"%.15g"` carries no exponent | the `cn` | - | at the bound |
/// | a `Rational`, and a `Real` or a `RealE` in exponent form | the `cn` and its `sep` | - | at the bound |
/// | a `Real` which is the positive infinity or no number | `infinity`, `notanumber` | - | at the bound |
/// | one which is the negative infinity | `apply`, `minus`, `infinity` | - | at the bound |
/// | a piecewise | the `piecewise` | every child in a `piece` or an `otherwise` | at the bound |
/// | a lambda | the `lambda` | every child but the last in a `bvar` | at the bound |
/// | a logarithm or a root of more than one child | the `apply` and its operator | the first child in a `logbase` or a `degree`, the last as the argument | at the bound |
/// | a logarithm of one child | the `apply` and its operator | the child is written as no element at all | with lxml |
/// | a root of one child | the `apply` and its operator | the child as the argument, without a `degree` | with lxml |
/// | every other node libsbml writes as an `apply` | the `apply` and its operator | the operands of [`operands`] | at the bound |
/// | an `apply` none of whose operands writes anything | the `apply` and its operator alone | - | with lxml |
///
/// A wrapper follows the position of a child and not what is written of it, so
/// an empty `piece`, `bvar`, `logbase` or `degree` is a level of its own, each
/// of which was measured with lxml and not at the bound; an `apply` writes its
/// operator element whatever its operands are, so it is two levels deep even
/// where it has none; and a `piecewise` of no children at all is one element.
/// The three rows which were not measured at the bound, and those three
/// wrappers around nothing, all need a node of the kind [`AstKind::Unknown`] or
/// a shape no valid document holds, and each is worth one level at most.
///
/// Measured against libsbml 5.21.2 and the python implementation at the bound,
/// shape by shape, in `tests::the_mathml_depth_of_a_shape_is_the_one_libsbml_writes`
/// of the parent module. It was also checked against the depth `lxml` reads of
/// the mathml of all 1,832 maths of the corpus in the fixture while this was
/// written: 0 differed, over documents of 2 to 15 elements. That check is not
/// committed, since the fixture carries no depth of its own, and no math of the
/// corpus comes anywhere near the bound. What guards the mapping is the table of
/// that test and the pair of entries the fixture carries at the bound itself, a
/// chain of 254 nested divides and one of 255, which python typesets the first of
/// and not the second (`the_fixture_holds_the_two_maths_at_the_bound_of_the_latex_pipeline`
/// of `crates/oracles/tests/math.rs`).
///
/// # Errors
///
/// [`MathError::TooDeep`] for a tree nested deeper than [`MAX_AST_DEPTH`],
/// which is the bound of every walk of this crate.
///
/// # The stack
///
/// The walk keeps its own stack on the heap, so a tree costs it no call frame
/// per level. The one recursion it reaches is the unrolling of [`operands`],
/// which follows a chain of sums or of products one frame per level and is
/// bounded by [`MAX_AST_DEPTH`] like every other walk here; the latex writer
/// makes the same call, so this adds no frame the writer did not already need.
/// Measured in a debug build, `math` of a chain of a thousand sums needs
/// 512 KiB and one of a thousand quotients 384 KiB, inside the megabyte
/// `wasm32-unknown-unknown` passes
/// (`tests::the_math_of_a_chain_at_the_cap_is_written_within_the_stack_of_wasm`
/// of the parent module, which runs on a thread pinned to that megabyte).
pub(super) fn mathml_depth(ast: &Ast) -> Result<usize, MathError> {
    let mut stack = vec![frame(0, ast, 1)?];
    // the depth of the elements of the node whose frame was last finished,
    // which for the root is the depth of the whole document below `<math>`
    let mut depth = 0;
    while let Some(top) = stack.last_mut() {
        if let Some(&(wrappers, child)) = top.children.get(top.next) {
            top.next += 1;
            if stack.len() >= MAX_AST_DEPTH {
                return Err(MathError::TooDeep);
            }
            let level = stack.len() + 1;
            stack.push(frame(wrappers, child, level)?);
            continue;
        }
        // every child of the node is walked, so the depth of its elements is
        // known and its frame is handed up to its parent
        let (own, deepest, wrappers) = (top.own, top.deepest, top.wrappers);
        let _ = stack.pop();
        depth = own + deepest;
        if let Some(parent) = stack.last_mut() {
            parent.deepest = parent.deepest.max(wrappers + depth);
        }
    }
    Ok(1 + depth)
}

/// One node of the walk of [`mathml_depth`], and what is known of it so far.
#[derive(Debug)]
struct Frame<'a> {
    /// The wrapper elements libsbml writes between the top element of the
    /// parent of the node and the top element of the node itself.
    wrappers: usize,
    /// The children libsbml writes below the node, each with its wrappers.
    children: Vec<(usize, &'a Ast)>,
    /// Which of them the walk takes next.
    next: usize,
    /// The elements the node writes above its children: one, or none for a node
    /// libsbml writes nothing of.
    own: usize,
    /// The deepest branch of the ones walked so far, below the top element.
    deepest: usize,
}

/// The frame of `node`, which stands below `wrappers` wrapper elements of its
/// parent and at `level` of the tree.
///
/// # Errors
///
/// [`MathError::TooDeep`] where the unrolling of [`operands`] would follow a
/// chain of sums or products deeper than [`MAX_AST_DEPTH`].
fn frame(wrappers: usize, node: &Ast, level: usize) -> Result<Frame<'_>, MathError> {
    // a child of the kind `Unknown` is walked like any other: it answers a
    // depth of zero, which leaves the wrapper it stands in as the only element
    // of its branch, and that is what libsbml writes of it
    let (own, deepest, children) = match node.kind {
        // libsbml writes no element of a math it could not read and none of
        // what stands below it: a chain of a thousand applies below such a node
        // adds nothing to the document (measured against 5.21.2)
        AstKind::Unknown => (0, 0, Vec::new()),
        // a token element, which carries text and no element
        AstKind::Name
        | AstKind::NameTime
        | AstKind::NameAvogadro
        | AstKind::Integer
        | AstKind::ConstantE
        | AstKind::ConstantFalse
        | AstKind::ConstantPi
        | AstKind::ConstantTrue
        // the writers refuse a tree which holds a node of a kind the shim does
        // not map before this is ever asked, so the arm is defensive only
        | AstKind::Other => (1, 0, Vec::new()),
        AstKind::Real | AstKind::RealE | AstKind::Rational => {
            let below = match special_real(node) {
                // `apply`, `minus` and `infinity`, which is two levels
                Some(SpecialReal::NegativeInfinity) => 1,
                // one `infinity` or one `notanumber`
                Some(SpecialReal::NotANumber | SpecialReal::Infinity) => 0,
                // a `cn`, with a `sep` below it in exponent form
                None => usize::from(number_texts(node).1.is_some()),
            };
            (1, below, Vec::new())
        }
        // every child of a piecewise stands in a `piece` or in an `otherwise`,
        // which libsbml writes by the position of the child; a piecewise of no
        // children at all is the one element
        AstKind::FunctionPiecewise => {
            (1, 0, node.children.iter().map(|child| (1, child)).collect())
        }
        // every child of a lambda but the last stands in a `bvar`
        AstKind::Lambda => {
            let last = node.children.len().saturating_sub(1);
            let children = node
                .children
                .iter()
                .enumerate()
                .map(|(index, child)| (usize::from(index < last), child))
                .collect();
            (1, 0, children)
        }
        // an `apply` writes its operator element whatever its operands are, so
        // the branch below it is never empty
        AstKind::FunctionLog | AstKind::FunctionRoot => (1, 1, logarithm_or_root(node)),
        _ => (
            1,
            1,
            operands(node, level)?
                .into_iter()
                .map(|child| (0, child))
                .collect(),
        ),
    };
    Ok(Frame {
        wrappers,
        children,
        next: 0,
        own,
        deepest,
    })
}

/// The children libsbml writes below the `apply` of a logarithm or of a root,
/// each with the wrapper element it stands in.
///
/// `writeFunctionLog` writes the base of a logarithm in a `logbase` and
/// `writeFunctionRoot` the degree of a root in a `degree`, each only where the
/// node carries more than one child, and then the last child as the argument;
/// so a logarithm of one child writes no argument at all, and a root of one
/// writes its child as the argument without a degree. The wrapper is written
/// whether or not the child inside it writes anything, which is where this
/// differs from [`operands`], whose caller reads the operands of the
/// stylesheets and not the elements of the document.
fn logarithm_or_root(node: &Ast) -> Vec<(usize, &Ast)> {
    let mut children = Vec::new();
    if node.children.len() > 1 {
        if let Some(first) = node.children.first() {
            children.push((1, first));
        }
        if let Some(last) = node.children.last() {
            children.push((0, last));
        }
    } else if node.kind == AstKind::FunctionRoot {
        children.extend(node.children.iter().map(|child| (0, child)));
    }
    children
}

/* ------------------------------------------- the string value of a node */

/// One piece of work left in the production of a string value.
#[derive(Debug)]
enum Step<'a> {
    /// The string value of this node, which stands at this level of the
    /// document and this deep in the tree.
    Node(&'a Ast, usize, usize),
    /// A newline and the indentation of this level.
    Indent(usize),
    /// This text.
    Text(Cow<'a, str>),
}

/// The piece being produced, and how many of its bytes are done.
#[derive(Debug)]
enum Current<'a> {
    /// Nothing is being produced.
    Nothing,
    /// A newline and the indentation of this level.
    Indent(usize, usize),
    /// This text.
    Text(Cow<'a, str>, usize),
}

/// The bytes of the XPath string value of the mathml element libsbml writes for
/// a node, produced one at a time.
///
/// The plus template and the binary template of ctopff ask `. != ../*[2]`, which
/// in XPath 1.0 compares the string values of two elements, to tell the right
/// operand of a difference from the left one. The string value of an element is
/// the text of everything below it, the indentation libsbml writes between its
/// elements included, so two operands which differ in their operators alone have
/// the same string value and the right one of them is written without the
/// parentheses it needs. That is what the python implementation writes, so it is
/// what this writes: `(a*b) - (a+b)` is
/// ` \mathit{a}\cdot \mathit{b}-\mathit{a}+\mathit{b}`.
///
/// The value is produced rather than built for two reasons, and both of them are
/// about a chain of differences, which asks for this comparison at every level
/// of itself:
///
/// - The indentation grows with the level of an element, so the string value of
///   a tree is quadratic in its depth on its own, and a comparison which built
///   both values whole would walk such a chain in the cube of its depth.
///   [`string_values_differ`] produces the two in step and stops at the first
///   byte which differs, which for a chain is within the first few.
/// - The production keeps its own stack rather than the call stack, so the
///   comparison recurses no further than the one `operands` call `expand` makes
///   for a chain of sums or products, whatever the depth the latex walk already
///   stands at.
///
/// `level` is how deep the element stands, which decides the indentation before
/// each of its children. The two operands of a difference are siblings and so
/// stand at the same level, and comparing their values at level zero answers the
/// comparison of their real ones.
#[derive(Debug)]
struct StringValue<'a> {
    /// What is left to produce, the next piece last.
    steps: Vec<Step<'a>>,
    /// The piece being produced.
    current: Current<'a>,
}

impl<'a> StringValue<'a> {
    /// The string value of `node`, which stands `depth` deep in the tree.
    fn of(node: &'a Ast, depth: usize) -> Self {
        Self {
            steps: vec![Step::Node(node, 0, depth)],
            current: Current::Nothing,
        }
    }

    /// The next byte of the value, [`None`] where it is complete.
    ///
    /// # Errors
    ///
    /// [`MathError::TooDeep`] for a subtree nested deeper than [`MAX_AST_DEPTH`].
    fn next_byte(&mut self) -> Result<Option<u8>, MathError> {
        loop {
            match &mut self.current {
                Current::Indent(level, at) => {
                    // a newline and two spaces per level
                    if *at <= 2 * *level {
                        let byte = if *at == 0 { b'\n' } else { b' ' };
                        *at += 1;
                        return Ok(Some(byte));
                    }
                }
                Current::Text(text, at) => {
                    if let Some(byte) = text.as_bytes().get(*at) {
                        *at += 1;
                        return Ok(Some(*byte));
                    }
                }
                Current::Nothing => {}
            }
            let Some(step) = self.steps.pop() else {
                return Ok(None);
            };
            match step {
                Step::Indent(level) => self.current = Current::Indent(level, 0),
                Step::Text(text) => self.current = Current::Text(text, 0),
                Step::Node(node, level, depth) => {
                    self.current = Current::Nothing;
                    self.expand(node, level, depth)?;
                }
            }
        }
    }

    /// Puts the pieces of the string value of `node` on the stack.
    fn expand(&mut self, node: &'a Ast, level: usize, depth: usize) -> Result<(), MathError> {
        if depth > MAX_AST_DEPTH {
            return Err(MathError::TooDeep);
        }
        let mut steps: Vec<Step<'a>> = Vec::new();
        match special_real(node) {
            // an empty element, which carries no text at all
            Some(SpecialReal::NotANumber | SpecialReal::Infinity) => {}
            // the apply of a minus and an infinity, which `writeCN` writes
            // with `setAutoIndent(false)` and a space around each of its two
            // empty elements, so its text is three spaces and no indentation
            Some(SpecialReal::NegativeInfinity) => steps.extend([
                Step::Text(Cow::Borrowed(" ")),
                Step::Text(Cow::Borrowed(" ")),
                Step::Text(Cow::Borrowed(" ")),
            ]),
            None => match node.kind {
                AstKind::Integer | AstKind::Real | AstKind::RealE | AstKind::Rational => {
                    let (first, second) = number_texts(node);
                    steps.extend(spaced(Cow::Owned(first)));
                    if let Some(second) = second {
                        steps.extend(spaced(Cow::Owned(second)));
                    }
                }
                AstKind::Name | AstKind::NameTime | AstKind::NameAvogadro => {
                    steps.extend(spaced(named_text(node)));
                }
                AstKind::ConstantE
                | AstKind::ConstantFalse
                | AstKind::ConstantPi
                | AstKind::ConstantTrue => {}
                // a math libsbml could not read is no element, so it carries no
                // text and no indentation of its own either
                AstKind::Unknown => {}
                AstKind::FunctionPiecewise => {
                    // each case of a piecewise stands in a piece or an otherwise
                    for case in node.children.chunks(2) {
                        steps.push(Step::Indent(level + 1));
                        for child in case.iter().filter(|c| c.kind != AstKind::Unknown) {
                            steps.push(Step::Indent(level + 2));
                            steps.push(Step::Node(child, level + 2, depth + 1));
                        }
                        steps.push(Step::Indent(level + 1));
                    }
                    if !node.children.is_empty() {
                        steps.push(Step::Indent(level));
                    }
                }
                AstKind::Lambda => {
                    // every child of a lambda but the last stands in a bvar
                    let last = node.children.len().saturating_sub(1);
                    for (n, child) in node.children.iter().enumerate() {
                        steps.push(Step::Indent(level + 1));
                        if n == last {
                            steps.push(Step::Node(child, level + 1, depth + 1));
                        } else {
                            steps.push(Step::Indent(level + 2));
                            steps.push(Step::Node(child, level + 2, depth + 1));
                            steps.push(Step::Indent(level + 1));
                        }
                    }
                    if !node.children.is_empty() {
                        steps.push(Step::Indent(level));
                    }
                }
                // every other kind is an apply whose first element names it: a
                // ci or a csymbol which carries a text, or an element of none
                kind => {
                    steps.push(Step::Indent(level + 1));
                    if kind == AstKind::Function || is_csymbol_function(kind) {
                        steps.extend(spaced(named_text(node)));
                    }
                    let args = operands(node, depth)?;
                    // the first operand of a logarithm and of a root of two
                    // stands in a logbase or a degree, one element deeper
                    let wrapped = matches!(kind, AstKind::FunctionLog | AstKind::FunctionRoot)
                        && args.len() == 2;
                    for (n, child) in args.iter().enumerate() {
                        steps.push(Step::Indent(level + 1));
                        if wrapped && n == 0 {
                            steps.push(Step::Indent(level + 2));
                            steps.push(Step::Node(child, level + 2, depth + 1));
                            steps.push(Step::Indent(level + 1));
                        } else {
                            steps.push(Step::Node(child, level + 1, depth + 1));
                        }
                    }
                    steps.push(Step::Indent(level));
                }
            },
        }
        self.steps.extend(steps.into_iter().rev());
        Ok(())
    }
}

/// The text libsbml writes in the `ci` or the `csymbol` of a named node.
fn named_text(node: &Ast) -> Cow<'_, str> {
    Cow::Borrowed(node.name.as_deref().unwrap_or_default())
}

/// `text` with the space libsbml writes on each side of the text of an element.
fn spaced(text: Cow<'_, str>) -> [Step<'_>; 3] {
    [
        Step::Text(Cow::Borrowed(" ")),
        Step::Text(text),
        Step::Text(Cow::Borrowed(" ")),
    ]
}

/// Whether the string values of `left` and of `right` differ.
///
/// The two are produced in step and read up to the first byte which differs, so
/// neither is ever built whole. See [`StringValue`].
///
/// # Errors
///
/// [`MathError::TooDeep`] for a subtree nested deeper than [`MAX_AST_DEPTH`].
fn string_values_differ(left: &Ast, right: &Ast, depth: usize) -> Result<bool, MathError> {
    let mut left = StringValue::of(left, depth);
    let mut right = StringValue::of(right, depth);
    loop {
        match (left.next_byte()?, right.next_byte()?) {
            (None, None) => return Ok(false),
            (ours, theirs) if ours != theirs => return Ok(true),
            _ => {}
        }
    }
}

/* --------------------------------------------------------------- the walk */

/// Writes the latex of `node` as the two stylesheets write it.
///
/// `p` is the `$p` of ctopff, the precedence its parent passed down, `parent`
/// and `index` are what the templates which ask `name(../*[1])` and
/// `. != ../*[2]` read, and `depth` is how deep the node stands.
fn emit(
    node: &Ast,
    p: i32,
    parent: Option<&Ast>,
    index: usize,
    depth: usize,
    out: &mut String,
) -> Result<(), MathError> {
    if depth > MAX_AST_DEPTH {
        return Err(MathError::TooDeep);
    }
    if node.kind == AstKind::Other {
        return Err(MathError::UnknownKind(node.kind));
    }
    // libsbml writes no mathml element of a math it could not read, so the
    // stylesheets see a tree without that node and write nothing of it. Every
    // construct which counts its operands sees one operand fewer, which
    // `operands` is where that happens.
    if node.kind == AstKind::Unknown {
        return Ok(());
    }
    if let Some(special) = special_real(node) {
        emit_special_real(special, p, out);
        return Ok(());
    }
    match node.kind {
        AstKind::Name => write_mi_italic(node.name.as_deref().unwrap_or_default(), out),
        // a csymbol is typeset by the text the document gave it, not by the word
        // libsbml knows it under
        AstKind::NameTime | AstKind::NameAvogadro => {
            write_mi(node.name.as_deref().unwrap_or_default(), out);
        }
        AstKind::Integer | AstKind::Real | AstKind::RealE | AstKind::Rational => {
            emit_number(node, out);
        }
        AstKind::ConstantE => write_mi("e", out),
        AstKind::ConstantPi => write_mi("\u{03c0}", out),
        AstKind::ConstantTrue => write_mi("true", out),
        AstKind::ConstantFalse => write_mi("false", out),
        _ => return emit_apply(node, p, parent, index, depth, out),
    }
    Ok(())
}

/// Writes a real libsbml writes as an element of its own rather than as a `cn`.
fn emit_special_real(special: SpecialReal, p: i32, out: &mut String) {
    match special {
        SpecialReal::NotANumber => write_mi("NaN", out),
        SpecialReal::Infinity => write_mi("\u{221e}", out),
        SpecialReal::NegativeInfinity => {
            // the unary minus template, whose one operand is the infinity
            let group = p >= 5;
            open(group, true, out);
            out.push('-');
            write_mi("\u{221e}", out);
            close(group, true, out);
        }
    }
}

/// Writes a number: its `cn`, and the `sep` of a rational or of an exponent form.
fn emit_number(node: &Ast, out: &mut String) {
    let (first, second) = number_texts(node);
    write_mn(&first, out);
    let Some(second) = second else {
        return;
    };
    if node.kind == AstKind::Rational {
        out.push('/');
        write_mn(&second, out);
    } else {
        out.push_str("\\cdot {10}^{");
        write_mn(&second, out);
        out.push('}');
    }
}

/// Writes a node which libsbml writes as an `apply`, a `piecewise` or a `lambda`.
///
/// Every construct is written by a function of its own, which keeps the frame of
/// this one small: a chain of a thousand nodes puts a thousand of them on the
/// stack.
fn emit_apply(
    node: &Ast,
    p: i32,
    parent: Option<&Ast>,
    index: usize,
    depth: usize,
    out: &mut String,
) -> Result<(), MathError> {
    let op = parent_operator(parent);
    let args = operands(node, depth)?;
    if let Some(shape) = shape(node.kind) {
        return match shape {
            Shape::Delimited(delimiters) => emit_delimited(node, &args, delimiters, depth, out),
            Shape::Named(name) => emit_named(node, &args, name, false, p, op, depth, out),
            Shape::Infix(token, this_p) => emit_infix(node, &args, token, this_p, p, depth, out),
            Shape::Set(name) => emit_set(node, &args, name, depth, out),
        };
    }
    match node.kind {
        AstKind::Divide => {
            // a quotient is wrapped only as the base of a power, where the
            // parentheses hold a fraction alone and so are stretched
            emit_fraction(node, &args, p >= 5 && op == Operator::Power, depth, out)
        }
        AstKind::Minus if args.len() == 1 => emit_negation(node, &args, p, depth, out),
        AstKind::Minus if args.len() >= 2 => {
            emit_binary(node, &args, "-", 2, p, parent, index, depth, out)
        }
        AstKind::LogicalImplies => emit_binary(
            node,
            &args,
            "\\Rightarrow ",
            3,
            p,
            parent,
            index,
            depth,
            out,
        ),
        AstKind::Plus => emit_plus(node, &args, p, parent, index, depth, out),
        AstKind::Times => emit_times(node, &args, p, op, 0, depth, out),
        AstKind::Power | AstKind::FunctionPower => {
            let group = op == Operator::Power && args.get(1).is_some_and(|e| is_apply(e));
            emit_power(node, &args, group, depth, out)
        }
        AstKind::FunctionExp => {
            let group = op == Operator::Power && args.first().is_some_and(|e| is_apply(e));
            emit_exponential(node, &args, group, depth, out)
        }
        AstKind::FunctionLog => emit_logarithm(node, &args, p, op, depth, out),
        AstKind::FunctionRoot => emit_root(node, &args, depth, out),
        AstKind::FunctionPiecewise => emit_piecewise(node, depth, out),
        AstKind::Lambda => emit_lambda(node, depth, out),
        // the generic apply template: a call of a function of the model, a
        // csymbol function, and a difference of no operand, which is written as
        // a call of its name
        _ => emit_call(node, &args, depth, out),
    }
}

/// A construct ctopff writes by one template which a fixed token or name fills in.
#[derive(Debug, Clone, Copy)]
enum Shape {
    /// A node whose arguments stand between delimiters, such as the floor.
    Delimited(&'static Delimiters),
    /// A trigonometric function or the natural logarithm, written by its name.
    Named(&'static str),
    /// A relation or a conjunction, written between its operands.
    Infix(&'static str, i32),
    /// A maximum, a minimum or a remainder, written as a name and a set.
    Set(&'static str),
}

/// How ctopff writes a node of `kind`, [`None`] for a construct with a template
/// of its own.
///
/// Reading the shape of a node once, rather than asking each table in turn in
/// the dispatcher, is what keeps the frame of [`emit_apply`] small.
fn shape(kind: AstKind) -> Option<Shape> {
    if let Some(delimiters) = delimiters(kind) {
        return Some(Shape::Delimited(delimiters));
    }
    if let Some(name) = trigonometric_name(kind) {
        return Some(Shape::Named(name));
    }
    if let Some((token, this_p)) = infix_operator(kind) {
        return Some(Shape::Infix(token, this_p));
    }
    set_function_name(kind).map(Shape::Set)
}

/// Writes a logarithm, whose base stands below its name unless it is ten.
fn emit_logarithm(
    node: &Ast,
    args: &[&Ast],
    p: i32,
    op: Operator,
    depth: usize,
    out: &mut String,
) -> Result<(), MathError> {
    // the base stands in a logbase and is no child of the apply, so the
    // argument alone is written and tested
    let rest: Vec<&Ast> = args.get(1..).unwrap_or_default().to_vec();
    let subscript = args.len() > 1 && !args.first().is_some_and(|b| wrapper_number_is(b, 10.0));
    emit_named(node, &rest, "log", subscript, p, op, depth, out)
}

/// Writes a quotient as a fraction, in parentheses where it is the base of a power.
fn emit_fraction(
    node: &Ast,
    args: &[&Ast],
    group: bool,
    depth: usize,
    out: &mut String,
) -> Result<(), MathError> {
    open(group, false, out);
    out.push_str("\\frac{");
    argument(node, args, 0, 0, depth, out)?;
    out.push_str("}{");
    argument(node, args, 1, 0, depth, out)?;
    out.push('}');
    close(group, false, out);
    Ok(())
}

/// Writes a negation, the unary minus template of ctopff.
fn emit_negation(
    node: &Ast,
    args: &[&Ast],
    p: i32,
    depth: usize,
    out: &mut String,
) -> Result<(), MathError> {
    let group = p >= 5;
    open(group, true, out);
    out.push('-');
    argument(node, args, 0, 5, depth, out)?;
    close(group, true, out);
    Ok(())
}

/// Writes a power as a superscript, its base wrapped where the power is itself one.
fn emit_power(
    node: &Ast,
    args: &[&Ast],
    group: bool,
    depth: usize,
    out: &mut String,
) -> Result<(), MathError> {
    open(group, false, out);
    out.push('{');
    argument(node, args, 0, 5, depth, out)?;
    out.push_str("}^{");
    argument(node, args, 1, 0, depth, out)?;
    out.push('}');
    close(group, false, out);
    Ok(())
}

/// Writes an exponential as a power of `e`.
fn emit_exponential(
    node: &Ast,
    args: &[&Ast],
    group: bool,
    depth: usize,
    out: &mut String,
) -> Result<(), MathError> {
    open(group, false, out);
    out.push_str("{e}^{");
    argument(node, args, 0, 0, depth, out)?;
    out.push('}');
    close(group, false, out);
    Ok(())
}

/// What ctopff writes before, between and after the arguments of a delimited
/// node, and the precedence it passes them.
#[derive(Debug, Clone, Copy)]
struct Delimiters {
    /// What stands before the first argument.
    before: &'static str,
    /// What stands between the two arguments, [`None`] where one is written.
    between: Option<&'static str>,
    /// What stands after the last argument.
    after: &'static str,
    /// The `$p` the arguments are written with.
    inner_p: i32,
}

/// `\left|a\right|`, the absolute value.
const ABSOLUTE_VALUE: Delimiters = Delimiters {
    before: "\\left|",
    between: None,
    after: "\\right|",
    inner_p: 0,
};

/// `\lfloor a\rfloor `, the floor.
const FLOOR: Delimiters = Delimiters {
    before: "\\lfloor ",
    between: None,
    after: "\\rfloor ",
    inner_p: 0,
};

/// `\lceil a\rceil `, the ceiling.
const CEILING: Delimiters = Delimiters {
    before: "\\lceil ",
    between: None,
    after: "\\rceil ",
    inner_p: 0,
};

/// `\lfloor a/b\rfloor `, the integer quotient of the extended math.
const INTEGER_QUOTIENT: Delimiters = Delimiters {
    before: "\\lfloor ",
    between: Some("/"),
    after: "\\rfloor ",
    inner_p: 0,
};

/// `a!`, the factorial.
const FACTORIAL: Delimiters = Delimiters {
    before: "",
    between: None,
    after: "!",
    inner_p: 7,
};

/// `\neg a`, the logical negation.
const NEGATION: Delimiters = Delimiters {
    before: "\\neg ",
    between: None,
    after: "",
    inner_p: 7,
};

/// The delimiters ctopff writes around the arguments of a node of `kind`,
/// [`None`] for every node it writes otherwise.
fn delimiters(kind: AstKind) -> Option<&'static Delimiters> {
    Some(match kind {
        AstKind::FunctionAbs => &ABSOLUTE_VALUE,
        AstKind::FunctionFloor => &FLOOR,
        AstKind::FunctionCeiling => &CEILING,
        AstKind::FunctionQuotient => &INTEGER_QUOTIENT,
        AstKind::FunctionFactorial => &FACTORIAL,
        AstKind::LogicalNot => &NEGATION,
        _ => return None,
    })
}

/// Writes a node whose arguments stand between delimiters of their own.
fn emit_delimited(
    node: &Ast,
    args: &[&Ast],
    delimiters: &Delimiters,
    depth: usize,
    out: &mut String,
) -> Result<(), MathError> {
    out.push_str(delimiters.before);
    argument(node, args, 0, delimiters.inner_p, depth, out)?;
    if let Some(token) = delimiters.between {
        out.push_str(token);
        argument(node, args, 1, delimiters.inner_p, depth, out)?;
    }
    out.push_str(delimiters.after);
    Ok(())
}

/// Writes a root, as a square root unless its degree says otherwise.
fn emit_root(node: &Ast, args: &[&Ast], depth: usize, out: &mut String) -> Result<(), MathError> {
    let degreed = args.len() > 1;
    if !degreed
        || args
            .first()
            .is_some_and(|degree| wrapper_number_is(degree, 2.0))
    {
        out.push_str("\\sqrt{");
        // a root of one child carries its radicand there, one of two after its degree
        argument(node, args, usize::from(degreed), 0, depth, out)?;
        out.push('}');
        return Ok(());
    }
    out.push_str("\\sqrt[");
    if let Some(degree) = args.first() {
        emit(degree, 0, None, 0, depth + 1, out)?;
    }
    out.push_str("]{");
    argument(node, args, 1, 0, depth, out)?;
    out.push('}');
    Ok(())
}

/// Writes a relation or a conjunction, the `infix` template of ctopff.
fn emit_infix(
    node: &Ast,
    args: &[&Ast],
    token: &str,
    this_p: i32,
    p: i32,
    depth: usize,
    out: &mut String,
) -> Result<(), MathError> {
    let group = this_p < p;
    let has_operator = args.len() > 1;
    open(group, has_operator, out);
    for (n, child) in args.iter().enumerate() {
        if n > 0 {
            out.push_str(token);
        }
        emit(child, this_p, Some(node), n, depth + 1, out)?;
    }
    close(group, has_operator, out);
    Ok(())
}

/// Writes a maximum, a minimum or a remainder, the `set` template of ctopff.
///
/// The template separates its arguments with a comma, which is an operator
/// between the parentheses and keeps them unstretched.
fn emit_set(
    node: &Ast,
    args: &[&Ast],
    name: &str,
    depth: usize,
    out: &mut String,
) -> Result<(), MathError> {
    write_mi(name, out);
    let has_operator = args.len() > 1;
    open(true, has_operator, out);
    for (n, child) in args.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        emit(child, 0, Some(node), n, depth + 1, out)?;
    }
    close(true, has_operator, out);
    Ok(())
}

/// Writes a call of a function of the model or of a csymbol, the generic apply template.
///
/// The arguments stand in an `mfenced`, which `glayout.xsl` writes between a
/// stretched pair of parentheses whatever stands inside them.
fn emit_call(node: &Ast, args: &[&Ast], depth: usize, out: &mut String) -> Result<(), MathError> {
    let name = node.name.as_deref().unwrap_or_default();
    if node.kind == AstKind::Function {
        write_mi_italic(name, out);
    } else if is_csymbol_function(node.kind) {
        write_mi(name, out);
    } else {
        write_mi(operator_name(node.kind)?, out);
    }
    out.push_str("\\left(");
    for (n, child) in args.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        emit(child, 0, Some(node), n, depth + 1, out)?;
    }
    out.push_str("\\right)");
    Ok(())
}

/// Writes operand `n` of `node`, nothing where the node has none.
///
/// The stylesheets select the element of an operand by its position and write
/// nothing where there is none, so a quotient of one operand is `\frac{a}{}`
/// rather than an error.
fn argument(
    node: &Ast,
    args: &[&Ast],
    n: usize,
    p: i32,
    depth: usize,
    out: &mut String,
) -> Result<(), MathError> {
    match args.get(n) {
        Some(child) => emit(child, p, Some(node), n, depth + 1, out),
        None => Ok(()),
    }
}

/// Writes a trigonometric function, a natural logarithm or a logarithm.
///
/// The template writes parentheses around the argument only where the node
/// carries an `apply` among its children, and wraps the whole call where its
/// parent binds at least as tight as a power and the argument is no `apply`.
/// `args` are the operands the apply carries, whose first is the argument;
/// `subscript` says that the base of a logarithm stands below its name, in which
/// case it is the first child of the node itself.
#[expect(clippy::too_many_arguments, reason = "the parameters of the template")]
fn emit_named(
    node: &Ast,
    args: &[&Ast],
    name: &str,
    subscript: bool,
    p: i32,
    op: Operator,
    depth: usize,
    out: &mut String,
) -> Result<(), MathError> {
    let has_apply = args.iter().any(|child| is_apply(child));
    let group = p >= 5 && !has_apply && op != Operator::Minus;
    // the name and the invisible function application stand between the outer
    // parentheses, so those are never stretched
    open(group, true, out);
    if subscript {
        out.push('{');
        write_mi(name, out);
        out.push_str("}_{");
        if let Some(base) = node.children.first() {
            emit(base, 0, None, 0, depth + 1, out)?;
        }
        out.push('}');
    } else {
        write_mi(name, out);
    }
    open(has_apply, false, out);
    if let Some(first) = args.first() {
        emit(first, 0, Some(node), 0, depth + 1, out)?;
    }
    close(has_apply, false, out);
    close(group, true, out);
    Ok(())
}

/// Writes a difference or an implication, the `binary` template of ctopff.
#[expect(clippy::too_many_arguments, reason = "the parameters of the template")]
fn emit_binary(
    node: &Ast,
    args: &[&Ast],
    token: &str,
    this_p: i32,
    p: i32,
    parent: Option<&Ast>,
    index: usize,
    depth: usize,
    out: &mut String,
) -> Result<(), MathError> {
    let group = this_p < p
        || (this_p == p
            && token == "-"
            && parent_operator(parent) == Operator::Minus
            && right_of_minus(parent, index, depth)?);
    open(group, true, out);
    argument(node, args, 0, this_p, depth, out)?;
    out.push_str(token);
    argument(node, args, 1, this_p, depth, out)?;
    close(group, true, out);
    Ok(())
}

/// Writes a sum, the plus template of ctopff.
///
/// The template writes a minus rather than a plus before an operand which is a
/// negative number or a negated expression.
fn emit_plus(
    node: &Ast,
    args: &[&Ast],
    p: i32,
    parent: Option<&Ast>,
    index: usize,
    depth: usize,
    out: &mut String,
) -> Result<(), MathError> {
    let op = parent_operator(parent);
    let group = (p > 2 && op != Operator::Minus)
        || (op == Operator::Minus && args.len() != 1 && right_of_minus(parent, index, depth)?);
    // an operand of its own carries no operator, so a sum of one positive
    // operand is written between stretched parentheses
    let has_operator = args.len() > 1
        || args
            .first()
            .is_some_and(|first| is_unary_minus(first) || is_negative_number(first));
    open(group, has_operator, out);
    for (n, child) in args.iter().enumerate() {
        if is_unary_minus(child) || is_negative_number(child) {
            out.push('-');
        } else if n > 0 {
            out.push('+');
        }
        emit_summand(node, child, n, depth, out)?;
    }
    close(group, has_operator, out);
    Ok(())
}

/// Writes one operand of a sum, whose sign the plus template has written.
///
/// The template writes the number of a negative operand negated, and pulls the
/// sign out of a product whose first factor is a negative number, which drops
/// that sign altogether, and out of one whose first factor is a difference of
/// three operands or more.
fn emit_summand(
    node: &Ast,
    child: &Ast,
    index: usize,
    depth: usize,
    out: &mut String,
) -> Result<(), MathError> {
    if is_negative_number(child) {
        write_mn(&xpath_number(-number_value(child)), out);
        return Ok(());
    }
    if special_real(child) == Some(SpecialReal::NegativeInfinity) {
        // the one element of that apply is the infinity, which the node holds
        // no child for
        write_mi("\u{221e}", out);
        return Ok(());
    }
    if is_unary_minus(child) {
        if let Some(operand) = child.children.first() {
            emit(operand, 2, Some(child), 0, depth + 2, out)?;
        }
        return Ok(());
    }
    if child.kind == AstKind::Times && emit_pulled_product(child, depth, out)? {
        return Ok(());
    }
    emit(child, 2, Some(node), index, depth + 1, out)
}

/// Writes a product of a sum whose first factor the plus template pulls apart,
/// and answers whether it did.
///
/// The template writes the first factor itself where it is a negative number,
/// whose sign it has already written, and where it is a difference whose apply
/// holds more than three elements (`count(*) > 3` of `ctopff.xsl:537`), of which
/// it writes the first alone. **That second branch is dead in both
/// implementations**: `writeOperatorArgs` of libsbml writes the left and the
/// right child of a difference and nothing between them, so the apply of a
/// difference never holds more than three elements. It is ported because the
/// template has it, not because a tree reaches it.
fn emit_pulled_product(child: &Ast, depth: usize, out: &mut String) -> Result<bool, MathError> {
    let factors = operands(child, depth + 1)?;
    let Some(first) = factors.first() else {
        return Ok(false);
    };
    if is_negative_number(first) {
        write_mn(&xpath_number(-number_value(first)), out);
    } else if first.kind == AstKind::Minus {
        let inner = operands(first, depth + 2)?;
        if inner.len() < 3 {
            return Ok(false);
        }
        if let Some(inner) = inner.first() {
            emit(inner, 0, Some(first), 0, depth + 3, out)?;
        }
    } else {
        return Ok(false);
    }
    emit_times(child, &factors, 2, Operator::Other, 1, depth + 1, out)?;
    Ok(true)
}

/// Writes a product, the times template of ctopff.
///
/// `first` is the `$first` of the template: the number of leading factors whose
/// own latex is left out, which the plus template sets where it has written the
/// first factor of a product itself.
fn emit_times(
    node: &Ast,
    args: &[&Ast],
    p: i32,
    op: Operator,
    first: usize,
    depth: usize,
    out: &mut String,
) -> Result<(), MathError> {
    let group = p > 3 && op != Operator::Minus;
    let has_operator = args.len() > 1;
    open(group, has_operator, out);
    for (n, child) in args.iter().enumerate() {
        if n > 0 {
            out.push_str("\\cdot ");
        }
        if n >= first {
            emit(child, 3, Some(node), n, depth + 1, out)?;
        }
    }
    close(group, has_operator, out);
    Ok(())
}

/// Writes a piecewise as the array `tables.xsl` writes, which the replacements
/// turn into a cases environment.
fn emit_piecewise(node: &Ast, depth: usize, out: &mut String) -> Result<(), MathError> {
    out.push_str("\\{\\begin{array}{");
    // the template counts the columns of the first row, which is three whether
    // that row is a piece or the otherwise
    if !node.children.is_empty() {
        out.push_str("ccc");
    }
    out.push('}');
    for (n, case) in node.children.chunks(2).enumerate() {
        if n > 0 {
            out.push_str("\\\\ ");
        }
        // libsbml writes a pair of children as a piece and a trailing single
        // child as the otherwise, whatever either of them is, so the row is the
        // raw pair; a child of a math it could not read is written as no element
        // at all, so the children of the row which are written fill its slots
        // from the first, and the slot left over stays empty
        let written: Vec<(usize, &Ast)> = case
            .iter()
            .enumerate()
            .filter(|(_, child)| child.kind != AstKind::Unknown)
            .map(|(at, child)| (2 * n + at, child))
            .collect();
        if let Some((index, value)) = written.first() {
            emit(value, 0, Some(node), *index, depth + 1, out)?;
        }
        if case.len() > 1 {
            out.push_str("& \\text{\u{a0} if \u{a0}}\\hfill & ");
            if let Some((index, condition)) = written.get(1) {
                emit(condition, 0, Some(node), *index, depth + 1, out)?;
            }
        } else {
            out.push_str("& \\multicolumn{2}{c}{\\text{\u{a0} otherwise}}");
        }
    }
    out.push_str("\\end{array}");
    Ok(())
}

/// Writes a lambda, whose bound variables the replacements separate and close.
///
/// The syntax tree does not record which children of a lambda are its bound
/// variables, so the last one is read as its body, which is what every lambda of
/// a document has. A lambda of a bound variable and no body, which is not valid
/// SBML and which libsbml writes with the bound variable in both places, is
/// written here as a body alone.
fn emit_lambda(node: &Ast, depth: usize, out: &mut String) -> Result<(), MathError> {
    let Some((body, bvars)) = node.children.split_last() else {
        // a lambda of no child at all, which ctopff writes by the template of a
        // lambda without a bound variable
        out.push_str("{|}_{}");
        return Ok(());
    };
    if bvars.is_empty() {
        emit(body, 0, Some(node), 0, depth + 1, out)?;
        out.push_str("{|}_{}");
        return Ok(());
    }
    out.push_str("\\lambda ");
    for (n, bvar) in bvars.iter().enumerate() {
        emit(bvar, 0, Some(node), n, depth + 1, out)?;
    }
    out.push_str(".\\left(");
    emit(body, 0, Some(node), bvars.len(), depth + 1, out)?;
    out.push_str("\\right)");
    Ok(())
}

/* -------------------------------------------------------- the parentheses */

/// Writes the opening parenthesis of a group, stretched where it holds no operator.
///
/// `tokens.xsl` writes `\left(` for a parenthesis whose nearest operator sibling
/// is another parenthesis and a plain `(` where an operator stands between the
/// two, which is why `(a+b)` is plain and `\left(\frac{a}{b}\right)` is
/// stretched.
fn open(group: bool, has_operator: bool, out: &mut String) {
    if group {
        out.push_str(if has_operator { "(" } else { "\\left(" });
    }
}

/// Writes the closing parenthesis of a group.
fn close(group: bool, has_operator: bool, out: &mut String) {
    if group {
        out.push_str(if has_operator { ")" } else { "\\right)" });
    }
}

/* ---------------------------------------------------------- the predicates */

/// The operator of the parent of a node, which is `name(../*[1])` of ctopff.
///
/// Only a difference and a power are asked for by name: a minus spares its
/// operands the parentheses their precedence would ask for, and a power asks its
/// base for ones it would not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operator {
    /// The parent is a `minus`.
    Minus,
    /// The parent is a `power`.
    Power,
    /// The parent is anything else, or the node is the root of the math.
    Other,
}

/// The operator element of the `apply` of the parent of a node.
fn parent_operator(parent: Option<&Ast>) -> Operator {
    match parent.map(|node| node.kind) {
        Some(AstKind::Minus) => Operator::Minus,
        Some(AstKind::Power | AstKind::FunctionPower) => Operator::Power,
        _ => Operator::Other,
    }
}

/// `(. != ../*[2] or count(../*) = 2)`: whether a node is the right operand of a difference.
///
/// The templates tell the two operands of a difference apart by their string
/// values rather than by their position, so two operands of the same text are
/// taken for one and the right one is written without parentheses. See
/// [`StringValue`].
///
/// # Errors
///
/// [`MathError::TooDeep`] where an operand is nested deeper than [`MAX_AST_DEPTH`].
fn right_of_minus(parent: Option<&Ast>, index: usize, depth: usize) -> Result<bool, MathError> {
    let Some(parent) = parent else {
        return Ok(false);
    };
    if parent.kind != AstKind::Minus {
        return Ok(false);
    }
    let args = operands(parent, depth)?;
    let (Some(left), Some(right)) = (args.first(), args.get(1)) else {
        // a negation, whose apply carries two elements
        return Ok(true);
    };
    if index != 1 {
        return Ok(false);
    }
    string_values_differ(left, right, depth)
}

/// Whether libsbml writes `node` as an apply of a minus and one element.
///
/// This is `self::m:apply[count(*)=2 and *[1][self::m:minus]]` of the plus
/// template. A number whose value is the negative infinity satisfies it as well
/// as a negated expression does: `writeCN` writes it as that apply, although the
/// node carries no child at all.
fn is_unary_minus(node: &Ast) -> bool {
    node.kind == AstKind::Minus && node.children.len() == 1
        || special_real(node) == Some(SpecialReal::NegativeInfinity)
}

/// Whether libsbml writes `node` as a `cn` without a `sep` whose number is below zero.
///
/// This is `self::m:cn[not(m:sep) and (number(.) < 0)]` of the plus template.
/// The `sep` is what separates the two text pieces of a number, so a rational
/// and every number libsbml writes in exponent form are no candidates. Which
/// those are is not decided by the kind of the node but by the number it holds:
/// a plain `cn` whose `"%.15g"` carries an exponent is written in exponent form
/// as well, and `-1e-6` of a document is such a node. A real which libsbml
/// writes as an element of its own is no `cn` at all.
fn is_negative_number(node: &Ast) -> bool {
    match node.kind {
        AstKind::Integer | AstKind::Real => {
            special_real(node).is_none()
                && number_texts(node).1.is_none()
                && number_value(node) < 0.0
        }
        _ => false,
    }
}

/// The value `number(.)` of XPath reads of the `cn` of a number node.
fn number_value(node: &Ast) -> f64 {
    match node.number {
        Some(AstNumber::Integer(integer)) => integer as f64,
        Some(AstNumber::Real(real)) => real,
        _ => 0.0,
    }
}

/// Whether `number()` of the wrapper around `node` is `value`.
///
/// ctopff asks this of the `logbase` of a logarithm and of the `degree` of a
/// root, whose string value is a number only where they hold a number of one
/// text piece: a rational, a number in exponent form and every expression answer
/// `NaN` and so are written out.
fn wrapper_number_is(node: &Ast, value: f64) -> bool {
    if !matches!(node.kind, AstKind::Integer | AstKind::Real) || special_real(node).is_some() {
        return false;
    }
    let (first, second) = number_texts(node);
    second.is_none() && first.parse::<f64>().is_ok_and(|parsed| parsed == value)
}

/// Whether libsbml writes a node of `kind` as a csymbol applied to its arguments.
///
/// The delay, the rate of change and every distribution of the distrib package
/// are csymbols, which carry the text the document gave them; the functions of
/// the extended math of Level 3 Version 2 are elements of their own. A csymbol
/// of a definition URL libsbml does not know is one of its own text as well.
fn is_csymbol_function(kind: AstKind) -> bool {
    matches!(
        kind,
        AstKind::CsymbolFunction
            | AstKind::FunctionDelay
            | AstKind::FunctionRateOf
            | AstKind::DistribBernoulli
            | AstKind::DistribBinomial
            | AstKind::DistribCauchy
            | AstKind::DistribChisquare
            | AstKind::DistribExponential
            | AstKind::DistribGamma
            | AstKind::DistribLaplace
            | AstKind::DistribLognormal
            | AstKind::DistribNormal
            | AstKind::DistribPoisson
            | AstKind::DistribRayleigh
            | AstKind::DistribUniform
    )
}

/// The name of the mathml element of a node ctopff writes upright before its
/// argument, [`None`] for every other kind.
///
/// ctopff writes the local name of the element, so these are the names of the
/// MathML elements libsbml writes and not the words it gives the nodes.
fn trigonometric_name(kind: AstKind) -> Option<&'static str> {
    Some(match kind {
        AstKind::FunctionSin => "sin",
        AstKind::FunctionCos => "cos",
        AstKind::FunctionTan => "tan",
        AstKind::FunctionSec => "sec",
        AstKind::FunctionCsc => "csc",
        AstKind::FunctionCot => "cot",
        AstKind::FunctionSinh => "sinh",
        AstKind::FunctionCosh => "cosh",
        AstKind::FunctionTanh => "tanh",
        AstKind::FunctionSech => "sech",
        AstKind::FunctionCsch => "csch",
        AstKind::FunctionCoth => "coth",
        AstKind::FunctionArcsin => "arcsin",
        AstKind::FunctionArccos => "arccos",
        AstKind::FunctionArctan => "arctan",
        AstKind::FunctionArcsinh => "arcsinh",
        AstKind::FunctionArccosh => "arccosh",
        AstKind::FunctionArctanh => "arctanh",
        AstKind::FunctionArcsec => "arcsec",
        AstKind::FunctionArcsech => "arcsech",
        AstKind::FunctionArccsc => "arccsc",
        AstKind::FunctionArccsch => "arccsch",
        AstKind::FunctionArccot => "arccot",
        AstKind::FunctionArccoth => "arccoth",
        AstKind::FunctionLn => "ln",
        _ => return None,
    })
}

/// The token and the precedence of a node ctopff writes with the `infix` template.
fn infix_operator(kind: AstKind) -> Option<(&'static str, i32)> {
    Some(match kind {
        AstKind::RelationalEq => ("=", 1),
        AstKind::RelationalNeq => ("\\ne ", 1),
        AstKind::RelationalGt => (">", 1),
        AstKind::RelationalLt => ("<", 1),
        AstKind::RelationalGeq => ("\\ge ", 1),
        AstKind::RelationalLeq => ("\\le ", 1),
        AstKind::LogicalAnd => ("\\wedge ", 2),
        AstKind::LogicalOr => ("\\vee ", 3),
        AstKind::LogicalXor => ("\\oplus ", 3),
        _ => return None,
    })
}

/// The name of a node ctopff writes as a name and a parenthesised set of arguments.
fn set_function_name(kind: AstKind) -> Option<&'static str> {
    Some(match kind {
        AstKind::FunctionMax => "max",
        AstKind::FunctionMin => "min",
        AstKind::FunctionRem => "rem",
        _ => return None,
    })
}

/// The name ctopff gives an operator element it writes as a call of its name.
///
/// Only a difference of no operand reaches this: every other operator has a
/// template of its own whatever its number of operands, a sum and a product of
/// none included.
fn operator_name(kind: AstKind) -> Result<&'static str, MathError> {
    match kind {
        AstKind::Minus => Ok("minus"),
        _ => Err(MathError::UnknownKind(kind)),
    }
}
