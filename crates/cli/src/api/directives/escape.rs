use std::collections::{HashMap, HashSet};
use tree_sitter::{QueryPredicate, QueryPredicateArg};

pub fn collect(predicates: &[QueryPredicate]) -> HashMap<u32, HashSet<String>> {
  let mut map: HashMap<u32, HashSet<String>> = HashMap::new();

  for pred in predicates {
    if pred.operator.as_ref() != "escape!" {
      continue;
    }

    let Ok((capture, escape_chars)) = parse_escape_predicate(pred) else {
      continue;
    };

    map.entry(capture).or_default().extend(escape_chars);
  }

  map
}

pub fn escape_chars(modifiers: &HashMap<u32, HashSet<String>>, capture: u32) -> HashSet<String> {
  modifiers.get(&capture).cloned().unwrap_or_default()
}

fn parse_escape_predicate(pred: &QueryPredicate) -> anyhow::Result<(u32, HashSet<String>)> {
  if pred.args.len() < 2 {
    anyhow::bail!("Escape predicate requires at least 2 arguments");
  }

  let QueryPredicateArg::Capture(capture) = pred.args[0] else {
    anyhow::bail!("Escape predicate requires capture as first argument");
  };

  let mut escape_chars = HashSet::new();
  for arg in pred.args.iter().skip(1) {
    let QueryPredicateArg::String(value) = arg else {
      anyhow::bail!("Escape predicate only supports string arguments");
    };
    escape_chars.insert(value.to_string());
  }

  Ok((capture, escape_chars))
}
