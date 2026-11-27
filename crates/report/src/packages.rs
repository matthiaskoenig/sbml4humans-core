//! Package report projections. The reader owns XML; this module owns wire names and keys.

use crate::{CompSBase, Double, KeyValuePair, ListOf, ReportModel, SBase, Uncertainty, build, pk};
pub(crate) use sbml4humans_model::packages::{attr, children};
use sbml4humans_model::{Attribute, Element, Extension, Model};
use serde_json::{Value, json};

fn qualified<'a>(attributes: &'a [Attribute], package: &str, key: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|a| {
            a.name == key
                && a.namespace
                    .as_deref()
                    .is_some_and(|ns| ns.contains(&format!("/{package}/version")))
        })
        .map(|a| a.value.as_str())
}
fn direct<'a>(element: &'a Element, package: &str, name: &str) -> Option<&'a Extension> {
    element
        .extensions
        .iter()
        .find(|e| e.package == package && e.name == name)
}
fn members<'a>(object: &'a Extension, list: &str) -> Vec<&'a Extension> {
    object
        .children
        .iter()
        .filter(|e| e.name == list)
        .flat_map(|e| &e.children)
        .collect()
}
pub(crate) fn kind(object: &Extension) -> &'static str {
    match object.name.as_str() {
        "submodel" => "Submodel",
        "port" => "Port",
        "deletion" => "Deletion",
        "replacedBy" => "ReplacedBy",
        "replacedElement" => "ReplacedElement",
        "sBaseRef" => "SBaseRef",
        "externalModelDefinition" => "ExternalModelDefinition",
        "geneProduct" => "GeneProduct",
        "geneProductRef" => "GeneProductRef",
        "and" => "And",
        "or" => "Or",
        "geneProductAssociation" => "GeneProductAssociation",
        "objective" => "Objective",
        "fluxObjective" => "FluxObjective",
        "fluxBound" => "FluxBound",
        "userDefinedConstraint" => "UserDefinedConstraint",
        "userDefinedConstraintComponent" => "UserDefinedConstraintComponent",
        "qualitativeSpecies" => "QualitativeSpecies",
        "transition" => "Transition",
        "input" => "Input",
        "output" => "Output",
        "functionTerm" => "FunctionTerm",
        "defaultTerm" => "DefaultTerm",
        "uncertainty" => "Uncertainty",
        "uncertParameter" => "UncertParameter",
        "uncertSpan" => "UncertSpan",
        _ => "ListOf",
    }
}
fn indexed(objects: &[&Extension]) -> Vec<String> {
    pk::indexed(objects, |e| &e.element, |e| kind(e))
}
fn named(objects: &[&Extension], parent: &str, name: &str) -> Vec<String> {
    pk::keyed_by(
        objects,
        |e| &e.element,
        &objects
            .iter()
            .enumerate()
            .map(|(i, _)| format!("{parent}.{name}.{i}"))
            .collect::<Vec<_>>(),
    )
}
fn own(object: &Extension, fallback: &str) -> String {
    pk::key(&object.element, || fallback.to_owned())
}
fn reference_path(object: &Extension) -> Vec<&str> {
    let mut path = Vec::new();
    let mut current = Some(object);
    while let Some(reference) = current {
        if let Some(value) = ["portRef", "idRef", "unitRef", "metaIdRef"]
            .into_iter()
            .find_map(|k| attr(reference, k))
        {
            path.push(value);
        }
        current = reference.children.iter().find(|c| c.name == "sBaseRef");
    }
    path
}
fn reference_key(object: &Extension, parent: &str, replacement: bool) -> String {
    let mut path = reference_path(object);
    if replacement
        && path.is_empty()
        && let Some(deletion) = attr(object, "deletion")
    {
        path.push(deletion);
    }
    let prefix = if replacement {
        format!(
            "{parent}.replacedElement.{}",
            attr(object, "submodelRef").unwrap_or("")
        )
    } else {
        format!("{parent}.deletion")
    };
    if path.is_empty() {
        prefix
    } else {
        format!("{prefix}.{}", path.join("."))
    }
}
fn measure_key(object: &Extension, parent: &str) -> String {
    let name = attr(object, "type").unwrap_or(&object.name);
    if name == "externalParameter"
        && let Some(url) = attr(object, "definitionURL")
    {
        return format!("{parent}.{name}.{url}");
    }
    format!("{parent}.{name}")
}
fn measures(
    context: &build::BuildContext,
    object: &Extension,
    scope: &str,
    key: &str,
) -> Vec<Value> {
    let objects = if object.name == "uncertainty" {
        object
            .children
            .iter()
            .filter(|c| matches!(c.name.as_str(), "uncertParameter" | "uncertSpan"))
            .collect::<Vec<_>>()
    } else {
        members(object, "listOfUncertParameters")
    };
    let candidates = objects
        .iter()
        .enumerate()
        .map(|(i, p)| {
            if object.name == "uncertainty" {
                measure_key(p, key)
            } else {
                format!("{key}.{}.{i}", p.name)
            }
        })
        .collect::<Vec<_>>();
    let keys = pk::keyed_by(&objects, |e| &e.element, &candidates);
    objects
        .into_iter()
        .zip(keys)
        .map(|(p, k)| object_value(context, p, scope, &k))
        .collect()
}
fn uncertainty(
    context: &build::BuildContext,
    object: &Extension,
    scope: &str,
    fallback: &str,
) -> Uncertainty {
    let key = own(object, fallback);
    Uncertainty {
        sbase: build::sbase(context, scope, "Uncertainty", &object.element, &key),
        uncert_parameters: measures(context, object, scope, &key),
    }
}

