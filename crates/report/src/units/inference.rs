//! Unit inference over owned SBML data, with explicit work stacks for mathematical trees.

use std::collections::HashMap;
use std::rc::Rc;

use sbml4humans_model::{Ast, AstKind as K, AstNumber, Compartment, Model, Species, Unit};

use super::{UnitError, definition, symbol};

#[derive(Clone, Default)]
struct Info {
    units: Vec<Unit>,
    undeclared: bool,
    value: Option<f64>,
}

fn unit(kind: &str, exponent: f64) -> Unit {
    Unit {
        kind: Some(kind.to_owned()),
        exponent: Some(exponent),
        scale: Some(0),
        multiplier: Some(1.0),
    }
}

fn empty() -> Info {
    Info {
        undeclared: true,
        ..Default::default()
    }
}
fn dimensionless() -> Info {
    Info {
        units: vec![unit("dimensionless", 1.0)],
        ..Default::default()
    }
}
fn exponent(unit: &Unit) -> f64 {
    unit.exponent.unwrap_or(1.0)
}
fn multiplier(unit: &Unit) -> f64 {
    unit.multiplier.unwrap_or(1.0)
}
fn round15(value: f64) -> f64 {
    format!("{value:.14e}").parse().unwrap_or(value)
}

/// Normalize repeated kinds without losing the scale of untouched authored factors.
fn simplify(units: &mut Vec<Unit>) {
    if units.len() < 2 {
        return;
    }
    let has_dimensionless = units
        .iter()
        .any(|u| u.kind.as_deref() == Some("dimensionless"));
    let mut scalar = 1.0;
    if has_dimensionless {
        for u in units.iter_mut() {
            u.multiplier = Some(round15(
                multiplier(u) * 10.0_f64.powf(f64::from(u.scale.unwrap_or(0))),
            ));
            u.scale = Some(0);
            if u.kind.as_deref() == Some("dimensionless") {
                scalar *= multiplier(u).powf(exponent(u));
            }
        }
        units.retain(|u| u.kind.as_deref() != Some("dimensionless"));
    }
    let mut merged: Vec<Unit> = Vec::new();
    for mut next in units.drain(..) {
        if let Some(previous) = merged.iter_mut().find(|u| u.kind == next.kind) {
            let first_power = exponent(previous);
            let second_power = exponent(&next);
            let first = round15(
                multiplier(previous) * 10.0_f64.powf(f64::from(previous.scale.unwrap_or(0))),
            );
            let second =
                round15(multiplier(&next) * 10.0_f64.powf(f64::from(next.scale.unwrap_or(0))));
            let first = if first_power == 0.0 {
                first
            } else {
                first.powf(first_power)
            };
            let second = if second_power == 0.0 {
                second
            } else {
                second.powf(second_power)
            };
            let power = first_power + second_power;
            let amount = if power == 0.0 {
                first * second
            } else {
                (first * second).powf(1.0 / power)
            };
            previous.exponent = Some(power);
            previous.scale = Some(0);
            previous.multiplier = Some(round15(amount));
        } else {
            // Aliases are the same SBML kind, even when spelled differently in XML.
            if next.kind.as_deref() == Some("meter") {
                next.kind = Some("metre".into());
            }
            if next.kind.as_deref() == Some("liter") {
                next.kind = Some("litre".into());
            }
            merged.push(next);
        }
    }
    let mut cancelled = has_dimensionless;
    merged.retain(|u| {
        if exponent(u) == 0.0 {
            scalar *= multiplier(u);
            cancelled = true;
            false
        } else {
            true
        }
    });
    if merged.is_empty() && cancelled {
        let mut value = unit("dimensionless", 1.0);
        value.multiplier = Some(scalar);
        merged.push(value);
    } else if scalar != 1.0
        && let Some(first) = merged.first_mut()
    {
        first.multiplier = Some(multiplier(first) * scalar.powf(1.0 / exponent(first)));
    }
    *units = merged;
}

