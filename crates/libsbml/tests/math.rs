//! The syntax tree libsbml fills of a content MathML and of the formula of a
//! Level 1 math, and what it refuses.

use sbml4humans_libsbml::{ReadError, read, read_formula, read_math};
use sbml4humans_model::{AstKind, AstNumber, MAX_AST_DEPTH};

/// A MathML document of a body, the envelope every math of a test is read in.
fn math(body: &str) -> String {
    format!("<math xmlns=\"http://www.w3.org/1998/Math/MathML\">{body}</math>")
}

/// The `math` element of the kinetic law of the one reaction of the test document.
fn kinetic_law_math() -> &'static str {
    let document = include_str!("../testdata/kinetic_law.xml");
    let start = document.find("<math").expect("the document holds a math");
    let end = document.find("</math>").expect("the math ends") + "</math>".len();
    &document[start..end]
}

/// The `formula` of the kinetic law of the one reaction of the Level 1 document.
fn level1_formula() -> &'static str {
    let document = include_str!("../testdata/level1_power.xml");
    let opening = "<kineticLaw formula=\"";
    let start = document
        .find(opening)
        .expect("the document holds a kinetic law")
        + opening.len();
    let formula = &document[start..];
    &formula[..formula.find('"').expect("the formula ends")]
}

/// A MathML of `1 + (1 + (1 + ...))`, `levels` applications deep.
///
/// The innermost number is one level below the innermost application, so the
/// deepest node of the tree is at level `levels + 1`.
fn nested(levels: usize) -> String {
    let mut body = String::new();
    for _ in 0..levels {
        body.push_str("<apply><plus/><cn type=\"integer\">1</cn>");
    }
    body.push_str("<cn type=\"integer\">1</cn>");
    for _ in 0..levels {
        body.push_str("</apply>");
    }
    math(&body)
}

/// A MathML of one flat `apply` of a `plus` over `terms` numbers.
///
/// It is the shape a document writes of a wide sum, `1 + 1 + ... + 1` as the
/// one n-ary application content MathML has for it, and it carries no nesting
/// at all.
fn flat_sum(terms: usize) -> String {
    let mut body = String::from("<apply><plus/>");
    for _ in 0..terms {
        body.push_str("<cn type=\"integer\">1</cn>");
    }
    body.push_str("</apply>");
    math(&body)
}

/// How many levels deep the tree `ast` is, without recursing.
fn depth_of(ast: &sbml4humans_model::Ast) -> usize {
    let mut deepest = 0;
    let mut stack = vec![(ast, 1)];
    while let Some((node, level)) = stack.pop() {
        deepest = deepest.max(level);
        stack.extend(node.children.iter().map(|child| (child, level + 1)));
    }
    deepest
}