pub(crate) fn extend(
    context: &build::BuildContext,
    base: &mut SBase,
    element: &Element,
    scope: &str,
    key: &str,
) {
    let replaced_by = direct(element, "comp", "replacedBy")
        .map(|r| object_value(context, r, scope, &format!("{key}.replacedBy")));
    let replacements = children(element, "comp", "listOfReplacedElements");
    let replacement_keys = pk::derived(
        &replacements,
        |e| &e.element,
        |e| reference_key(e, key, true),
    );
    let replaced_elements: Vec<_> = replacements
        .into_iter()
        .zip(replacement_keys)
        .map(|(r, k)| object_value(context, r, scope, &k))
        .collect();
    if replaced_by.is_some() || !replaced_elements.is_empty() {
        base.comp = Some(CompSBase {
            replaced_by,
            replaced_elements,
        });
    }
    let uncertainties = children(element, "distrib", "listOfUncertainties");
    let uncertainty_keys = named(&uncertainties, key, "uncertainty");
    base.uncertainties = uncertainties
        .into_iter()
        .zip(uncertainty_keys)
        .map(|(u, k)| uncertainty(context, u, scope, &k))
        .collect();
    base.key_value_pairs = children(element, "fbc", "listOfKeyValuePairs")
        .into_iter()
        .map(|p| KeyValuePair {
            key: attr(p, "key").map(str::to_owned),
            value: attr(p, "value").map(str::to_owned),
            uri: attr(p, "uri").map(str::to_owned),
        })
        .collect();
    let prefix = if matches!(base.sbml_type, "Model" | "SBMLDocument") {
        String::new()
    } else {
        format!("{key}.")
    };
    let list_keys = pk::derived(
        &element.lists,
        |l| &l.element,
        |l| format!("{prefix}{}", l.name),
    );
    base.lists = element
        .lists
        .iter()
        .zip(list_keys)
        .map(|(l, k)| ListOf {
            sbase: build::sbase(context, scope, "ListOf", &l.element, &k),
            element: l.name.clone(),
            size: l.size as i64,
        })
        .collect();
}

