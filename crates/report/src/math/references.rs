//! Free mathematical symbols with lexical lambda scope.

use sbml4humans_model::{Ast, AstKind};
use std::collections::{BTreeSet, HashMap};

/// Collect free names with explicit lambda entry/exit frames, independent of call-stack depth.
pub fn free_symbols(root: &Ast) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    enum Frame<'a> {
        Enter(&'a Ast),
        Leave(Vec<&'a str>),
    }
    let mut bound: HashMap<&str, usize> = HashMap::new();
    let mut stack = vec![Frame::Enter(root)];
    while let Some(frame) = stack.pop() {
        match frame {
            Frame::Leave(variables) => {
                for name in variables {
                    if let Some(count) = bound.get_mut(name) {
                        *count -= 1;
                        if *count == 0 {
                            bound.remove(name);
                        }
                    }
                }
            }
            Frame::Enter(node) => {
                if node.kind == AstKind::Lambda
                    && let Some((body, variables)) = node.children.split_last()
                {
                    let variables: Vec<_> =
                        variables.iter().filter_map(|v| v.name.as_deref()).collect();
                    for &name in &variables {
                        *bound.entry(name).or_default() += 1;
                    }
                    stack.push(Frame::Leave(variables));
                    stack.push(Frame::Enter(body));
                    continue;
                }
                if matches!(
                    node.kind,
                    AstKind::Name | AstKind::NameTime | AstKind::NameAvogadro | AstKind::Function
                ) && let Some(name) = node.name.as_deref()
                    && !bound.contains_key(name)
                {
                    names.insert(name.to_owned());
                }
                stack.extend(node.children.iter().rev().map(Frame::Enter));
            }
        }
    }
    names
}