/// Every kind a content MathML expresses, as the smallest math which has it.
///
/// Two kinds are not in the table, and each is covered by a test of its own.
/// `AstKind::Other` is by definition the type the shim does not name, which no
/// document of libsbml 5.21.2 reaches, so there is no math which has it; the
/// reader refuses a tree which holds one, which nothing but a libsbml that
/// declares a further type can produce. `AstKind::Power` is the `^` of the
/// `formula` attribute of a Level 1 math, which libsbml parses outside of
/// MathML, where `<power/>` is `AstKind::FunctionPower`, and which
/// `the_power_of_the_formula_of_a_level_1_kinetic_law_is_filled_as_a_power`
/// reads.
///
/// Three kinds are in the table as something other than an element of their own:
/// `AstKind::Function`, a call of a function the model defines, is an `apply` of
/// a `ci`; `AstKind::CsymbolFunction` is an `apply` of a csymbol of a definition
/// URL libsbml does not know; and `AstKind::Unknown` is the node libsbml answers
/// of a math it could not read, of which an empty `math` element is the one a
/// string reaches. A content MathML libsbml does not know reaches it as well,
/// with the children of the `apply` below it, but only through the reader of a
/// document: `readMathMLFromString` answers nothing at all of such a string, so
/// `crates/libsbml/tests/read.rs` covers that shape.
const KINDS: &[(&str, AstKind)] = &[
    ("", AstKind::Unknown),
    ("<ci>S1</ci>", AstKind::Name),
    (
        "<csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/time\">time</csymbol>",
        AstKind::NameTime,
    ),
    (
        "<csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/avogadro\">avogadro</csymbol>",
        AstKind::NameAvogadro,
    ),
    ("<cn type=\"integer\">2</cn>", AstKind::Integer),
    ("<cn>1.5</cn>", AstKind::Real),
    ("<cn type=\"e-notation\">1.35<sep/>-5</cn>", AstKind::RealE),
    ("<cn type=\"rational\">3<sep/>4</cn>", AstKind::Rational),
    ("<exponentiale/>", AstKind::ConstantE),
    ("<false/>", AstKind::ConstantFalse),
    ("<pi/>", AstKind::ConstantPi),
    ("<true/>", AstKind::ConstantTrue),
    ("<apply><plus/><cn>1</cn><cn>2</cn></apply>", AstKind::Plus),
    (
        "<apply><minus/><cn>1</cn><cn>2</cn></apply>",
        AstKind::Minus,
    ),
    (
        "<apply><times/><cn>1</cn><cn>2</cn></apply>",
        AstKind::Times,
    ),
    (
        "<apply><divide/><cn>1</cn><cn>2</cn></apply>",
        AstKind::Divide,
    ),
    ("<apply><ci>f</ci><cn>1</cn></apply>", AstKind::Function),
    (
        "<apply><csymbol encoding=\"text\" definitionURL=\"http://example.org/of/nothing\">g</csymbol><cn>1</cn></apply>",
        AstKind::CsymbolFunction,
    ),
    ("<apply><abs/><cn>1</cn></apply>", AstKind::FunctionAbs),
    (
        "<apply><arccos/><cn>1</cn></apply>",
        AstKind::FunctionArccos,
    ),
    (
        "<apply><arccosh/><cn>1</cn></apply>",
        AstKind::FunctionArccosh,
    ),
    (
        "<apply><arccot/><cn>1</cn></apply>",
        AstKind::FunctionArccot,
    ),
    (
        "<apply><arccoth/><cn>1</cn></apply>",
        AstKind::FunctionArccoth,
    ),
    (
        "<apply><arccsc/><cn>1</cn></apply>",
        AstKind::FunctionArccsc,
    ),
    (
        "<apply><arccsch/><cn>1</cn></apply>",
        AstKind::FunctionArccsch,
    ),
    (
        "<apply><arcsec/><cn>1</cn></apply>",
        AstKind::FunctionArcsec,
    ),
    (
        "<apply><arcsech/><cn>1</cn></apply>",
        AstKind::FunctionArcsech,
    ),
    (
        "<apply><arcsin/><cn>1</cn></apply>",
        AstKind::FunctionArcsin,
    ),
    (
        "<apply><arcsinh/><cn>1</cn></apply>",
        AstKind::FunctionArcsinh,
    ),
    (
        "<apply><arctan/><cn>1</cn></apply>",
        AstKind::FunctionArctan,
    ),
    (
        "<apply><arctanh/><cn>1</cn></apply>",
        AstKind::FunctionArctanh,
    ),
    (
        "<apply><ceiling/><cn>1.5</cn></apply>",
        AstKind::FunctionCeiling,
    ),
    ("<apply><cos/><cn>1</cn></apply>", AstKind::FunctionCos),
    ("<apply><cosh/><cn>1</cn></apply>", AstKind::FunctionCosh),
    ("<apply><cot/><cn>1</cn></apply>", AstKind::FunctionCot),
    ("<apply><coth/><cn>1</cn></apply>", AstKind::FunctionCoth),
    ("<apply><csc/><cn>1</cn></apply>", AstKind::FunctionCsc),
    ("<apply><csch/><cn>1</cn></apply>", AstKind::FunctionCsch),
    (
        "<apply><csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/delay\">delay</csymbol><ci>S1</ci><cn>1</cn></apply>",
        AstKind::FunctionDelay,
    ),
    ("<apply><exp/><cn>1</cn></apply>", AstKind::FunctionExp),
    (
        "<apply><factorial/><cn type=\"integer\">3</cn></apply>",
        AstKind::FunctionFactorial,
    ),
    (
        "<apply><floor/><cn>1.5</cn></apply>",
        AstKind::FunctionFloor,
    ),
    ("<apply><ln/><cn>1</cn></apply>", AstKind::FunctionLn),
    ("<apply><log/><cn>100</cn></apply>", AstKind::FunctionLog),
    (
        "<apply><max/><cn>1</cn><cn>2</cn></apply>",
        AstKind::FunctionMax,
    ),
    (
        "<apply><min/><cn>1</cn><cn>2</cn></apply>",
        AstKind::FunctionMin,
    ),
    (
        "<piecewise><piece><cn>1</cn><true/></piece><otherwise><cn>0</cn></otherwise></piecewise>",
        AstKind::FunctionPiecewise,
    ),
    (
        "<apply><power/><cn>2</cn><cn>3</cn></apply>",
        AstKind::FunctionPower,
    ),
    (
        "<apply><quotient/><cn type=\"integer\">7</cn><cn type=\"integer\">2</cn></apply>",
        AstKind::FunctionQuotient,
    ),
    (
        "<apply><csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/rateOf\">rateOf</csymbol><ci>S1</ci></apply>",
        AstKind::FunctionRateOf,
    ),
    (
        "<apply><rem/><cn type=\"integer\">7</cn><cn type=\"integer\">2</cn></apply>",
        AstKind::FunctionRem,
    ),
    ("<apply><root/><cn>4</cn></apply>", AstKind::FunctionRoot),
    ("<apply><sec/><cn>1</cn></apply>", AstKind::FunctionSec),
    ("<apply><sech/><cn>1</cn></apply>", AstKind::FunctionSech),
    ("<apply><sin/><cn>1</cn></apply>", AstKind::FunctionSin),
    ("<apply><sinh/><cn>1</cn></apply>", AstKind::FunctionSinh),
    ("<apply><tan/><cn>1</cn></apply>", AstKind::FunctionTan),
    ("<apply><tanh/><cn>1</cn></apply>", AstKind::FunctionTanh),
    (
        "<lambda><bvar><ci>x</ci></bvar><ci>x</ci></lambda>",
        AstKind::Lambda,
    ),
    (
        "<apply><eq/><cn>1</cn><cn>2</cn></apply>",
        AstKind::RelationalEq,
    ),
    (
        "<apply><geq/><cn>1</cn><cn>2</cn></apply>",
        AstKind::RelationalGeq,
    ),
    (
        "<apply><gt/><cn>1</cn><cn>2</cn></apply>",
        AstKind::RelationalGt,
    ),
    (
        "<apply><leq/><cn>1</cn><cn>2</cn></apply>",
        AstKind::RelationalLeq,
    ),
    (
        "<apply><lt/><cn>1</cn><cn>2</cn></apply>",
        AstKind::RelationalLt,
    ),
    (
        "<apply><neq/><cn>1</cn><cn>2</cn></apply>",
        AstKind::RelationalNeq,
    ),
    ("<apply><and/><true/><false/></apply>", AstKind::LogicalAnd),
    ("<apply><not/><true/></apply>", AstKind::LogicalNot),
    ("<apply><or/><true/><false/></apply>", AstKind::LogicalOr),
    ("<apply><xor/><true/><false/></apply>", AstKind::LogicalXor),
    (
        "<apply><implies/><true/><false/></apply>",
        AstKind::LogicalImplies,
    ),
    (
        "<apply><csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/distrib/bernoulli\">bernoulli</csymbol><cn>0.5</cn></apply>",
        AstKind::DistribBernoulli,
    ),
    (
        "<apply><csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/distrib/binomial\">binomial</csymbol><cn type=\"integer\">10</cn><cn>0.5</cn></apply>",
        AstKind::DistribBinomial,
    ),
    (
        "<apply><csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/distrib/cauchy\">cauchy</csymbol><cn>0</cn><cn>1</cn></apply>",
        AstKind::DistribCauchy,
    ),
    (
        "<apply><csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/distrib/chisquare\">chisquare</csymbol><cn>1</cn></apply>",
        AstKind::DistribChisquare,
    ),
    (
        "<apply><csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/distrib/exponential\">exponential</csymbol><cn>1</cn></apply>",
        AstKind::DistribExponential,
    ),
    (
        "<apply><csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/distrib/gamma\">gamma</csymbol><cn>1</cn><cn>1</cn></apply>",
        AstKind::DistribGamma,
    ),
    (
        "<apply><csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/distrib/laplace\">laplace</csymbol><cn>0</cn><cn>1</cn></apply>",
        AstKind::DistribLaplace,
    ),
    (
        "<apply><csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/distrib/lognormal\">lognormal</csymbol><cn>0</cn><cn>1</cn></apply>",
        AstKind::DistribLognormal,
    ),
    (
        "<apply><csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/distrib/normal\">normal</csymbol><cn>0</cn><cn>1</cn></apply>",
        AstKind::DistribNormal,
    ),
    (
        "<apply><csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/distrib/poisson\">poisson</csymbol><cn>1</cn></apply>",
        AstKind::DistribPoisson,
    ),
    (
        "<apply><csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/distrib/rayleigh\">rayleigh</csymbol><cn>1</cn></apply>",
        AstKind::DistribRayleigh,
    ),
    (
        "<apply><csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/distrib/uniform\">uniform</csymbol><cn>0</cn><cn>1</cn></apply>",
        AstKind::DistribUniform,
    ),
];

