//! Filling the syntax tree of a math with the getters of libsbml.
//!
//! Nothing here may end a read of a math a request brought: the fill of a tree
//! which is nested deeper than [`MAX_AST_DEPTH`] answers an error rather than
//! following it, since the stack a deeper one would need is no error but an
//! abort of the process and, in WebAssembly, the end of the instance.

use std::ffi::{CString, c_double, c_int, c_longlong};

use sbml4humans_model::{Ast, AstKind, AstNumber, MAX_AST_DEPTH};

use crate::fill::{ReadError, locked};
use crate::strings;
use crate::sys;

/// The number of a node is an integer, `s4h_ast_number` wrote the integer.
const NUMBER_INTEGER: c_int = 1;
/// The number of a node is a real, `s4h_ast_number` wrote the real.
const NUMBER_REAL: c_int = 2;
/// The number of a node is in e-notation, with the real as its mantissa.
const NUMBER_REAL_E: c_int = 3;
/// The number of a node is a rational, with the integer as its numerator.
const NUMBER_RATIONAL: c_int = 4;

/// A syntax tree of libsbml, freed when the fill of it ends.
struct Parsed(*mut sys::ASTNode);

impl Drop for Parsed {
    fn drop(&mut self) {
        unsafe { sys::s4h_ast_free(self.0) };
    }
}

/// The syntax tree of a math, as libsbml reads its content MathML.
///
/// The string is the `math` element of a math of SBML with its namespace, the
/// one libsbml itself writes of a tree. Every string which is no content
/// MathML, and every tree which is nested deeper than [`MAX_AST_DEPTH`], is an
/// error; nothing panics, whatever the string holds.
pub fn read_math(mathml: &str) -> Result<Ast, ReadError> {
    let xml = CString::new(mathml).map_err(|_| {
        ReadError::Libsbml("the math holds a nul byte, which no XML document holds".to_owned())
    })?;
    // libsbml keeps global state, the registry of the definition URLs of the
    // csymbols of its packages, which the read of a MathML fills, among it
    let _lock = locked();
    let parsed = Parsed(unsafe { sys::s4h_read_math_from_string(xml.as_ptr()) });
    if parsed.0.is_null() {
        return Err(ReadError::Libsbml(
            "libsbml read no math of the string".to_owned(),
        ));
    }
    unsafe { ast_of(parsed.0, 1) }
}

/// The node at level `depth` of a syntax tree of libsbml and its children.
///
/// `name` holds the name libsbml answers of the node: the identifier a `ci`
/// refers to, the name of a call of a function of the model, and the text of a
/// csymbol, which is `time` for the time csymbol and `avogadro` for the
/// Avogadro one, so that the kinds `NameTime` and `NameAvogadro` carry the
/// name the writers typeset. A built-in function, a constant and a lambda
/// carry the name libsbml gives them of its own, an operator and a number
/// carry none.
///
/// `number` and `units` are the value and the `sbml:units` of a number node,
/// and `None` of every other node.
///
/// # Safety
///
/// `node` is a node of a syntax tree of libsbml which is alive, and the caller
/// holds the lock of libsbml.
pub(crate) unsafe fn ast_of(node: *mut sys::ASTNode, depth: usize) -> Result<Ast, ReadError> {
    if depth > MAX_AST_DEPTH {
        return Err(ReadError::MathTooDeep);
    }
    unsafe {
        // a child libsbml does not answer is left out, so that the number it
        // states of its children cannot make the fill ask for one which is
        // not there
        let mut children = Vec::new();
        for n in 0..sys::s4h_ast_num_children(node) {
            let child = sys::s4h_ast_child(node, n);
            if !child.is_null() {
                children.push(ast_of(child, depth + 1)?);
            }
        }
        Ok(Ast {
            kind: AstKind::from_code(sys::s4h_ast_kind(node)),
            name: strings::borrowed(sys::s4h_ast_name(node)),
            number: number(node),
            units: strings::handed_over(sys::s4h_ast_units(node)),
            children,
        })
    }
}

/// The value of a number node, `None` when the node is no number.
///
/// # Safety
///
/// `node` is a node of a syntax tree of libsbml which is alive.
unsafe fn number(node: *mut sys::ASTNode) -> Option<AstNumber> {
    let mut integer: c_longlong = 0;
    let mut real: c_double = 0.0;
    let mut exponent: c_longlong = 0;
    let mut denominator: c_longlong = 0;
    let tag = unsafe {
        sys::s4h_ast_number(
            node,
            &mut integer,
            &mut real,
            &mut exponent,
            &mut denominator,
        )
    };
    match tag {
        NUMBER_INTEGER => Some(AstNumber::Integer(integer)),
        NUMBER_REAL => Some(AstNumber::Real(real)),
        NUMBER_REAL_E => Some(AstNumber::RealE {
            mantissa: real,
            exponent,
        }),
        NUMBER_RATIONAL => Some(AstNumber::Rational {
            numerator: integer,
            denominator,
        }),
        // 0 is the node of no number, and so is a tag the shim does not write
        _ => None,
    }
}
