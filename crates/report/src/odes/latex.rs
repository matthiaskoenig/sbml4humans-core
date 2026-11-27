//! Textbook LaTeX used by the equation report, distinct from the MathML view.
use super::ast;
use sbml4humans_model::{Ast, AstKind as K, AstNumber};
use std::{borrow::Cow, collections::HashMap};

#[derive(Clone)]
struct Printed {
    code: String,
    precedence: u8,
}
fn printed(code: impl Into<String>, precedence: u8) -> Printed {
    Printed {
        code: code.into(),
        precedence,
    }
}
fn parentheses(code: &str) -> String {
    format!(r"\mathopen{{}}\left({code}\right)")
}
fn wrap(p: &Printed, precedence: u8) -> String {
    if p.precedence < precedence {
        parentheses(&p.code)
    } else {
        p.code.clone()
    }
}
fn unsigned(p: Printed) -> Printed {
    if p.precedence == 9 {
        printed(parentheses(&p.code), 11)
    } else {
        p
    }
}
fn negate(p: Printed) -> Printed {
    printed(format!("-{}", wrap(&unsigned(p), 8)), 9)
}
fn literal(code: String) -> Printed {
    let precedence = if code.starts_with('-') {
        9
    } else if code.contains(r" \times ") {
        8
    } else if code.contains('^') {
        10
    } else {
        11
    };
    printed(code, precedence)
}
fn call(name: &str, args: Vec<Printed>) -> Printed {
    printed(
        format!(
            "{name}{}",
            parentheses(
                &args
                    .into_iter()
                    .map(|p| p.code)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        ),
        10,
    )
}
fn number(value: f64) -> String {
    if value.is_nan() {
        return r"\mathrm{NaN}".into();
    }
    if value.is_infinite() {
        return if value < 0.0 { r"-\infty" } else { r"\infty" }.into();
    }
    let text = crate::number::python_float(value + 0.0);
    let (m, e) = text.split_once('e').unwrap_or((&text, ""));
    let m = m.strip_suffix(".0").unwrap_or(m);
    if e.is_empty() {
        return m.to_owned();
    }
    let e = e.parse::<i32>().unwrap_or(0);
    if m == "1" {
        format!("10^{{{e}}}")
    } else if m == "-1" {
        format!("-10^{{{e}}}")
    } else {
        format!(r"{m} \times 10^{{{e}}}")
    }
}
fn signed(a: &Ast) -> bool {
    match a.kind {
        K::Plus | K::Minus => !a.children.is_empty(),
        K::Times if a.children.len() > 1 => {
            let f = &a.children[0];
            negative(f) || (f.kind == K::Minus && f.children.len() == 1)
        }
        _ => false,
    }
}
fn negative(a: &Ast) -> bool {
    matches!(a.kind, K::Integer | K::Real | K::RealE) && ast::value(a).is_some_and(|n| n < 0.0)
}
fn terms(root: &Ast) -> Vec<(bool, Cow<'_, Ast>)> {
    let mut out = Vec::new();
    let mut stack = vec![(false, Cow::Borrowed(root))];
    while let Some((sign, current)) = stack.pop() {
        if matches!(current.kind, K::Plus | K::Minus) && !current.children.is_empty() {
            let subtract = current.kind == K::Minus;
            let unary = subtract && current.children.len() == 1;
            match current {
                Cow::Borrowed(a) => {
                    for (i, c) in a.children.iter().enumerate().rev() {
                        stack.push((sign ^ (subtract && (i > 0 || unary)), Cow::Borrowed(c)));
                    }
                }
                Cow::Owned(mut a) => {
                    for (i, c) in std::mem::take(&mut a.children)
                        .into_iter()
                        .enumerate()
                        .rev()
                    {
                        stack.push((sign ^ (subtract && (i > 0 || unary)), Cow::Owned(c)));
                    }
                }
            }
        } else if negative(&current) {
            out.push((
                !sign,
                Cow::Owned(ast::number(-ast::value(&current).unwrap_or(0.0))),
            ));
        } else if current.kind == K::Times && signed(&current) {
            let first = &current.children[0];
            let factor = if first.kind == K::Minus {
                first.children[0].clone()
            } else {
                ast::number(-ast::value(first).unwrap_or(0.0))
            };
            let mut factors = Vec::new();
            if !ast::is(&factor, 1.0) {
                factors.push(factor);
            }
            factors.extend(current.children.iter().skip(1).cloned());
            stack.push((!sign, Cow::Owned(ast::product(factors))));
        } else {
            out.push((sign, current));
        }
    }
    out
}
fn sum_codes(terms: Vec<(bool, Printed)>) -> Vec<String> {
    let single = terms.len() == 1;
    terms
        .into_iter()
        .enumerate()
        .map(|(i, (negative, p))| {
            if i == 0 {
                let p = if negative { negate(p) } else { p };
                if single { p.code } else { wrap(&p, 7) }
            } else {
                format!(
                    "{}{}",
                    if negative { "- " } else { "+ " },
                    wrap(&unsigned(p), 8)
                )
            }
        })
        .collect()
}
fn logic(operands: Vec<Printed>, operator: &str, precedence: u8, relations: bool) -> Printed {
    printed(
        operands
            .into_iter()
            .map(|p| {
                if p.precedence == 4 || p.precedence >= 10 || (relations && p.precedence == 5) {
                    p.code
                } else {
                    parentheses(&p.code)
                }
            })
            .collect::<Vec<_>>()
            .join(operator),
        precedence,
    )
}
fn boolean(p: Printed, condition: bool) -> Printed {
    if condition {
        p
    } else {
        printed(format!(r"\mathopen{{}}\left[{}\right]", p.code), 11)
    }
}
fn supported_symbol(sid: &str) -> Result<(), String> {
    let mut chars = sid.chars();
    if !chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        Err(format!(
            "The id '{sid}' is not an SId and cannot be typeset."
        ))
    } else {
        Ok(())
    }
}
fn lookup(symbols: &HashMap<String, String>, sid: &str) -> Result<String, String> {
    if let Some(symbol) = symbols.get(sid) {
        return Ok(symbol.clone());
    }
    supported_symbol(sid)?;
    symbols.get(sid).cloned().ok_or_else(||format!("The identifier '{sid}' has no symbol and is not supported by the latex printer (e.g. a local parameter or a function definition which is not part of the symbols)."))
}