#[test]
fn the_kinetic_law_of_a_reaction_is_filled_as_a_tree() {
    // k1 * S1 / (K + S1): divide at the root, times left, plus right
    let ast = read_math(kinetic_law_math()).expect("the math of the kinetic law parses");
    assert_eq!(ast.kind, AstKind::Divide);
    assert_eq!(ast.children.len(), 2);
    assert_eq!(ast.children[0].kind, AstKind::Times);
    assert_eq!(ast.children[1].kind, AstKind::Plus);
    assert_eq!(ast.children[0].children[0].name.as_deref(), Some("k1"));
    assert_eq!(ast.children[0].children[1].name.as_deref(), Some("S1"));
    assert_eq!(ast.children[1].children[0].name.as_deref(), Some("K"));
}

#[test]
fn content_mathml_parses_to_the_same_tree() {
    let ast = read_math(
        "<math xmlns=\"http://www.w3.org/1998/Math/MathML\"><apply><times/><ci>k1</ci><ci>S1</ci></apply></math>",
    )
    .expect("it parses");
    assert_eq!(ast.kind, AstKind::Times);
    assert_eq!(ast.children.len(), 2);
    assert_eq!(ast.children[0].kind, AstKind::Name);
    assert_eq!(ast.children[0].name.as_deref(), Some("k1"));
}

