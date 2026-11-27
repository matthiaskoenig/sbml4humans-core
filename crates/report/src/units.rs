//! Reader-independent rendering of SBML unit factors and definitions.
//!
//! Display follows the Python report's Pint compaction policy. Semantic unit
//! inference uses the same factors, rather than parsing their rendered text.

mod inference;

use sbml4humans_model::Unit;

use crate::number::python_float;

/// A unit kind which the Python report's registry cannot render.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("'{0}' is not defined in the unit registry")]
pub struct UnitError(pub String);

/// The short symbol used for a base unit within a definition.
fn symbol(kind: &str) -> Result<&'static str, UnitError> {
    Ok(match kind {
        "ampere" => "A",
        "avogadro" => "avogadro",
        "becquerel" => "Bq",
        "candela" => "cd",
        "coulomb" => "C",
        "dimensionless" => "",
        "farad" => "F",
        "gram" => "g",
        "gray" => "Gy",
        "henry" => "H",
        "hertz" => "Hz",
        "item" => "item",
        "joule" => "J",
        "katal" => "kat",
        "kelvin" => "K",
        "kilogram" => "kg",
        "liter" | "litre" => "l",
        "lumen" => "lm",
        "lux" => "lx",
        "meter" | "metre" => "m",
        "mole" => "mol",
        "newton" => "N",
        "ohm" => "Ω",
        "pascal" => "Pa",
        "radian" => "rad",
        "second" => "s",
        "siemens" => "S",
        "sievert" => "Sv",
        "steradian" => "sr",
        "tesla" => "T",
        "volt" => "V",
        "watt" => "W",
        "weber" => "Wb",
        _ => return Err(UnitError(kind.to_owned())),
    })
}

/// Six significant digits, the general format used for unit exponents.
fn general(value: f64) -> String {
    if !value.is_finite() || value == 0.0 {
        return python_float(value).trim_end_matches(".0").to_owned();
    }
    let scientific = format!("{value:.5e}");
    let (digits, power) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let power: i32 = power.parse().unwrap_or(0);
    if !(-4..6).contains(&power) {
        format!(
            "{}e{power:+03}",
            digits.trim_end_matches('0').trim_end_matches('.')
        )
    } else {
        let precision = (5 - power).max(0) as usize;
        let fixed = format!("{value:.precision$}");
        if fixed.contains('.') {
            fixed.trim_end_matches('0').trim_end_matches('.').to_owned()
        } else {
            fixed
        }
    }
}

/// Render one SBML factor. `None` means a factor equal to one without a unit.
///
/// Invalid or omitted numeric attributes use the same defaults as the Python
/// report. An unrepresentable scale retains the authored power of ten.
pub fn factor(
    kind: &str,
    exponent: f64,
    scale: i32,
    multiplier: f64,
) -> Result<Option<String>, UnitError> {
    let short = symbol(kind)?;
    let exponent = if exponent.is_finite() { exponent } else { 1.0 };
    let multiplier = if multiplier.is_finite() {
        multiplier
    } else {
        1.0
    };
    let magnitude = multiplier * 10.0_f64.powf(f64::from(scale));
    let power = exponent.abs();
    if !magnitude.is_finite() {
        let mut text = format!("10^{{{scale}}}*{short}");
        if power != 1.0 {
            text = format!("({text})^{}", general(power));
        }
        return Ok(Some(text));
    }
    let mut magnitude = magnitude.powf(power);
    // A negative fractional power is not a real quantity and the Python
    // implementation refuses it rather than inventing a display unit.
    if magnitude.is_nan() {
        return Err(UnitError(format!("non-real factor {kind}")));
    }
    let mut prefix = "";
    let mut base = short;
    let unitless = matches!(kind, "dimensionless" | "item" | "avogadro") || power == 0.0;
    if power == 0.0 {
        base = "";
    }
    if !unitless && magnitude != 0.0 && magnitude.is_finite() {
        const PREFIXES: [&str; 21] = [
            "q", "r", "y", "z", "a", "f", "p", "n", "μ", "m", "", "k", "M", "G", "T", "P", "E",
            "Z", "Y", "R", "Q",
        ];
        let original_scale = if kind == "kilogram" { 1000.0_f64 } else { 1.0 };
        let base_magnitude = magnitude * original_scale.powf(power);
        let engineering = (base_magnitude.abs().log10() / power / 3.0).floor() * 3.0;
        let selected = engineering.clamp(-30.0, 30.0) as i32;
        prefix = PREFIXES[((selected + 30) / 3) as usize];
        if kind == "kilogram" {
            base = "g";
        }
        let conversion =
            original_scale.powf(power) * 10.0_f64.powf(f64::from(selected)).powf(-power);
        // Pint returns the original quantity when the selected unit is unchanged.
        if !(kind == "kilogram" && selected == 3) {
            magnitude *= conversion;
        }
    }
    let unity = (magnitude - 1.0).abs() <= 1.0e-8 + 1.0e-5;
    let amount = if unity {
        "1".to_owned()
    } else {
        python_float(magnitude)
    };
    let mut unit = format!("{prefix}{base}");
    if !unit.is_empty() && power != 1.0 {
        unit.push_str(&format!("^{}", general(power)));
    }
    let mut text = if unit.is_empty() {
        amount
    } else {
        format!("{amount} {unit}")
    };
    if text == "1" {
        return Ok(None);
    }
    for (from, to) in [
        ("60.0 s", "1 min"),
        ("3600.0 s", "1 hr"),
        ("3.6 ks", "1 hr"),
        ("86.4 ks", "1 day"),
        ("10.0 mm", "1 cm"),
    ] {
        text = text.replace(from, to);
    }
    Ok(Some(text.replace("1 ", "")))
}