/// SI dimensions used for compatibility; item keeps its separate SBML dimension.
fn dimensions(units: &[Unit]) -> [f64; 8] {
    let mut result = [0.0; 8];
    for u in units {
        let base: [f64; 8] = match u.kind.as_deref().unwrap_or("") {
            "ampere" => [0., 0., 0., 1., 0., 0., 0., 0.],
            "candela" | "lumen" => [0., 0., 0., 0., 0., 0., 1., 0.],
            "Celsius" | "kelvin" => [0., 0., 0., 0., 1., 0., 0., 0.],
            "gram" | "kilogram" => [1., 0., 0., 0., 0., 0., 0., 0.],
            "metre" | "meter" => [0., 1., 0., 0., 0., 0., 0., 0.],
            "litre" | "liter" => [0., 3., 0., 0., 0., 0., 0., 0.],
            "second" => [0., 0., 1., 0., 0., 0., 0., 0.],
            "mole" => [0., 0., 0., 0., 0., 1., 0., 0.],
            "item" => [0., 0., 0., 0., 0., 0., 0., 1.],
            "becquerel" | "hertz" => [0., 0., -1., 0., 0., 0., 0., 0.],
            "coulomb" => [0., 0., 1., 1., 0., 0., 0., 0.],
            "farad" => [-1., -2., 4., 2., 0., 0., 0., 0.],
            "gray" | "sievert" => [0., 2., -2., 0., 0., 0., 0., 0.],
            "henry" => [1., 2., -2., -2., 0., 0., 0., 0.],
            "joule" => [1., 2., -2., 0., 0., 0., 0., 0.],
            "katal" => [0., 0., -1., 0., 0., 1., 0., 0.],
            "lux" => [0., -2., 0., 0., 0., 0., 1., 0.],
            "newton" => [1., 1., -2., 0., 0., 0., 0., 0.],
            "ohm" => [1., 2., -3., -2., 0., 0., 0., 0.],
            "pascal" => [1., -1., -2., 0., 0., 0., 0., 0.],
            "siemens" => [-1., -2., 3., 2., 0., 0., 0., 0.],
            "tesla" => [1., 0., -2., -1., 0., 0., 0., 0.],
            "volt" => [1., 2., -3., -1., 0., 0., 0., 0.],
            "watt" => [1., 2., -3., 0., 0., 0., 0., 0.],
            "weber" => [1., 2., -2., -1., 0., 0., 0., 0.],
            _ => [0.; 8],
        };
        for i in 0..8 {
            result[i] += base[i] * exponent(u);
        }
    }
    result
}

pub(super) struct Context<'a> {
    model: &'a Model,
    level: u32,
    globals: HashMap<&'a str, Info>,
    functions: HashMap<&'a str, &'a Ast>,
    pub(super) enabled: bool,
}

impl<'a> Context<'a> {
    pub(super) fn new(model: &'a Model, level: u32) -> Self {
        let mut context = Self {
            model,
            level,
            globals: HashMap::new(),
            functions: HashMap::new(),
            enabled: true,
        };
        for f in &model.function_definitions {
            if let (Some(id), Some(math)) = (f.element.id.as_deref(), f.math.as_ref()) {
                context.functions.insert(id, math);
            }
        }
        for c in &model.compartments {
            if let Some(id) = c.element.id.as_deref() {
                let mut info = context.compartment(c);
                info.value = c.size;
                context.globals.entry(id).or_insert(info);
            }
        }
        for s in &model.species {
            if let Some(id) = s.element.id.as_deref() {
                let mut info = context.species(s);
                info.value = s.initial_amount.or(s.initial_concentration);
                context.globals.entry(id).or_insert(info);
            }
        }
        for p in &model.parameters {
            if let Some(id) = p.element.id.as_deref() {
                let mut info = context.lookup(p.units.as_deref());
                info.value = p.value;
                context.globals.entry(id).or_insert(info);
            }
        }
        for reaction in &model.reactions {
            if let Some(id) = reaction.element.id.as_deref() {
                let mut info = context.lookup(if level < 3 {
                    Some("substance")
                } else {
                    model.units.extent.as_deref()
                });
                let mut time = context.lookup(if level < 3 {
                    Some("time")
                } else {
                    model.units.time.as_deref()
                });
                for unit in &mut time.units {
                    unit.exponent = Some(-exponent(unit));
                }
                info.units.extend(time.units);
                info.undeclared |= time.undeclared;
                context.globals.entry(id).or_insert(info);
            }
            for reference in reaction.reactants.iter().chain(&reaction.products) {
                if let Some(id) = reference.element.id.as_deref() {
                    let mut info = dimensionless();
                    info.value = Some(reference.stoichiometry);
                    context.globals.entry(id).or_insert(info);
                }
            }
        }
        context.enabled = context.within_budget();
        context
    }

