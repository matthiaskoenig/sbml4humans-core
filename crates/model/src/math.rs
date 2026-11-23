//! The syntax tree of the math of an element.

/// The deepest nesting of a syntax tree which a reader or a writer follows.
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
#[derive(Debug, Clone, PartialEq, Default)]
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