fn chain_node(a: &Ast) -> bool {
    !signed(a)
        && ((a.kind == K::Times && a.children.len() >= 2)
            || (a.kind == K::Divide && a.children.len() == 2))
}
fn operation(kind: K, mut args: Vec<Printed>) -> Printed {
    if kind == K::Divide {
        return printed(
            format!(r"\frac{{{}}}{{{}}}", args[0].code, args[1].code),
            10,
        );
    }
    let first = args.remove(0);
    let negative = first.precedence == 9;
    let mut parts = vec![wrap(&first, 8)];
    parts.extend(args.into_iter().map(|p| wrap(&unsigned(p), 9)));
    printed(parts.join(r" \cdot "), if negative { 9 } else { 8 })
}
fn render_chain(
    a: &Ast,
    symbols: &HashMap<String, String>,
    depth: usize,
) -> Result<Printed, String> {
    let mut frames = vec![(a, 0, Vec::new())];
    loop {
        let Some((current, next, mut args)) = frames.pop() else {
            return Ok(printed("0", 11));
        };
        if let Some(child) = current.children.get(next) {
            if chain_node(child) {
                frames.push((current, next + 1, args));
                frames.push((child, 0, vec![]));
            } else {
                args.push(render(child, symbols, false, depth + 1)?);
                frames.push((current, next + 1, args));
            }
        } else {
            let result = operation(current.kind, args);
            if let Some(parent) = frames.last_mut() {
                parent.2.push(result);
            } else {
                return Ok(result);
            }
        }
    }
}
fn render(
    a: &Ast,
    symbols: &HashMap<String, String>,
    condition: bool,
    depth: usize,
) -> Result<Printed, String> {
    if depth >= 100 {
        return Err(
            "The math is nested more than 100 levels deep, more than the latex printer writes."
                .into(),
        );
    }
    if signed(a) {
        let rendered = terms(a)
            .into_iter()
            .map(|(s, a)| Ok((s, render(&a, symbols, false, depth + 1)?)))
            .collect::<Result<Vec<_>, String>>()?;
        let single = rendered.len() == 1;
        let precedence = if single {
            if rendered[0].0 {
                9
            } else {
                rendered[0].1.precedence
            }
        } else {
            7
        };
        return Ok(printed(sum_codes(rendered).join(" "), precedence));
    }
    if chain_node(a) && a.children.iter().any(chain_node) {
        return render_chain(a, symbols, depth);
    }
    let mut args = Vec::new();
    for (i, c) in a.children.iter().enumerate() {
        let cond = match a.kind {
            K::LogicalAnd | K::LogicalOr | K::LogicalXor | K::LogicalNot | K::LogicalImplies => {
                true
            }
            K::FunctionPiecewise => {
                if i % 2 == 1 {
                    true
                } else {
                    condition
                }
            }
            _ => false,
        };
        args.push(render(c, symbols, cond, depth + 1)?);
    }
    let unsupported = || {
        format!(
            "The math '{}' is not supported by the latex printer.",
            crate::math::formula(a).unwrap_or_default()
        )
    };
    let p = match a.kind {
        K::Name => printed(lookup(symbols, a.name.as_deref().unwrap_or(""))?, 11),
        K::Function => call(&lookup(symbols, a.name.as_deref().unwrap_or(""))?, args),
        K::Integer => literal(match a.number {
            Some(AstNumber::Integer(n)) => n.to_string(),
            _ => "0".into(),
        }),
        K::Real | K::RealE => literal(number(ast::value(a).unwrap_or(f64::NAN))),
        K::Rational => match a.number {
            Some(AstNumber::Rational {
                numerator,
                denominator,
            }) => printed(format!(r"\frac{{{numerator}}}{{{denominator}}}"), 10),
            _ => return Err(unsupported()),
        },
        K::ConstantE => printed(r"\mathrm{e}", 11),
        K::ConstantPi => printed(r"\pi", 11),
        K::NameTime => printed("t", 11),
        K::NameAvogadro => printed(r"N_{\mathrm{A}}", 11),
        K::ConstantTrue | K::ConstantFalse => {
            return Ok(boolean(
                printed(
                    if a.kind == K::ConstantTrue {
                        r"\mathrm{true}"
                    } else {
                        r"\mathrm{false}"
                    },
                    11,
                ),
                condition,
            ));
        }
        K::Plus => printed("0", 11),
        K::Times => {
            if args.is_empty() {
                printed("1", 11)
            } else if args.len() == 1 {
                args.remove(0)
            } else {
                let first = args.remove(0);
                let negative = first.precedence == 9;
                let mut parts = vec![wrap(&first, 8)];
                parts.extend(args.into_iter().map(|p| wrap(&unsigned(p), 9)));
                printed(parts.join(r" \cdot "), if negative { 9 } else { 8 })
            }
        }
        K::Divide if args.len() == 2 => printed(
            format!(r"\frac{{{}}}{{{}}}", args[0].code, args[1].code),
            10,
        ),
        K::Power | K::FunctionPower if args.len() == 2 => {
            printed(format!("{}^{{{}}}", wrap(&args[0], 11), args[1].code), 10)
        }
        K::FunctionRoot | K::FunctionLog if !args.is_empty() && args.len() <= 2 => {
            let value = args.pop().unwrap();
            let degree = if a.children.len() == 2
                && !ast::is(
                    &a.children[0],
                    if a.kind == K::FunctionRoot { 2.0 } else { 10.0 },
                ) {
                args.pop()
            } else {
                None
            };
            if a.kind == K::FunctionRoot {
                let index = degree
                    .map(|p| {
                        if p.code.contains(']') {
                            format!("[{{{}}}]", p.code)
                        } else {
                            format!("[{}]", p.code)
                        }
                    })
                    .unwrap_or_default();
                printed(format!(r"\sqrt{index}{{{}}}", value.code), 10)
            } else {
                call(
                    &format!(
                        r"\log_{{{}}}",
                        degree.map(|p| p.code).unwrap_or_else(|| "10".into())
                    ),
                    vec![value],
                )
            }
        }
        K::FunctionFactorial if args.len() == 1 => printed(format!("{}!", wrap(&args[0], 11)), 10),
        K::FunctionAbs | K::FunctionCeiling | K::FunctionFloor if args.len() == 1 => {
            let (left, right) = match a.kind {
                K::FunctionAbs => (r"\mathopen{}\left| ", r" \right|"),
                K::FunctionCeiling => (r"\mathopen{}\left\lceil ", r" \right\rceil"),
                _ => (r"\mathopen{}\left\lfloor ", r" \right\rfloor"),
            };
            printed(format!("{left}{}{right}", args[0].code), 11)
        }
        K::FunctionPiecewise => {
            let mut rows = Vec::new();
            for pair in args.as_chunks::<2>().0 {
                rows.push(format!(r"{} & \text{{if }} {}", pair[0].code, pair[1].code));
            }
            let otherwise = if args.len() % 2 == 1 {
                args.last().unwrap().code.clone()
            } else {
                number(f64::NAN)
            };
            rows.push(format!(r"{otherwise} & \text{{otherwise}}"));
            printed(
                format!(r"\begin{{cases}} {} \end{{cases}}", rows.join(r" \\ ")),
                1,
            )
        }
        K::LogicalAnd | K::LogicalOr | K::LogicalXor | K::LogicalImplies => {
            let p = if args.is_empty() {
                printed(
                    if a.kind == K::LogicalAnd {
                        r"\mathrm{true}"
                    } else {
                        r"\mathrm{false}"
                    },
                    11,
                )
            } else if args.len() == 1 {
                args.remove(0)
            } else {
                let (op, precedence) = match a.kind {
                    K::LogicalAnd => (r" \land ", 3),
                    K::LogicalOr => (r" \lor ", 2),
                    K::LogicalXor => (r" \oplus ", 2),
                    _ => (r" \Rightarrow ", 2),
                };
                logic(args, op, precedence, a.kind != K::LogicalXor)
            };
            return Ok(boolean(p, condition));
        }
        K::LogicalNot if args.len() == 1 => {
            let arg = &args[0];
            let code = if arg.precedence == 4 || arg.precedence >= 10 {
                arg.code.clone()
            } else {
                parentheses(&arg.code)
            };
            return Ok(boolean(printed(format!(r"\lnot {code}"), 4), condition));
        }
        K::RelationalEq
        | K::RelationalNeq
        | K::RelationalGt
        | K::RelationalGeq
        | K::RelationalLt
        | K::RelationalLeq => {
            let op = match a.kind {
                K::RelationalEq => "=",
                K::RelationalNeq => r"\neq",
                K::RelationalGt => ">",
                K::RelationalGeq => r"\geq",
                K::RelationalLt => "<",
                _ => r"\leq",
            };
            let relations = args
                .windows(2)
                .map(|p| printed(format!("{} {op} {}", wrap(&p[0], 6), wrap(&p[1], 6)), 5))
                .collect::<Vec<_>>();
            let p = if relations.len() == 1 {
                relations[0].clone()
            } else if relations.is_empty() {
                printed(r"\mathrm{true}", 11)
            } else {
                logic(relations, r" \land ", 3, true)
            };
            return Ok(boolean(p, condition));
        }
        K::FunctionDelay | K::FunctionRateOf | K::Lambda | K::Unknown | K::Other => {
            return Err(unsupported());
        }
        _ => {
            let debug = format!("{:?}", a.kind);
            let function = debug
                .strip_prefix("Function")
                .or_else(|| debug.strip_prefix("Distrib"))
                .ok_or_else(unsupported)?
                .to_lowercase();
            if matches!(a.kind, K::FunctionMin | K::FunctionMax) && args.len() == 1 {
                args.remove(0)
            } else {
                let operator = if matches!(
                    function.as_str(),
                    "arccos"
                        | "arcsin"
                        | "arctan"
                        | "cos"
                        | "cosh"
                        | "cot"
                        | "coth"
                        | "csc"
                        | "exp"
                        | "ln"
                        | "max"
                        | "min"
                        | "sec"
                        | "sin"
                        | "sinh"
                        | "tan"
                        | "tanh"
                ) {
                    format!("\\{function}")
                } else {
                    format!(r"\operatorname{{{function}}}")
                };
                call(&operator, args)
            }
        }
    };
    Ok(p)
}