    fn lookup(&self, id: Option<&str>) -> Info {
        let Some(id) = id else { return empty() };
        let units = if symbol(id).is_ok() || id == "Celsius" {
            vec![unit(id, 1.0)]
        } else if let Some(u) = self
            .model
            .unit_definitions
            .iter()
            .find(|u| u.element.id.as_deref() == Some(id))
        {
            u.units.clone()
        } else if self.level < 3 {
            match id {
                "substance" => vec![unit("mole", 1.)],
                "volume" => vec![unit("litre", 1.)],
                "area" => vec![unit("metre", 2.)],
                "length" => vec![unit("metre", 1.)],
                "time" => vec![unit("second", 1.)],
                _ => vec![],
            }
        } else {
            vec![]
        };
        Info {
            undeclared: units.is_empty(),
            units,
            value: None,
        }
    }

    fn old_default(&self, id: &str) -> Info {
        let mut found = self.lookup(Some(id));
        // Older levels use the first factor of a redefined compartment or substance default.
        found.units.truncate(1);
        found
    }

    fn compartment(&self, c: &Compartment) -> Info {
        if let Some(id) = c.units.as_deref() {
            return self.lookup(Some(id));
        }
        let dimensions =
            c.spatial_dimensions
                .unwrap_or(if self.level < 3 { 3.0 } else { f64::NAN });
        let choice = match dimensions as i32 {
            1 => Some(("length", self.model.units.length.as_deref())),
            2 => Some(("area", self.model.units.area.as_deref())),
            3 => Some(("volume", self.model.units.volume.as_deref())),
            _ => None,
        };
        if self.level < 3 {
            if dimensions == 0.0 {
                dimensionless()
            } else {
                choice
                    .map(|(id, _)| self.old_default(id))
                    .unwrap_or_else(empty)
            }
        } else {
            choice.map(|(_, id)| self.lookup(id)).unwrap_or_else(empty)
        }
    }

    fn species(&self, species: &Species) -> Info {
        let substance = species.substance_units.as_deref().or(self
            .model
            .units
            .substance
            .as_deref());
        let mut info = if let Some(substance) = substance {
            self.lookup(Some(substance))
        } else if self.level < 3 {
            self.old_default("substance")
        } else {
            empty()
        };
        if info.units.is_empty() || species.has_only_substance_units == Some(true) {
            return info;
        }
        let compartment = self
            .model
            .compartments
            .iter()
            .find(|c| c.element.id.as_deref() == Some(species.compartment.as_str()));
        if let Some(c) = compartment {
            if c.spatial_dimensions == Some(0.0) {
                return info;
            }
            let mut volume = species
                .spatial_size_units
                .as_deref()
                .map_or_else(|| self.compartment(c), |units| self.lookup(Some(units)));
            if self.level > 2 && volume.units.is_empty() {
                return empty();
            }
            for unit in &mut volume.units {
                unit.exponent = Some(-exponent(unit));
            }
            info.units.extend(volume.units);
            simplify(&mut info.units);
        }
        info
    }

    fn within_budget(&self) -> bool {
        let mut depths: HashMap<&str, usize> = HashMap::new();
        let mut waiting: HashMap<&str, usize> = HashMap::new();
        let mut callers: HashMap<&str, Vec<&str>> = HashMap::new();
        for (&id, math) in &self.functions {
            let body = if math.kind == K::Lambda {
                math.children.last().unwrap_or(math)
            } else {
                math
            };
            let names = crate::math::free_symbols(body);
            let mut count = 0;
            for name in &names {
                if let Some((&callee, _)) = self.functions.get_key_value(name.as_str()) {
                    count += 1;
                    callers.entry(callee).or_default().push(id);
                }
            }
            waiting.insert(id, count);
        }
        let mut ready: Vec<_> = waiting
            .iter()
            .filter_map(|(&name, &count)| (count == 0).then_some(name))
            .collect();
        while let Some(id) = ready.pop() {
            let math = self.functions[id];
            let body = if math.kind == K::Lambda {
                math.children.last().unwrap_or(math)
            } else {
                math
            };
            depths.insert(id, depth(body, &depths));
            if let Some(dependents) = callers.get(id) {
                for &caller in dependents {
                    if let Some(count) = waiting.get_mut(caller) {
                        *count -= 1;
                        if *count == 0 {
                            ready.push(caller);
                        }
                    }
                }
            }
        }
        depths.len() == self.functions.len()
            && self
                .all_maths()
                .into_iter()
                .all(|ast| depth(ast, &depths) <= 5000)
    }

