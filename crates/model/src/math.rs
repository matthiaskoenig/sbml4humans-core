//! The syntax tree of the math of an element.

use std::fmt;

/// The deepest nesting of a syntax tree which a reader or a writer follows.
///
/// # What the bound costs
///
/// It is reached by the **width** of a math and not by its nesting alone: the
/// MathML reader of libsbml nests a flat n-ary application into a binary tree
/// to the left, one `apply` per operand after the first and each of them
/// holding the sum of the terms before it and the next term, so a flat sum of
/// a thousand terms is a tree of exactly a thousand levels and a flat sum of
/// one term more is refused.
/// A wide sum is ordinary in a model and no adversarial shape, and the python
/// implementation reports such a document in full and with its latex, because
/// `writeMathML` unrolls the nesting back into one flat `apply`, so the lxml
/// pipeline it feeds sees a document of three elements. The bound is therefore
/// a difference from the python implementation and not only a guard, and the
/// refusal of one math ends the read of the whole document:
/// `the_bound_is_reached_by_the_width_of_a_flat_sum_and_not_by_its_nesting_alone`
/// of `crates/libsbml/tests/math.rs` pins both answers at 1,000 and 1,001
/// terms, and `a_wide_sum_is_reported_in_full_up_to_the_bound_and_costs_the_whole_report_past_it`
/// of `crates/report/tests/build.rs` pins what one term more costs a report.
///
/// What keeps the bound this low is that two walks of a tree still recurse per
/// level: `ast_of` of `crates/libsbml/src/math.rs`, which fills a tree, and
/// `operands` of `crates/report/src/math/latex.rs`, which unrolls a chain of
/// sums or of products. A stack which runs out is no error but an abort of the
/// process, and in WebAssembly the end of the instance, so neither may follow
/// a tree of any depth. Raising the bound belongs to the slice which makes
/// those two walks non-recursive, which the port owes before the wheel ships.
pub const MAX_AST_DEPTH: usize = 1000;