#[test]
fn a_number_keeps_its_kind_and_its_units() {
    // `sbml:units` is an attribute of Level 3, so the math declares the
    // namespace of core, and a `cn` without a `type` is a real, so the
    // integer states its type
    let ast = read_math(concat!(
        "<math xmlns=\"http://www.w3.org/1998/Math/MathML\"",
        " xmlns:sbml=\"http://www.sbml.org/sbml/level3/version1/core\">",
        "<apply><plus/>",
        "<cn type=\"integer\" sbml:units=\"mg\"> 2 </cn>",
        "<cn type=\"e-notation\"> 1.35 <sep/> -5 </cn>",
        "</apply></math>",
    ))
    .expect("it parses");
    assert_eq!(ast.children[0].kind, AstKind::Integer);
    assert_eq!(ast.children[0].number, Some(AstNumber::Integer(2)));
    assert_eq!(ast.children[0].units.as_deref(), Some("mg"));
    assert_eq!(ast.children[1].kind, AstKind::RealE);
    assert_eq!(
        ast.children[1].number,
        Some(AstNumber::RealE {
            mantissa: 1.35,
            exponent: -5
        })
    );
    assert_eq!(ast.children[1].units, None);
}

#[test]
fn a_rational_and_a_real_keep_their_parts() {
    let ast = read_math(&math("<cn type=\"rational\">3<sep/>4</cn>")).expect("it parses");
    assert_eq!(
        ast.number,
        Some(AstNumber::Rational {
            numerator: 3,
            denominator: 4
        })
    );
    let ast = read_math(&math("<cn>1.5</cn>")).expect("it parses");
    assert_eq!(ast.number, Some(AstNumber::Real(1.5)));
    let ast = read_math(&math("<ci>S1</ci>")).expect("it parses");
    assert_eq!(ast.number, None);
}