    fn all_maths(&self) -> Vec<&'a Ast> {
        let mut result = Vec::new();
        result.extend(
            self.model
                .function_definitions
                .iter()
                .filter_map(|f| f.math.as_ref()),
        );
        result.extend(
            self.model
                .initial_assignments
                .iter()
                .filter_map(|x| x.math.as_ref()),
        );
        result.extend(self.model.rules.iter().filter_map(|x| x.math.as_ref()));
        result.extend(
            self.model
                .constraints
                .iter()
                .filter_map(|x| x.math.as_ref()),
        );
        result.extend(
            self.model
                .reactions
                .iter()
                .filter_map(|r| r.kinetic_law.as_ref())
                .filter_map(|l| l.math.as_ref()),
        );
        for e in &self.model.events {
            result.extend(e.trigger.iter().filter_map(|x| x.math.as_ref()));
            result.extend(e.delay.iter().filter_map(|x| x.math.as_ref()));
            result.extend(e.priority.iter().filter_map(|x| x.math.as_ref()));
            result.extend(e.event_assignments.iter().filter_map(|x| x.math.as_ref()));
        }
        result
    }

    pub(super) fn declared(&self, id: Option<&str>) -> Result<Option<String>, UnitError> {
        if !self.enabled {
            return Ok(None);
        }
        definition(&self.lookup(id).units).map(Some)
    }
    pub(super) fn compartment_text(&self, c: &Compartment) -> Result<Option<String>, UnitError> {
        if !self.enabled {
            return Ok(None);
        }
        definition(&self.compartment(c).units).map(Some)
    }
    pub(super) fn species_text(&self, s: &Species) -> Result<Option<String>, UnitError> {
        if !self.enabled {
            return Ok(None);
        }
        definition(&self.species(s).units).map(Some)
    }

    pub(super) fn math(
        &self,
        math: Option<&'a Ast>,
        local: Option<usize>,
    ) -> Result<Option<String>, UnitError> {
        if !self.enabled {
            return Ok(None);
        }
        let Some(math) = math else { return Ok(None) };
        type Environment<'b> = Rc<HashMap<&'b str, [Info; 2]>>;
        enum Work<'b> {
            Visit(&'b Ast, Environment<'b>),
            Reduce(&'b Ast, Environment<'b>),
        }
        let mut bindings = HashMap::new();
        if let Some(law) = local
            .and_then(|i| self.model.reactions.get(i))
            .and_then(|r| r.kinetic_law.as_ref())
        {
            for p in &law.local_parameters {
                if let Some(id) = p.element.id.as_deref() {
                    let mut info = self.lookup(p.units.as_deref());
                    info.value = p.value;
                    bindings.insert(
                        id,
                        [
                            info.clone(),
                            Info {
                                undeclared: true,
                                ..info
                            },
                        ],
                    );
                }
            }
        }
        let env = Rc::new(bindings);
        let mut stack = vec![Work::Visit(math, env)];
        let mut values: Vec<[Info; 2]> = Vec::new();
        while let Some(work) = stack.pop() {
            match work {
                Work::Visit(node, env) => {
                    stack.push(Work::Reduce(node, env.clone()));
                    if node.kind != K::Lambda {
                        stack.extend(
                            node.children
                                .iter()
                                .rev()
                                .map(|child| Work::Visit(child, env.clone())),
                        );
                    }
                }
                Work::Reduce(node, env) => {
                    let count = if node.kind == K::Lambda {
                        0
                    } else {
                        node.children.len()
                    };
                    let start = values.len().saturating_sub(count);
                    let children: Vec<_> = values.drain(start..).collect();
                    if node.kind == K::Function
                        && let Some(lambda) = node
                            .name
                            .as_deref()
                            .and_then(|name| self.functions.get(name))
                    {
                        let body = if lambda.kind == K::Lambda {
                            lambda.children.last().unwrap_or(lambda)
                        } else {
                            lambda
                        };
                        let mut next = (*env).clone();
                        for (variable, value) in lambda
                            .children
                            .iter()
                            .take(lambda.children.len().saturating_sub(1))
                            .zip(&children)
                        {
                            if let Some(name) = variable.name.as_deref() {
                                next.insert(name, value.clone());
                            }
                        }
                        stack.push(Work::Visit(body, Rc::new(next)));
                        continue;
                    }
                    let mut pair = [
                        self.reduce(node, &children, &env, false),
                        self.reduce(node, &children, &env, true),
                    ];
                    for info in &mut pair {
                        // Earlier levels expose integral unit exponents, including the
                        // exponents inferred from a root or a fractional power.
                        if self.level < 3 {
                            for unit in &mut info.units {
                                unit.exponent = Some(exponent(unit).trunc());
                            }
                        }
                        if info.units.len() > 1 {
                            simplify(&mut info.units);
                        }
                    }
                    values.push(pair);
                }
            }
        }
        definition(
            &values
                .last()
                .map(|pair| pair[0].clone())
                .unwrap_or_else(empty)
                .units,
        )
        .map(Some)
    }

    fn reduce(
        &self,
        node: &Ast,
        children: &[[Info; 2]],
        env: &HashMap<&str, [Info; 2]>,
        incoming: bool,
    ) -> Info {
        let index = usize::from(incoming);
        if node.kind == K::Name {
            if let Some(pair) = node.name.as_deref().and_then(|name| env.get(name)) {
                return pair[index].clone();
            }
            let mut info = node
                .name
                .as_deref()
                .and_then(|name| self.globals.get(name))
                .cloned()
                .unwrap_or_else(empty);
            info.undeclared |= incoming;
            return info;
        }
        if matches!(
            node.kind,
            K::Plus
                | K::Minus
                | K::FunctionAbs
                | K::FunctionCeiling
                | K::FunctionFloor
                | K::FunctionMax
                | K::FunctionMin
        ) {
            let mut pos = 0;
            let mut info = children
                .first()
                .map(|p| p[index].clone())
                .unwrap_or_else(empty);
            let mut undeclared = incoming;
            while info.undeclared && pos + 1 < children.len() {
                undeclared = true;
                pos += 1;
                info = children[pos][0].clone();
            }
            for child in children.iter().skip(pos + 1) {
                let other = &child[0];
                if !other.units.is_empty() && dimensions(&other.units) != dimensions(&info.units) {
                    info.units.clear();
                }
                undeclared |= other.undeclared;
            }
            if children.len() > 1 {
                info.undeclared = undeclared;
            }
            let numeric_children: Vec<_> = children.iter().map(|p| p[0].clone()).collect();
            info.value = numeric(node.kind, &numeric_children);
            return info;
        }
        if matches!(
            node.kind,
            K::FunctionPiecewise | K::FunctionDelay | K::FunctionRem
        ) {
            return children
                .first()
                .map(|p| p[index].clone())
                .unwrap_or_else(empty);
        }
        if node.kind == K::FunctionRoot {
            let mut info = children
                .get(1)
                .map(|p| p[index].clone())
                .unwrap_or_else(empty);
            let power = children
                .first()
                .map(|p| p[usize::from(info.undeclared)].clone())
                .unwrap_or_else(empty);
            let numeric = node.children.first().is_some_and(|n| n.number.is_some());
            if numeric || (!power.units.is_empty() && dimensions(&power.units) == [0.; 8]) {
                for u in &mut info.units {
                    if u.kind.as_deref() != Some("dimensionless") {
                        u.exponent = Some(exponent(u) / power.value.unwrap_or(f64::NAN));
                    }
                }
            } else if !power.units.is_empty() {
                info.units.clear();
                info.undeclared = power.undeclared;
            } else {
                info.undeclared = true;
            }
            return info;
        }
        let mut state = incoming;
        let sequence: Vec<Info> = children
            .iter()
            .map(|pair| {
                let info = pair[usize::from(state)].clone();
                state = info.undeclared;
                info
            })
            .collect();
        let first = || sequence.first().cloned().unwrap_or_else(empty);
        match node.kind {
            K::NameTime => {
                let mut info = self.lookup(if self.level < 3 {
                    Some("time")
                } else {
                    self.model.units.time.as_deref()
                });
                info.undeclared |= incoming;
                info
            }
            K::Integer | K::Real | K::RealE | K::Rational => {
                let mut info = self.lookup(node.units.as_deref());
                info.value = number(node);
                info
            }
            K::ConstantPi => {
                let mut info = dimensionless();
                info.undeclared = incoming;
                info.value = Some(std::f64::consts::PI);
                info
            }
            K::ConstantE => {
                let mut info = empty();
                info.value = Some(std::f64::consts::E);
                info
            }
            K::Times | K::Divide | K::FunctionQuotient => {
                let mut info = if sequence.is_empty() && node.kind == K::Times {
                    let mut info = dimensionless();
                    info.undeclared = incoming;
                    info
                } else {
                    first()
                };
                for (i, child) in sequence.iter().enumerate().skip(1) {
                    let mut units = child.units.clone();
                    if i == 1 && node.kind != K::Times {
                        for u in &mut units {
                            u.exponent = Some(-exponent(u));
                        }
                    }
                    info.units.extend(units);
                }
                info.undeclared = state;
                info.value = numeric(node.kind, &sequence);
                info
            }
            K::Power | K::FunctionPower => {
                let mut info = first();
                if let Some(power) = sequence.get(1) {
                    if node.children.get(1).is_some_and(|n| n.number.is_some())
                        || (!power.units.is_empty() && dimensions(&power.units) == [0.; 8])
                    {
                        for u in &mut info.units {
                            u.exponent = Some(exponent(u) * power.value.unwrap_or(f64::NAN));
                        }
                    } else if !power.units.is_empty() {
                        info.units.clear();
                        info.undeclared = state;
                    } else {
                        info.undeclared = true;
                    }
                } else {
                    info.undeclared = true;
                }
                info.value = numeric(node.kind, &sequence);
                info
            }
            K::FunctionRateOf => {
                let mut info = first();
                let time = self.lookup(self.model.units.time.as_deref());
                for mut u in time.units {
                    u.exponent = Some(-exponent(&u));
                    info.units.push(u);
                }
                info
            }
            K::Function
            | K::Unknown
            | K::Other
            | K::NameAvogadro
            | K::DistribNormal
            | K::DistribUniform
            | K::DistribBernoulli
            | K::DistribBinomial
            | K::DistribCauchy
            | K::DistribChisquare
            | K::DistribExponential
            | K::DistribGamma
            | K::DistribLaplace
            | K::DistribLognormal
            | K::DistribPoisson
            | K::DistribRayleigh => empty(),
            _ => {
                let mut info = dimensionless();
                info.undeclared = incoming || sequence.iter().any(|c| c.undeclared);
                info
            }
        }
    }
}