/// Render a definition without reordering or merging its authored factors.
pub fn definition(units: &[Unit]) -> Result<String, UnitError> {
    let mut numerator = Vec::new();
    let mut denominator = Vec::new();
    let mut dimensionless = false;
    for unit in units {
        let exponent = unit.exponent.filter(|x| x.is_finite()).unwrap_or(1.0);
        if let Some(text) = factor(
            unit.kind.as_deref().unwrap_or("invalid"),
            exponent,
            unit.scale.unwrap_or(0),
            unit.multiplier.unwrap_or(1.0),
        )? {
            if exponent >= 0.0 {
                numerator.push(text);
            } else {
                denominator.push(text);
            }
        } else {
            dimensionless = true;
        }
    }
    let numerator = numerator.join("*").replace('*', " \\cdot ");
    let denominator = denominator.join("*").replace('*', " \\cdot ");
    Ok(match (numerator.is_empty(), denominator.is_empty()) {
        (false, false) => format!("\\frac{{{numerator}}}{{{denominator}}}"),
        (false, true) => numerator,
        (true, false) => format!("\\frac{{1}}{{{denominator}}}"),
        (true, true) => if dimensionless { "dimensionless" } else { "-" }.to_owned(),
    })
}

/// Render an authored units attribute, resolving named definitions in its model.
pub fn attribute(
    id: Option<&str>,
    model: &sbml4humans_model::Model,
) -> Result<Option<String>, UnitError> {
    let Some(id) = id else { return Ok(None) };
    if symbol(id).is_ok() || id == "Celsius" {
        return Ok(Some(
            match id {
                "metre" | "meter" => "m",
                "liter" | "litre" => "l",
                "dimensionless" => "-",
                "second" => "s",
                _ => id,
            }
            .to_owned(),
        ));
    }
    let found = model
        .unit_definitions
        .iter()
        .find(|u| u.element.id.as_deref() == Some(id));
    Ok(Some(match found {
        Some(unit) => definition(&unit.units)?,
        None => "-".to_owned(),
    }))
}

fn conversion(
    id: Option<&str>,
    model: &sbml4humans_model::Model,
) -> Option<crate::ConversionFactor> {
    id.map(|id| {
        let parameter = model
            .parameters
            .iter()
            .find(|p| p.element.id.as_deref() == Some(id));
        crate::ConversionFactor {
            sid: id.to_owned(),
            value: parameter.and_then(|p| p.value).map(crate::Double),
            units: parameter.and_then(|p| p.units.clone()),
        }
    })
}

/// Fill unit labels and resolved conversion-factor values from the reader-neutral model.
pub(crate) fn apply(
    document: &sbml4humans_model::Document,
    report: &mut crate::Report,
) -> Result<(), UnitError> {
    for (input, output) in document.models().zip(&mut report.models) {
        let inference = inference::Context::new(input, document.level);
        macro_rules! default_unit {
            ($raw:ident, $label:ident, $field:ident) => {
                output.$raw = input.units.$field.clone();
                output.$label = attribute(input.units.$field.as_deref(), input)?;
            };
        }
        default_unit!(substance_units, substance_units_latex, substance);
        default_unit!(time_units, time_units_latex, time);
        default_unit!(volume_units, volume_units_latex, volume);
        default_unit!(area_units, area_units_latex, area);
        default_unit!(length_units, length_units_latex, length);
        default_unit!(extent_units, extent_units_latex, extent);
        output.conversion_factor = conversion(input.conversion_factor.as_deref(), input);
        for (unit, result) in input
            .unit_definitions
            .iter()
            .zip(&mut output.list_of_unit_definitions)
        {
            result.units_latex = Some(definition(&unit.units)?);
        }
        for (source, compartment) in input
            .compartments
            .iter()
            .zip(&mut output.list_of_compartments)
        {
            compartment.derived_units = inference.compartment_text(source)?;
            compartment.units_latex = attribute(compartment.units.as_deref(), input)?;
        }
        for (species, result) in input.species.iter().zip(&mut output.list_of_species) {
            result.derived_units = inference.species_text(species)?;
            result.units_latex = attribute(species.substance_units.as_deref(), input)?;
            result.conversion_factor = conversion(species.conversion_factor.as_deref(), input);
        }
        for parameter in &mut output.list_of_parameters {
            parameter.derived_units = inference.declared(parameter.units.as_deref())?;
            parameter.units_latex = attribute(parameter.units.as_deref(), input)?;
        }
        for (source, result) in input
            .initial_assignments
            .iter()
            .zip(&mut output.list_of_initial_assignments)
        {
            result.derived_units = inference.math(source.math.as_ref(), None)?;
        }
        for (source, result) in input.rules.iter().zip(&mut output.list_of_rules) {
            result.derived_units = inference.math(source.math.as_ref(), None)?;
        }
        for (i, (source, result)) in input
            .reactions
            .iter()
            .zip(&mut output.list_of_reactions)
            .enumerate()
        {
            if let (Some(source), Some(law)) =
                (source.kinetic_law.as_ref(), result.kinetic_law.as_mut())
            {
                law.derived_units = inference.math(source.math.as_ref(), Some(i))?;
                for parameter in &mut law.list_of_local_parameters {
                    parameter.derived_units = inference.declared(parameter.units.as_deref())?;
                    parameter.units_latex = attribute(parameter.units.as_deref(), input)?;
                }
            }
        }
    }
    Ok(())
}