#[test]
fn a_csymbol_carries_the_name_libsbml_reports_of_it() {
    let time = read_math(&math(
        "<csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/time\">time</csymbol>",
    ))
    .expect("it parses");
    assert_eq!(time.kind, AstKind::NameTime);
    assert_eq!(time.name.as_deref(), Some("time"));
    let avogadro = read_math(&math(
        "<csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/avogadro\">avogadro</csymbol>",
    ))
    .expect("it parses");
    assert_eq!(avogadro.kind, AstKind::NameAvogadro);
    assert_eq!(avogadro.name.as_deref(), Some("avogadro"));
}

#[test]
fn a_name_is_the_one_libsbml_answers_of_the_node() {
    // a call of a function of the model carries the name of the function
    let call = read_math(&math("<apply><ci>f</ci><cn>1</cn></apply>")).expect("it parses");
    assert_eq!(call.kind, AstKind::Function);
    assert_eq!(call.name.as_deref(), Some("f"));
    assert_eq!(call.children.len(), 1);
    // a built-in carries the name libsbml gives it of its own
    let abs = read_math(&math("<apply><abs/><cn>1</cn></apply>")).expect("it parses");
    assert_eq!(abs.name.as_deref(), Some("abs"));
    // an operator and a number carry none
    let plus = read_math(&math("<apply><plus/><cn>1</cn><cn>2</cn></apply>")).expect("it parses");
    assert_eq!(plus.name, None);
    assert_eq!(plus.children[0].name, None);
}

#[test]
fn a_csymbol_of_a_definition_url_libsbml_does_not_know_is_a_csymbol_function() {
    // `AST_CSYMBOL_FUNCTION` is the type of a csymbol of a definition URL no
    // package of libsbml registers, which carries the text of the document and
    // which libsbml and the stylesheet of the latex write as a call of that
    // text, so the report has a kind of it
    let body =
        "<csymbol encoding=\"text\" definitionURL=\"http://example.org/of/nothing\">x</csymbol>";
    let ast = read_math(&math(body)).expect("it parses");
    assert_eq!(ast.kind, AstKind::CsymbolFunction);
    assert_eq!(ast.name.as_deref(), Some("x"));
}

#[test]
fn an_empty_math_is_the_unknown_node_libsbml_answers_of_it() {
    // `AST_UNKNOWN` is the node libsbml answers of a math it could not read, of
    // which an empty `math` element is the one a string reaches: libsbml answers
    // it with no child and reports the element as having a math. The python
    // implementation reports such a document, with the formula `()` and a latex
    // of one space, so the report has a kind of it and refuses nothing.
    let ast = read_math(&math("")).expect("an empty math reads");
    assert_eq!(ast.kind, AstKind::Unknown);
    assert_eq!(ast.name, None);
    assert!(ast.children.is_empty());
}