fn depth(root: &Ast, functions: &HashMap<&str, usize>) -> usize {
    let mut stack = vec![(root, 0usize)];
    let mut deepest = 0;
    while let Some((node, above)) = stack.pop() {
        let step = if node.kind == K::Function {
            node.name
                .as_deref()
                .and_then(|n| functions.get(n))
                .copied()
                .unwrap_or(1)
        } else {
            1
        };
        let depth = above.saturating_add(step).min(5001);
        deepest = deepest.max(depth);
        stack.extend(node.children.iter().map(|n| (n, depth)));
    }
    deepest
}

fn number(node: &Ast) -> Option<f64> {
    match node.number.as_ref()? {
        AstNumber::Integer(x) => Some(*x as f64),
        AstNumber::Real(x) => Some(*x),
        AstNumber::RealE { mantissa, exponent } => {
            Some(*mantissa * 10.0_f64.powf(*exponent as f64))
        }
        AstNumber::Rational {
            numerator,
            denominator,
        } => Some(*numerator as f64 / *denominator as f64),
    }
}

fn numeric(kind: K, children: &[Info]) -> Option<f64> {
    let values: Vec<f64> = children.iter().map(|x| x.value).collect::<Option<_>>()?;
    let a = *values.first()?;
    Some(match kind {
        K::Plus => values.iter().sum(),
        K::Times => values.iter().product(),
        K::Minus => {
            if values.len() == 1 {
                -a
            } else {
                a - values.get(1)?
            }
        }
        K::Divide | K::FunctionQuotient => a / values.get(1)?,
        K::Power | K::FunctionPower => a.powf(*values.get(1)?),
        K::FunctionRoot => values.get(1)?.powf(1.0 / a),
        K::FunctionAbs => a.abs(),
        K::FunctionCeiling => a.ceil(),
        K::FunctionFloor => a.floor(),
        K::FunctionMin => values.iter().copied().fold(f64::INFINITY, f64::min),
        K::FunctionMax => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        _ => return None,
    })
}
