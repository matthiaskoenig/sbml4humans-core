//! The document, its model and the elements of the model.

use crate::element::Element;

/// A compartment of a model.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Compartment {
    /// The fields every SBML object shares.
    pub element: Element,
    /// The `spatialDimensions` attribute.
    pub spatial_dimensions: Option<f64>,
    /// The `size` attribute.
    pub size: Option<f64>,
    /// The `constant` attribute.
    pub constant: Option<bool>,
    /// The `units` attribute.
    pub units: Option<String>,
}

/// A species of a model.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Species {
    /// The fields every SBML object shares.
    pub element: Element,
    /// The id of the compartment of the species.
    pub compartment: String,
    /// The `initialAmount` attribute.
    pub initial_amount: Option<f64>,
    /// The `initialConcentration` attribute.
    pub initial_concentration: Option<f64>,
    /// The `substanceUnits` attribute.
    pub substance_units: Option<String>,
    /// The `hasOnlySubstanceUnits` attribute.
    pub has_only_substance_units: Option<bool>,
    /// The `boundaryCondition` attribute.
    pub boundary_condition: Option<bool>,
    /// The `constant` attribute.
    pub constant: Option<bool>,
}

/// A parameter of a model.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Parameter {
    /// The fields every SBML object shares.
    pub element: Element,
    /// The `value` attribute.
    pub value: Option<f64>,
    /// The `constant` attribute.
    pub constant: Option<bool>,
    /// The `units` attribute.
    pub units: Option<String>,
}

/// The model of a document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Model {
    /// The fields every SBML object shares.
    pub element: Element,
    /// The compartments, in the order of the document.
    pub compartments: Vec<Compartment>,
    /// The species, in the order of the document.
    pub species: Vec<Species>,
    /// The parameters, in the order of the document.
    pub parameters: Vec<Parameter>,
}

/// An SBML package a document uses.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Package {
    /// The prefix of the package namespace, such as `comp`.
    pub prefix: String,
    /// The version of the package.
    pub version: u32,
    /// Whether the package is required by the document.
    pub required: bool,
}

/// An SBML document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Document {
    /// The SBML level.
    pub level: u32,
    /// The SBML version within the level.
    pub version: u32,
    /// The packages the document uses, in the order of the document.
    pub packages: Vec<Package>,
    /// The fields every SBML object shares.
    pub element: Element,
    /// The model, when the document has one.
    pub model: Option<Model>,
}