#[test]
fn every_kind_of_a_content_mathml_is_filled_as_that_kind() {
    let mut wrong: Vec<String> = Vec::new();
    for (body, kind) in KINDS {
        match read_math(&math(body)) {
            Ok(ast) if ast.kind == *kind => {}
            Ok(ast) => wrong.push(format!("{body} is {:?} and not {kind:?}", ast.kind)),
            Err(error) => wrong.push(format!("{body} of {kind:?} does not parse: {error}")),
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn the_kinds_of_the_table_are_every_kind_but_other_and_power() {
    let missing: Vec<AstKind> = AstKind::ALL
        .into_iter()
        .filter(|kind| !KINDS.iter().any(|(_, mapped)| mapped == kind))
        .collect();
    assert_eq!(missing, vec![AstKind::Other, AstKind::Power]);
    // `dedup` drops only neighbours, so the sort is what makes this the check
    // its message states
    let mut kinds: Vec<i32> = KINDS.iter().map(|(_, kind)| *kind as i32).collect();
    kinds.sort_unstable();
    kinds.dedup();
    assert_eq!(kinds.len(), KINDS.len(), "a kind is in the table twice");
}

#[test]
fn the_power_of_the_formula_of_a_level_1_kinetic_law_is_filled_as_a_power() {
    // `Power` is the one kind no content MathML yields, where `<power/>` is
    // `FunctionPower`. It is the `^` of the `formula` a Level 1 document
    // writes its math as, which `KineticLaw::getMath` parses with the very
    // call `read_formula` makes, `SBML_parseFormula` of the formula of the
    // kinetic law (`KineticLaw.cpp`)
    let source = include_str!("../testdata/level1_power.xml");
    let document = read(source.as_bytes()).expect("libsbml answers a report of the document");
    assert_eq!((document.level, document.version), (1, 2));
    // the tree of the kinetic law of the document itself, so that this is the
    // one check that the path through the shim fills the `formula` as a math
    let model = document.model.expect("the document holds a model");
    let law = model.reactions[0]
        .kinetic_law
        .as_ref()
        .expect("the reaction holds a kinetic law");
    let ast = law.math.as_ref().expect("the law holds a math");
    assert_eq!(ast.kind, AstKind::Times);
    assert_eq!(ast.children[0].name.as_deref(), Some("k1"));
    let power = &ast.children[1];
    assert_eq!(power.kind, AstKind::Power);
    assert_eq!(power.children[0].name.as_deref(), Some("S1"));
    assert_eq!(power.children[1].number, Some(AstNumber::Integer(2)));
    // and the same tree read from the formula string beside it, which is the
    // very call `KineticLaw::getMath` makes of it
    assert_eq!(
        read_formula(level1_formula()).expect("the formula parses"),
        *ast
    );
}

#[test]
fn a_string_which_is_no_mathml_is_an_error_and_never_a_panic() {
    for s in ["", "<math", "<html></html>", "not xml at all", "a\0b"] {
        assert!(read_math(s).is_err(), "{s} must not parse");
    }
}

#[test]
fn a_string_which_is_no_formula_is_an_error_and_never_a_panic() {
    for s in ["", "k1 *", "(", "1 +", "<html></html>", "a\0b"] {
        assert!(read_formula(s).is_err(), "{s} must not parse");
    }
}

#[test]
fn a_tree_as_deep_as_the_bound_is_filled() {
    let ast = read_math(&nested(MAX_AST_DEPTH - 1)).expect("it parses");
    let mut depth = 1;
    let mut node = &ast;
    while let Some(child) = node.children.last() {
        depth += 1;
        node = child;
    }
    assert_eq!(depth, MAX_AST_DEPTH);
}

#[test]
fn the_math_reads_of_many_threads_do_not_race() {
    // the read of a MathML fills the registry of the definition URLs of
    // libsbml, which is global state, so it holds the lock of the crate
    let readers: Vec<_> = (0..8)
        .map(|_| {
            std::thread::spawn(|| {
                read_math(kinetic_law_math())
                    .expect("the math of the kinetic law parses")
                    .children
                    .len()
            })
        })
        .collect();
    for reader in readers {
        assert_eq!(reader.join().unwrap(), 2);
    }
}

#[test]
fn a_tree_deeper_than_the_bound_is_an_error() {
    let deep = nested(MAX_AST_DEPTH);
    assert!(matches!(read_math(&deep), Err(ReadError::MathTooDeep)));
}

/// A width at which a free which followed the tree on the call stack would end
/// the process.
///
/// `~ASTNode` of libsbml deletes the children of a node itself, which follows a
/// tree one call frame per level: a plain `delete` of a chain of 300,000 nodes
/// ends the process with SIGSEGV on the 8 MiB stack of a main thread, and a
/// test thread gets 2 MiB. `s4h_ast_free` of the shim therefore takes a tree
/// apart on a stack of its own, and this is the width which pins it.
const PAST_A_RECURSIVE_FREE: usize = 200_000;

#[test]
fn the_tree_of_a_math_far_wider_than_the_bound_is_freed_without_ending_the_process() {
    // Whatever the bound answers of this width, libsbml has built the tree by
    // then and the read frees it on its way out, which is what this pins: the
    // test ends rather than aborting, and every other free of the crate takes
    // the same path.
    let _ = read_math(&flat_sum(PAST_A_RECURSIVE_FREE));
    // the parser of a Level 1 formula builds the same chain, whose tree is
    // freed the same way
    let mut formula = String::from("1");
    for _ in 1..PAST_A_RECURSIVE_FREE {
        formula.push_str("+1");
    }
    let _ = read_formula(&formula);
}

#[test]
fn the_bound_is_reached_by_the_width_of_a_flat_sum_and_not_by_its_nesting_alone() {
    // The MathML reader of libsbml nests a flat n-ary application into a binary
    // tree, one `apply` per operand after the first and the nesting on the left
    // operand, so the tree of a flat sum of `n` terms is `n` levels deep and the
    // bound of the reader is a bound on the width of a sum of a document as much
    // as on its nesting. An ordinary document reaches it: a sum of 1,001 terms is
    // nothing unusual and no adversarial shape.
    //
    // Measured here, and the figures the doc comment of `MAX_AST_DEPTH` and of
    // `ReadError::MathTooDeep` carry: a flat sum of 1,000 terms is read as a tree
    // of 1,000 levels and one of 1,001 terms is `MathTooDeep`. The python
    // implementation reports such a document in full and with its latex, because
    // `writeMathML` unrolls the nesting back into one flat `apply` and the lxml
    // pipeline it feeds sees a document of three elements.
    let at_the_bound = read_math(&flat_sum(MAX_AST_DEPTH)).expect("a flat sum of the bound");
    assert_eq!(depth_of(&at_the_bound), MAX_AST_DEPTH);
    assert_eq!(at_the_bound.kind, AstKind::Plus);
    // two operands at every level, the sum of the terms before it and the next
    // term, which is what makes the depth the number of the terms
    assert_eq!(at_the_bound.children.len(), 2);
    assert_eq!(
        at_the_bound.children[0].kind,
        AstKind::Plus,
        "the nesting is on the left operand"
    );
    assert!(matches!(
        read_math(&flat_sum(MAX_AST_DEPTH + 1)),
        Err(ReadError::MathTooDeep)
    ));
}

#[test]
fn a_number_whose_value_is_not_finite_is_filled_with_that_value() {
    // `writeCN` of libsbml writes a `notanumber`, an `infinity` or an apply of
    // a minus and an infinity for every node `ASTNode::isReal()` covers whose
    // value is not finite, and `isReal()` is `AST_REAL`, `AST_REAL_E` and
    // `AST_RATIONAL`. These are the three ways a document reaches such a node,
    // which the latex writer of `crates/report` has to recognise; this pins
    // that the fill side really produces them.
    let rational = read_math(&math("<cn type=\"rational\"> 3 <sep/> 0 </cn>"))
        .expect("a rational of a zero denominator is content MathML");
    assert_eq!(rational.kind, AstKind::Rational);
    assert_eq!(
        rational.number,
        Some(AstNumber::Rational {
            numerator: 3,
            denominator: 0,
        })
    );
    assert!(rational.children.is_empty());

    // the `formula` of a Level 1 math, which `KineticLaw::getMath` parses with
    // the call `read_formula` makes
    let infinite = read_formula("-INF").expect("the formula parses");
    assert_eq!(infinite.kind, AstKind::Real);
    assert_eq!(infinite.number, Some(AstNumber::Real(f64::NEG_INFINITY)));
    assert!(infinite.children.is_empty());

    let overflowed = read_formula("-1e400").expect("the formula parses");
    assert_eq!(overflowed.kind, AstKind::RealE);
    assert_eq!(
        overflowed.number,
        Some(AstNumber::RealE {
            mantissa: -1.0,
            exponent: 400,
        })
    );
    assert!(overflowed.children.is_empty());
}

#[test]
fn the_name_of_a_csymbol_is_the_text_the_document_wrote() {
    // libsbml answers the text of the `csymbol` element, not the word it knows
    // the csymbol under, and the two writers of `crates/report` differ on that
    // field on purpose: the formula writer ignores it and the latex writer
    // typesets it
    let time = read_math(&math(
        "<csymbol encoding=\"text\" definitionURL=\"http://www.sbml.org/sbml/symbols/time\"> \
         t </csymbol>",
    ))
    .expect("a csymbol is content MathML");
    assert_eq!(time.kind, AstKind::NameTime);
    assert_eq!(time.name.as_deref(), Some("t"));
}