/// Declares [`AstKind`] with its codes, [`AstKind::ALL`] and [`AstKind::from_code`]
/// from one list, so the three cannot disagree.
macro_rules! ast_kinds {
    ($($(#[$meta:meta])* $name:ident = $code:literal,)*) => {
        /// The kind of a node of a syntax tree.
        ///
        /// The explicit codes are the contract with the libsbml shim, which maps
        /// the `ASTNodeType_t` of libsbml onto them. Nothing else may depend on a
        /// particular code, apart from `Other` being 0.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
        #[repr(i32)]
        pub enum AstKind {
            $($(#[$meta])* $name = $code,)*
        }

        impl AstKind {
            /// Every kind, exactly once.
            pub const ALL: [AstKind; [$($code),*].len()] = [$(AstKind::$name),*];

            /// The kind of a code, `Other` for a code no kind has.
            pub fn from_code(code: i32) -> AstKind {
                match code {
                    $($code => AstKind::$name,)*
                    _ => AstKind::Other,
                }
            }
        }
    };
}

ast_kinds! {
    /// A type the shim does not name, which the writers refuse.
    #[default]
    Other = 0,
    /// The node libsbml answers of a math it could not read, with whatever stood below it.
    ///
    /// libsbml writes no mathml element of such a node, so the latex of a tree
    /// which holds one is the latex of the tree without it, while the formula
    /// writes it as its children between one pair of parentheses.
    Unknown = 1,
    /// A reference to a symbol by id, a species, parameter, compartment or local name.
    Name = 100,
    /// The csymbol time, the simulation time, which libsbml reports as a name.
    NameTime = 101,
    /// The csymbol avogadro, the Avogadro constant, which libsbml reports as a name.
    NameAvogadro = 102,
    /// An integer number.
    Integer = 200,
    /// A real number.
    Real = 201,
    /// A real number in exponent notation, a mantissa and an exponent.
    RealE = 202,
    /// A rational number, a numerator and a denominator.
    Rational = 203,
    /// The constant e, the base of the natural logarithm.
    ConstantE = 300,
    /// The boolean constant false.
    ConstantFalse = 301,
    /// The constant pi.
    ConstantPi = 302,
    /// The boolean constant true.
    ConstantTrue = 303,
    /// A sum of its children.
    Plus = 400,
    /// A difference, or the negation of a single child.
    Minus = 401,
    /// A product of its children.
    Times = 402,
    /// A quotient of two children.
    Divide = 403,
    /// A power written with the `^` of the `formula` of a Level 1 math, which renders like `FunctionPower`.
    Power = 404,
    /// A call of a user function, whose name is in `name`.
    Function = 500,
    /// A call of a csymbol of a definition URL libsbml does not know, whose text is in `name`.
    CsymbolFunction = 501,
    /// The absolute value of its child.
    FunctionAbs = 600,
    /// The inverse cosine of its child.
    FunctionArccos = 601,
    /// The inverse hyperbolic cosine of its child.
    FunctionArccosh = 602,
    /// The inverse cotangent of its child.
    FunctionArccot = 603,
    /// The inverse hyperbolic cotangent of its child.
    FunctionArccoth = 604,
    /// The inverse cosecant of its child.
    FunctionArccsc = 605,
    /// The inverse hyperbolic cosecant of its child.
    FunctionArccsch = 606,
    /// The inverse secant of its child.
    FunctionArcsec = 607,
    /// The inverse hyperbolic secant of its child.
    FunctionArcsech = 608,
    /// The inverse sine of its child.
    FunctionArcsin = 609,
    /// The inverse hyperbolic sine of its child.
    FunctionArcsinh = 610,
    /// The inverse tangent of its child.
    FunctionArctan = 611,
    /// The inverse hyperbolic tangent of its child.
    FunctionArctanh = 612,
    /// The smallest integer not below its child.
    FunctionCeiling = 613,
    /// The cosine of its child.
    FunctionCos = 614,
    /// The hyperbolic cosine of its child.
    FunctionCosh = 615,
    /// The cotangent of its child.
    FunctionCot = 616,
    /// The hyperbolic cotangent of its child.
    FunctionCoth = 617,
    /// The cosecant of its child.
    FunctionCsc = 618,
    /// The hyperbolic cosecant of its child.
    FunctionCsch = 619,
    /// A delayed value, the value of its first child a time given by its second child ago.
    FunctionDelay = 620,
    /// The exponential function of its child.
    FunctionExp = 621,
    /// The factorial of its child.
    FunctionFactorial = 622,
    /// The largest integer not above its child.
    FunctionFloor = 623,
    /// The natural logarithm of its child.
    FunctionLn = 624,
    /// A logarithm, to base 10 or to the base given by its first child.
    FunctionLog = 625,
    /// The maximum of its children, from the l3v2extendedmath set.
    FunctionMax = 626,
    /// The minimum of its children, from the l3v2extendedmath set.
    FunctionMin = 627,
    /// A piecewise defined value, a value and a condition per case and an optional value otherwise.
    FunctionPiecewise = 628,
    /// A power written as `power` in MathML, its first child raised to its second.
    FunctionPower = 629,
    /// The integer quotient of two children, from the l3v2extendedmath set.
    FunctionQuotient = 630,
    /// The rate of change of a symbol, from the l3v2extendedmath set.
    FunctionRateOf = 631,
    /// The remainder of the division of two children, from the l3v2extendedmath set.
    FunctionRem = 632,
    /// A root of its child, the square root or the degree given by the first child.
    FunctionRoot = 633,
    /// The secant of its child.
    FunctionSec = 634,
    /// The hyperbolic secant of its child.
    FunctionSech = 635,
    /// The sine of its child.
    FunctionSin = 636,
    /// The hyperbolic sine of its child.
    FunctionSinh = 637,
    /// The tangent of its child.
    FunctionTan = 638,
    /// The hyperbolic tangent of its child.
    FunctionTanh = 639,
    /// A lambda, the bound variables as children followed by the body.
    Lambda = 700,
    /// True when its children are equal.
    RelationalEq = 800,
    /// True when each child is at least the next.
    RelationalGeq = 801,
    /// True when each child is greater than the next.
    RelationalGt = 802,
    /// True when each child is at most the next.
    RelationalLeq = 803,
    /// True when each child is less than the next.
    RelationalLt = 804,
    /// True when its two children differ.
    RelationalNeq = 805,
    /// True when all its children are true.
    LogicalAnd = 900,
    /// The negation of its child.
    LogicalNot = 901,
    /// True when any of its children is true.
    LogicalOr = 902,
    /// True when an odd number of its children are true.
    LogicalXor = 903,
    /// The logical implication of the math of Level 3 Version 2, whose first child implies its second.
    LogicalImplies = 904,
    /// A draw from the bernoulli distribution of the distrib package.
    DistribBernoulli = 1000,
    /// A draw from the binomial distribution of the distrib package.
    DistribBinomial = 1001,
    /// A draw from the cauchy distribution of the distrib package.
    DistribCauchy = 1002,
    /// A draw from the chisquare distribution of the distrib package.
    DistribChisquare = 1003,
    /// A draw from the exponential distribution of the distrib package.
    DistribExponential = 1004,
    /// A draw from the gamma distribution of the distrib package.
    DistribGamma = 1005,
    /// A draw from the laplace distribution of the distrib package.
    DistribLaplace = 1006,
    /// A draw from the lognormal distribution of the distrib package.
    DistribLognormal = 1007,
    /// A draw from the normal distribution of the distrib package.
    DistribNormal = 1008,
    /// A draw from the poisson distribution of the distrib package.
    DistribPoisson = 1009,
    /// A draw from the rayleigh distribution of the distrib package.
    DistribRayleigh = 1010,
    /// A draw from the uniform distribution of the distrib package.
    DistribUniform = 1011,
}

/// The value of a number node.
#[derive(Debug, Clone, PartialEq)]
pub enum AstNumber {
    /// An integer.
    Integer(i64),
    /// A real number.
    Real(f64),
    /// A real number in exponent notation.
    RealE {
        /// The mantissa.
        mantissa: f64,
        /// The exponent, a power of ten.
        exponent: i64,
    },
    /// A rational number.
    Rational {
        /// The numerator.
        numerator: i64,
        /// The denominator.
        denominator: i64,
    },
}

/// A node of the syntax tree of a math.
///
/// A tree may be deeper than a call stack holds one frame per level of, so
/// [`Drop`], [`Clone`], [`PartialEq`] and [`fmt::Debug`] are written here
/// rather than derived: the glue a derive writes follows the children on the
/// call stack, and a stack which runs out is no error but an abort of the
/// process and, in WebAssembly, the end of the instance. Each of the four
/// walks the tree on a stack of its own on the heap, so what a tree costs any
/// of them is memory and never a frame, whatever [`MAX_AST_DEPTH`] is.
///
/// Writing [`Drop`] is what makes the type one no field can be moved out of, so
/// a node is built by naming all of its fields and not with `..Ast::default()`.
#[derive(Default)]
pub struct Ast {
    /// The kind of the node.
    pub kind: AstKind,
    /// The name of a symbol or of a user function.
    pub name: Option<String>,
    /// The value of a number node.
    pub number: Option<AstNumber>,
    /// The units of a number node.
    pub units: Option<String>,
    /// The children, in the order of the document.
    pub children: Vec<Ast>,
}

impl Ast {
    /// The node with its payload and without its children.
    fn without_children(&self) -> Ast {
        Ast {
            kind: self.kind,
            name: self.name.clone(),
            number: self.number.clone(),
            units: self.units.clone(),
            children: Vec::with_capacity(self.children.len()),
        }
    }
}

impl Drop for Ast {
    /// Drops the tree below the node without a call frame per level.
    ///
    /// The children of the node are taken and emptied one after the other on a
    /// stack of their own: a node whose children were taken drops with nothing
    /// below it, so the glue the compiler writes for this type never reaches a
    /// second level. The stack holds the children of one node at a time, which
    /// for a chain is one node.
    fn drop(&mut self) {
        let mut rest = std::mem::take(&mut self.children);
        while let Some(mut node) = rest.pop() {
            rest.append(&mut node.children);
        }
    }
}

impl Clone for Ast {
    /// Clones the tree below the node without a call frame per level.
    fn clone(&self) -> Self {
        // the node being cloned, how many of its children are done, and what
        // is built of its clone so far
        let mut stack: Vec<(&Ast, usize, Ast)> = vec![(self, 0, self.without_children())];
        loop {
            let (source, next, _) = stack.last_mut().expect("the frame of the root");
            if let Some(child) = source.children.get(*next) {
                *next += 1;
                stack.push((child, 0, child.without_children()));
                continue;
            }
            let (_, _, built) = stack.pop().expect("the frame which is done");
            match stack.last_mut() {
                Some((_, _, parent)) => parent.children.push(built),
                None => return built,
            }
        }
    }
}

impl PartialEq for Ast {
    /// Compares the two trees without a call frame per level.
    ///
    /// The pairs still to compare are kept on a stack of their own, so the
    /// answer is the one a derived comparison gives: two nodes are equal when
    /// their payloads are and their children are equal pairwise and in order.
    fn eq(&self, other: &Self) -> bool {
        let mut pairs = vec![(self, other)];
        while let Some((ours, theirs)) = pairs.pop() {
            if ours.kind != theirs.kind
                || ours.name != theirs.name
                || ours.number != theirs.number
                || ours.units != theirs.units
                || ours.children.len() != theirs.children.len()
            {
                return false;
            }
            pairs.extend(ours.children.iter().zip(theirs.children.iter()));
        }
        true
    }
}

/// One piece of the text [`fmt::Debug`] of an [`Ast`] writes.
enum DebugPiece<'a> {
    /// A node, written with its payload and its children.
    Node(&'a Ast),
    /// Literal text.
    Text(&'static str),
}

impl fmt::Debug for Ast {
    /// Writes the tree without a call frame per level.
    ///
    /// The text is the one a derived [`fmt::Debug`] writes of the plain form,
    /// `{:?}`. The alternate form `{:#?}` writes that same text rather than the
    /// indented one a derive writes, since the indentation of a tree of a
    /// million levels is a million levels of indentation and nothing reads it:
    /// what this form is for is a tree a test prints when it fails.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut pieces = vec![DebugPiece::Node(self)];
        while let Some(piece) = pieces.pop() {
            let node = match piece {
                DebugPiece::Text(text) => {
                    f.write_str(text)?;
                    continue;
                }
                DebugPiece::Node(node) => node,
            };
            write!(
                f,
                "Ast {{ kind: {:?}, name: {:?}, number: {:?}, units: {:?}, children: [",
                node.kind, node.name, node.number, node.units
            )?;
            pieces.push(DebugPiece::Text("] }"));
            for (n, child) in node.children.iter().enumerate().rev() {
                pieces.push(DebugPiece::Node(child));
                if n > 0 {
                    pieces.push(DebugPiece::Text(", "));
                }
            }
        }
        Ok(())
    }
}