pub(super) fn print(
    ast: &Ast,
    symbols: &HashMap<String, String>,
    condition: bool,
) -> Result<String, String> {
    Ok(render(ast, symbols, condition, 0)?.code)
}
pub(super) fn lines(ast: &Ast, symbols: &HashMap<String, String>) -> Result<Vec<String>, String> {
    let parts = terms(ast)
        .into_iter()
        .map(|(s, a)| Ok((s, render(&a, symbols, false, 0)?)))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(sum_codes(parts).chunks(4).map(|c| c.join(" ")).collect())
}
pub(super) fn symbol(sid: &str) -> Result<String, String> {
    supported_symbol(sid)?;
    Ok(typeset(sid, true))
}
fn text(s: &str) -> String {
    format!(r"\mathrm{{{}}}", s.replace('_', r"\_"))
}
fn typeset(sid: &str, greek: bool) -> String {
    let digit = sid.find(|c: char| c.is_ascii_digit());
    let numbered = digit.filter(|i| {
        *i > 0
            && sid[..*i].chars().all(|c| c.is_ascii_alphabetic())
            && sid[*i..].chars().all(|c| c.is_ascii_digit())
    });
    let numbered = numbered.map(|i| format!("{}_{}", &sid[..i], &sid[i..]));
    let sid = numbered.as_deref().unwrap_or(sid);
    let (base, sub) = sid
        .split_once('_')
        .map_or((sid, None), |(b, s)| (b, Some(s)));
    if base.is_empty() || sub == Some("") {
        return text(sid);
    }
    let is_greek = [
        "alpha", "beta", "gamma", "delta", "zeta", "eta", "theta", "iota", "kappa", "lambda", "mu",
        "nu", "xi", "pi", "rho", "sigma", "tau", "upsilon", "chi", "psi", "omega", "Gamma",
        "Delta", "Theta", "Lambda", "Xi", "Pi", "Sigma", "Upsilon", "Phi", "Psi", "Omega",
    ]
    .contains(&base);
    let base = if greek && base == "epsilon" {
        r"\varepsilon".into()
    } else if greek && base == "phi" {
        r"\varphi".into()
    } else if greek && base == "omicron" {
        "o".into()
    } else if greek && is_greek {
        format!("\\{base}")
    } else if base.len() == 1 {
        base.into()
    } else {
        text(base)
    };
    sub.map_or(base.clone(), |s| {
        format!(
            "{base}_{{{}}}",
            if s.chars().all(|c| c.is_ascii_digit()) {
                s.into()
            } else {
                text(s)
            }
        )
    })
}
pub(super) fn symbols(ids: &[String]) -> Result<HashMap<String, String>, String> {
    let mut result = HashMap::new();
    for id in ids {
        result.insert(id.clone(), symbol(id)?);
    }
    let mut counts = HashMap::new();
    for v in result.values() {
        *counts.entry(v.clone()).or_insert(0) += 1;
    }
    for id in ids {
        if counts[&result[id]] > 1 && id.chars().any(|c| c.is_ascii_digit()) && !id.contains('_') {
            result.insert(id.clone(), text(id));
        }
    }
    let mut counts = HashMap::new();
    for v in result.values() {
        *counts.entry(v.clone()).or_insert(0) += 1;
    }
    for id in ids {
        if counts[&result[id]] > 1 {
            result.insert(id.clone(), typeset(id, false));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equation_latex_matches_sbmlode_corpus() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../testdata/oracles/equation_maths.json"
        ))
        .unwrap();
        let mut failures = Vec::new();
        for (i, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
            let math = sbml4humans_libsbml::read_math(case["mathml"].as_str().unwrap()).unwrap();
            let symbols: HashMap<String, String> =
                serde_json::from_value(case["symbols"].clone()).unwrap();
            for mode in ["math", "condition", "lines"] {
                let actual = if mode == "lines" {
                    lines(&math, &symbols).map(|lines| serde_json::json!(lines))
                } else {
                    print(&math, &symbols, mode == "condition").map(|text| serde_json::json!(text))
                };
                let actual = match actual {
                    Ok(value) => serde_json::json!({"ok":value}),
                    Err(error) => serde_json::json!({"error":error}),
                };
                if actual != case[mode] {
                    failures.push(format!(
                        "case {i} {mode}: ours {}, expected {}",
                        actual.to_string().chars().take(350).collect::<String>(),
                        case[mode].to_string().chars().take(350).collect::<String>()
                    ));
                }
            }
        }
        assert!(
            failures.is_empty(),
            "{} differences:\n{}",
            failures.len(),
            failures
                .iter()
                .take(20)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