/// Project one known package object. Package nesting is bounded by the reader.
pub(crate) fn object_value(
    context: &build::BuildContext,
    object: &Extension,
    scope: &str,
    fallback: &str,
) -> Value {
    let key = own(object, fallback);
    let sbml_type = kind(object);
    let mut base = build::sbase(context, scope, sbml_type, &object.element, &key);
    let association = matches!(
        sbml_type,
        "GeneProductAssociation" | "GeneProductRef" | "And" | "Or"
    );
    if association {
        base.xml = None;
    }
    let mut value = json!(base);
    let fields = value
        .as_object_mut()
        .expect("SBase serializes to an object");
    macro_rules! fields {
        ($($name:literal => $source:literal : $type:ident),* $(,)?) => { $(fields.insert($name.into(), scalar(object, $source, stringify!($type)));)* };
    }
    macro_rules! list {
        ($field:literal, $source:literal, $name:expr) => {{
            let children: Vec<_> = members(object, $source)
                .into_iter()
                .filter(|e| $field != "listOfFunctionTerms" || e.name != "defaultTerm")
                .collect();
            let keys = named(&children, &key, $name);
            fields.insert(
                $field.into(),
                Value::Array(
                    children
                        .into_iter()
                        .zip(keys)
                        .map(|(e, k)| object_value(context, e, scope, &k))
                        .collect(),
                ),
            );
        }};
    }
    if matches!(
        sbml_type,
        "SBaseRef" | "Port" | "Deletion" | "ReplacedElement" | "ReplacedBy"
    ) {
        fields!("portRef"=>"portRef":string,"idRef"=>"idRef":string,"unitRef"=>"unitRef":string,"metaIdRef"=>"metaIdRef":string);
        fields.insert(
            "sbaseRef".into(),
            object
                .children
                .iter()
                .find(|c| c.name == "sBaseRef")
                .map(|c| object_value(context, c, scope, &format!("{key}.sBaseRef")))
                .unwrap_or(Value::Null),
        );
    }
    match sbml_type {
        "ReplacedBy" => {
            fields!("submodelRef"=>"submodelRef":required);
        }
        "ReplacedElement" => {
            fields!("submodelRef"=>"submodelRef":required,"deletion"=>"deletion":string,"conversionFactor"=>"conversionFactor":string);
        }
        "Submodel" => {
            fields!("modelRef"=>"modelRef":required,"timeConversionFactor"=>"timeConversionFactor":string,"extentConversionFactor"=>"extentConversionFactor":string);
            let children = members(object, "listOfDeletions");
            let keys = pk::derived(&children, |e| &e.element, |e| reference_key(e, &key, false));
            fields.insert(
                "listOfDeletions".into(),
                Value::Array(
                    children
                        .into_iter()
                        .zip(keys)
                        .map(|(e, k)| object_value(context, e, scope, &k))
                        .collect(),
                ),
            );
        }
        "ExternalModelDefinition" => {
            fields!("source"=>"source":required,"modelRef"=>"modelRef":string,"md5"=>"md5":string);
            let status = if attr(object, "source")
                .is_some_and(|s| s.contains("://") || s.starts_with("//"))
            {
                "remoteSource"
            } else {
                "notFound"
            };
            fields.insert(
                "resolution".into(),
                json!({"status":status,"entry":null,"model":null,"md5Matches":null}),
            );
        }
        "GeneProduct" => {
            fields!("label"=>"label":string,"associatedSpecies"=>"associatedSpecies":string);
        }
        "GeneProductRef" => {
            fields!("geneProduct"=>"geneProduct":required);
        }
        "GeneProductAssociation" => {
            fields.insert(
                "association".into(),
                object
                    .children
                    .iter()
                    .find(|e| matches!(e.name.as_str(), "and" | "or" | "geneProductRef"))
                    .map(|e| object_value(context, e, scope, &format!("{key}.association")))
                    .unwrap_or(Value::Null),
            );
        }
        "And" | "Or" => {
            let children: Vec<_> = object.children.iter().collect();
            let keys = pk::keyed_by(
                &children,
                |e| &e.element,
                &children
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("{key}.{i}"))
                    .collect::<Vec<_>>(),
            );
            fields.insert(
                "associations".into(),
                Value::Array(
                    children
                        .into_iter()
                        .zip(keys)
                        .map(|(e, k)| object_value(context, e, scope, &k))
                        .collect(),
                ),
            );
        }
        "Objective" => {
            fields!("type"=>"type":string);
            let children = members(object, "listOfFluxObjectives");
            let keys = pk::derived(
                &children,
                |e| &e.element,
                |e| {
                    let reaction = attr(e, "reaction").unwrap_or("");
                    let second = attr(e, "reaction2").or_else(|| {
                        (attr(e, "variableType") == Some("quadratic")).then_some(reaction)
                    });
                    second.map_or_else(
                        || format!("{key}.fluxObjective.{reaction}"),
                        |s| format!("{key}.fluxObjective.{reaction}.{s}"),
                    )
                },
            );
            fields.insert(
                "listOfFluxObjectives".into(),
                Value::Array(
                    children
                        .into_iter()
                        .zip(keys)
                        .map(|(e, k)| object_value(context, e, scope, &k))
                        .collect(),
                ),
            );
        }
        "FluxObjective" => {
            fields!("reaction"=>"reaction":required,"reaction2"=>"reaction2":string,"coefficient"=>"coefficient":number,"variableType"=>"variableType":string);
        }
        "FluxBound" => {
            fields!("reaction"=>"reaction":string,"operation"=>"operation":string,"value"=>"value":number);
        }
        "UserDefinedConstraint" => {
            fields!("lowerBound"=>"lowerBound":string,"upperBound"=>"upperBound":string);
            list!(
                "listOfUserDefinedConstraintComponents",
                "listOfUserDefinedConstraintComponents",
                "component"
            );
        }
        "UserDefinedConstraintComponent" => {
            fields!("variable"=>"variable":string,"variable2"=>"variable2":string,"coefficient"=>"coefficient":string,"variableType"=>"variableType":string);
        }
        "QualitativeSpecies" => {
            fields!("compartment"=>"compartment":required,"constant"=>"constant":boolean,"initialLevel"=>"initialLevel":integer,"maxLevel"=>"maxLevel":integer);
        }
        "Transition" => {
            list!("listOfInputs", "listOfInputs", "input");
            list!("listOfOutputs", "listOfOutputs", "output");
            list!("listOfFunctionTerms", "listOfFunctionTerms", "functionTerm");
            let default = members(object, "listOfFunctionTerms")
                .into_iter()
                .find(|e| e.name == "defaultTerm");
            fields.insert(
                "defaultTerm".into(),
                default
                    .map(|e| object_value(context, e, scope, &format!("{key}.defaultTerm")))
                    .unwrap_or(Value::Null),
            );
            // The default is stored in the same XML list, outside its member count.
            if let Some(Value::Array(terms)) = fields.get_mut("listOfFunctionTerms") {
                terms.retain(|t| t["sbmlType"] != "DefaultTerm");
            }
        }
        "Input" => {
            fields!("qualitativeSpecies"=>"qualitativeSpecies":required,"thresholdLevel"=>"thresholdLevel":integer,"transitionEffect"=>"transitionEffect":string,"sign"=>"sign":string);
        }
        "Output" => {
            fields!("qualitativeSpecies"=>"qualitativeSpecies":required,"outputLevel"=>"outputLevel":integer,"transitionEffect"=>"transitionEffect":string);
        }
        "FunctionTerm" | "DefaultTerm" => {
            fields!("resultLevel"=>"resultLevel":integer);
        }
        "UncertParameter" | "UncertSpan" => {
            fields!("type"=>"type":string,"var"=>"var":string,"value"=>"value":number,"units"=>"units":string,"definitionUrl"=>"definitionURL":string);
            fields.insert(
                "uncertParameters".into(),
                Value::Array(measures(context, object, scope, &key)),
            );
            if sbml_type == "UncertSpan" {
                fields!("valueLower"=>"valueLower":number,"valueUpper"=>"valueUpper":number,"varLower"=>"varLower":string,"varUpper"=>"varUpper":string);
            }
        }
        _ => {}
    }
    if matches!(sbml_type, "FunctionTerm" | "UncertParameter" | "UncertSpan") {
        fields.insert(
            "math".into(),
            object
                .math
                .as_ref()
                .filter(|_| !context.capture)
                .and_then(|a| crate::math::math(a).ok())
                .map(|m| json!(m))
                .unwrap_or(Value::Null),
        );
    }
    if association {
        fields.retain(|_, v| !v.is_null() && v != &json!([]));
    }
    value
}
fn scalar(object: &Extension, name: &str, type_: &str) -> Value {
    let value = attr(object, name);
    match type_ {
        "required" => json!(value.unwrap_or("")),
        "boolean" => value
            .and_then(|s| match s {
                "true" | "1" => Some(true),
                "false" | "0" => Some(false),
                _ => None,
            })
            .map_or(Value::Null, Value::Bool),
        "integer" => value
            .and_then(|s| s.parse::<i64>().ok())
            .map_or(Value::Null, |n| json!(n)),
        "number" => value
            .and_then(|s| match s {
                "INF" => Some(f64::INFINITY),
                "-INF" => Some(f64::NEG_INFINITY),
                _ => s.parse::<f64>().ok(),
            })
            .map_or(Value::Null, |n| json!(Double(n))),
        _ => json!(value),
    }
}

