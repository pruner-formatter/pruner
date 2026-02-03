use regex::Regex;
use std::collections::HashMap;
use tree_sitter::{QueryPredicate, QueryPredicateArg};

#[derive(Debug, Clone)]
pub struct GsubRule {
  pub regex: Regex,
  pub replacement: String,
}

pub fn collect(predicates: &[QueryPredicate]) -> HashMap<u32, Vec<GsubRule>> {
  let mut map: HashMap<u32, Vec<GsubRule>> = HashMap::new();

  for pred in predicates {
    if pred.operator.as_ref() != "gsub!" {
      continue;
    }

    let Ok((capture, lua_pattern, lua_replacement)) = parse_gsub_predicate(pred) else {
      continue;
    };

    let Ok(rule) = compile_gsub_rule(&lua_pattern, &lua_replacement) else {
      continue;
    };

    map.entry(capture).or_default().push(rule);
  }

  map
}

pub fn apply_gsub(modifiers: &HashMap<u32, Vec<GsubRule>>, capture: u32, text: &str) -> String {
  let Some(rules) = modifiers.get(&capture) else {
    return text.to_owned();
  };

  apply(text, rules)
}

pub fn apply(text: &str, rules: &[GsubRule]) -> String {
  let mut out = text.to_owned();
  for rule in rules {
    out = rule
      .regex
      .replace_all(&out, rule.replacement.as_str())
      .into_owned();
  }
  out
}

fn parse_gsub_predicate(pred: &QueryPredicate) -> anyhow::Result<(u32, String, String)> {
  if pred.args.len() != 3 {
    anyhow::bail!("Gsub predicate requires 3 arguments");
  }

  let [
    QueryPredicateArg::Capture(capture),
    QueryPredicateArg::String(pattern),
    QueryPredicateArg::String(replacement),
  ] = pred.args.as_ref()
  else {
    anyhow::bail!("Gsub predicate contained unexpected arguments");
  };

  Ok((*capture, pattern.to_string(), replacement.to_string()))
}

fn compile_gsub_rule(lua_pattern_src: &str, lua_replacement: &str) -> anyhow::Result<GsubRule> {
  let ast = lua_pattern::parse(lua_pattern_src)?;
  let re_src = lua_pattern::try_to_regex(&ast, false, false)?;
  let regex = Regex::new(&re_src)?;

  Ok(GsubRule {
    regex,
    replacement: lua_replacement_to_regex(lua_replacement),
  })
}

fn lua_replacement_to_regex(repl: &str) -> String {
  // Lua `string.gsub` uses `%1`..`%9` (and `%0`) for capture references and `%%` for a literal `%`.
  // Rust `regex` uses `$1`..`$9` (and `$0`) for capture references and `$$` for a literal `$`.
  let mut out = String::with_capacity(repl.len());
  let mut chars = repl.chars();

  while let Some(c) = chars.next() {
    match c {
      '$' => out.push_str("$$"),
      '%' => {
        let Some(next) = chars.next() else {
          out.push('%');
          continue;
        };

        match next {
          '%' => out.push('%'),
          d if d.is_ascii_digit() => {
            out.push('$');
            out.push(d);
          }
          other => {
            // Treat `%x` as escaping `x`.
            if other == '$' {
              out.push_str("$$")
            } else {
              out.push(other)
            }
          }
        }
      }
      other => out.push(other),
    }
  }

  out
}
