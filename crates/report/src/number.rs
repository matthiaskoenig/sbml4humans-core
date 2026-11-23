//! A double written the way python writes it.

/// The `str()` of python 3 for a float, which the readable equation of a reaction is written with.
///
/// Python writes the coefficient of a half equation as an f-string of the float
/// (`SBMLDocumentInfo._half_equation`), which is its `str()`, and that is `format_float_short` of CPython in
/// mode `r`: the shortest digits which round trip, in the fixed form while the decimal point sits in
/// `-4 < decpt <= 16` and in the exponent form outside of those two bounds, the exponent signed and padded
/// to two digits, and a trailing `.0` where the fixed form leaves no fractional digit. The upper bound is 16
/// and not the 17 of python 3.0 and before, so `1e16` is already written as an exponent.
///
/// Rust's `{}` writes the same shortest digits but never the exponent form, so a stoichiometry of `1e-05`
/// would come out as `0.00001` and the equation would differ from the one of python. The digits here are
/// therefore the ones Rust writes, from `{:e}`, and only the form is chosen, so that the two
/// implementations cannot disagree about which representation is the shortest one. The one double they write
/// differently is the one which lies exactly between two shortest representations, as `1333333333333333.25`
/// lies between `1333333333333333.2` and `.3`: Rust rounds that half away from zero and the dtoa of CPython
/// rounds it to even. It takes a step of the double wide enough to leave both of them round tripping, so of
/// a magnitude of about 1e15 and seventeen significant digits, which no stoichiometry of a model carries,
/// and writing the digits here instead would cost an arbitrary precision decimal of every double.
// Nothing calls it yet: the species reference whose stoichiometry it writes lands later in this slice.
// The expectation is unfulfilled as soon as a caller arrives, so the attribute reports itself as stale;
// it is set for the library alone, since the test below is a caller of its own.
#[cfg_attr(not(test), expect(dead_code))]
pub(crate) fn python_float(value: f64) -> String {
    if value.is_nan() {
        return "nan".to_owned();
    }
    if value == f64::INFINITY {
        return "inf".to_owned();
    }
    if value == f64::NEG_INFINITY {
        return "-inf".to_owned();
    }
    // `{:e}` of a finite double always writes `<mantissa>e<exponent>`; the two fallbacks take anything
    // else as an exponent of zero rather than panicking on it.
    let shortest = format!("{value:e}");
    let (mantissa, exponent) = shortest.split_once('e').unwrap_or((shortest.as_str(), "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    // The place of the decimal point within the digits, the `decpt` of CPython. `{:e}` has normalized the
    // mantissa to one digit before the point, so the exponent form is that mantissa with a written exponent.
    let point = exponent + 1;
    if point <= -4 || point > 16 {
        return format!("{mantissa}e{exponent:+03}");
    }
    let (sign, magnitude) = match mantissa.strip_prefix('-') {
        Some(magnitude) => ("-", magnitude),
        None => ("", mantissa),
    };
    let digits: String = magnitude.chars().filter(char::is_ascii_digit).collect();
    if point <= 0 {
        let zeros = "0".repeat(point.unsigned_abs() as usize);
        return format!("{sign}0.{zeros}{digits}");
    }
    let point = point.unsigned_abs() as usize;
    if point >= digits.len() {
        let zeros = "0".repeat(point - digits.len());
        format!("{sign}{digits}{zeros}.0")
    } else {
        let (integer, fraction) = digits.split_at_checked(point).unwrap_or((&digits, ""));
        format!("{sign}{integer}.{fraction}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_float_is_written_as_python_writes_it() {
        // generated from python 3.14: `for v in ...: print(repr(v), str(v))`
        for (value, text) in [
            (1.0, "1.0"),
            (2.0, "2.0"),
            (2.5, "2.5"),
            (0.5, "0.5"),
            (-1.0, "-1.0"),
            (0.0, "0.0"),
            (-0.0, "-0.0"),
            (0.1, "0.1"),
            (0.0001, "0.0001"),
            (1e-5, "1e-05"),
            (1.5e-7, "1.5e-07"),
            (1.0 / 3.0, "0.3333333333333333"),
            (12345.6789, "12345.6789"),
            (1e15, "1000000000000000.0"),
            (1e16, "1e+16"),
            (1e21, "1e+21"),
            (1e100, "1e+100"),
            (123456789012345680.0, "1.2345678901234568e+17"),
            (f64::INFINITY, "inf"),
            (f64::NEG_INFINITY, "-inf"),
        ] {
            assert_eq!(python_float(value), text, "{value:?}");
        }
        assert_eq!(python_float(f64::NAN), "nan");
    }

    #[test]
    fn a_double_between_two_shortest_representations_is_rounded_as_rust_rounds_it() {
        // The double of this literal is exactly 1333333333333333.25, which `1333333333333333.2` and
        // `1333333333333333.3` both round trip to. Python 3.14 writes the half to even and answers the
        // first; the digits here are Rust's, which rounds the half away from zero, and the doc comment
        // says why they stay Rust's.
        assert_eq!(python_float(1333333333333333.3), "1333333333333333.3");
    }
}