pub(crate) fn model(
    context: &build::BuildContext,
    input: &Model,
    output: &mut ReportModel,
    has_fbc: bool,
) {
    let scope = output.sbase.pk.split('/').next().unwrap_or("model");
    macro_rules! lists { ($($field:ident : $package:literal => $list:literal),* $(,)?) => { $(
        let objects = children(&input.element,$package,$list);
        let keys = indexed(&objects);
        output.$field = objects.into_iter().zip(keys).map(|(e,k)| object_value(context, e,scope,&k)).collect();
    )* }; }
    lists!(list_of_submodels:"comp"=>"listOfSubmodels",list_of_ports:"comp"=>"listOfPorts",list_of_gene_products:"fbc"=>"listOfGeneProducts",list_of_objectives:"fbc"=>"listOfObjectives",list_of_flux_bounds:"fbc"=>"listOfFluxBounds",list_of_user_defined_constraints:"fbc"=>"listOfUserDefinedConstraints",list_of_qualitative_species:"qual"=>"listOfQualitativeSpecies",list_of_transitions:"qual"=>"listOfTransitions");
    if has_fbc {
        let active = direct(&input.element, "fbc", "listOfObjectives")
            .and_then(|e| attr(e, "activeObjective"))
            .filter(|s| !s.is_empty());
        let strict =
            qualified(&input.element.package_attributes, "fbc", "strict").and_then(|s| match s {
                "true" | "1" => Some(true),
                "false" | "0" => Some(false),
                _ => None,
            });
        output.fbc = Some(json!({"strict":strict,"activeObjective":active}));
    }
    for (input, output) in input.species.iter().zip(&mut output.list_of_species) {
        let attrs = &input.element.package_attributes;
        let formula = qualified(attrs, "fbc", "chemicalFormula");
        let charge = qualified(attrs, "fbc", "charge")
            .and_then(|s| s.parse::<f64>().ok())
            .map(Double);
        if formula.is_some() || charge.is_some() {
            output.fbc = Some(json!({"chemicalFormula":formula,"charge":charge}));
        }
    }
    for (input, output) in input.reactions.iter().zip(&mut output.list_of_reactions) {
        let lower = qualified(&input.element.package_attributes, "fbc", "lowerFluxBound");
        let upper = qualified(&input.element.package_attributes, "fbc", "upperFluxBound");
        let key = output
            .sbase
            .pk
            .split_once(':')
            .map(|(_, k)| k)
            .unwrap_or("");
        let association = direct(&input.element, "fbc", "geneProductAssociation")
            .map(|e| object_value(context, e, scope, &format!("{key}.geneProductAssociation")));
        if lower.is_some() || upper.is_some() || association.is_some() {
            output.fbc = Some(
                json!({"lowerFluxBound":lower,"upperFluxBound":upper,"geneProductAssociation":association}),
            );
        }
    }
}

pub(crate) fn external(context: &build::BuildContext, element: &Element) -> Vec<Value> {
    let objects = children(element, "comp", "listOfExternalModelDefinitions");
    let keys = indexed(&objects);
    objects
        .into_iter()
        .zip(keys)
        .map(|(e, k)| object_value(context, e, "document", &k))
        .collect()
}
