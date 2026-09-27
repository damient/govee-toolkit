//! The Node and Python surfaces. napi writes `binding.d.cts`, and stubtest
//! checks `_govee_toolkit.pyi` against the module.

use std::collections::BTreeMap;

/// `Class.name(arg: type, …)`. A getter has no argument list.
pub(super) fn node(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut class: Option<&str> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("export declare class ") {
            class = rest.split([' ', '{']).next();
            continue;
        }
        if line.starts_with('}') {
            class = None;
            continue;
        }
        if let Some(rest) = line.strip_prefix("export declare function ") {
            out.push(signature("", rest));
            continue;
        }
        let Some(class) = class else { continue };
        let Some(member) = line.strip_prefix("  ") else {
            continue;
        };
        if member.starts_with(['/', '*', ' ']) {
            continue;
        }
        let member = member.strip_prefix("static ").unwrap_or(member);
        if let Some(getter) = member.strip_prefix("get ") {
            let name = getter.split('(').next().unwrap_or_default();
            out.push(format!("{class}.{name}"));
        } else if let Some(rest) = member.strip_prefix("constructor") {
            out.push(signature(&format!("new {class}"), rest));
        } else {
            out.push(signature(&format!("{class}."), member));
        }
    }
    out.sort();
    out
}

/// The `@param [name=value]` lines of each doc comment, by the `Class.name`
/// that follows them.
pub(super) fn node_defaults(text: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut out: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut class: Option<&str> = None;
    let mut pending = BTreeMap::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("export declare class ") {
            class = rest.split([' ', '{']).next();
            continue;
        }
        let trimmed = line.trim();
        if let Some(rest) = trimmed
            .strip_prefix("* @param [")
            .and_then(|rest| rest.rsplit_once(']'))
            .and_then(|(inside, _)| inside.split_once('='))
        {
            pending.insert(rest.0.to_owned(), rest.1.to_owned());
            continue;
        }
        if trimmed.starts_with('*') || trimmed.starts_with("/*") || pending.is_empty() {
            continue;
        }
        let member = trimmed.strip_prefix("static ").unwrap_or(trimmed);
        let name = member.split('(').next().unwrap_or_default();
        let key = match class {
            Some(class) => format!("{class}.{name}"),
            None => name
                .strip_prefix("export declare function ")
                .unwrap_or(name)
                .to_owned(),
        };
        out.insert(key, std::mem::take(&mut pending));
    }
    out
}

/// `Class.name(arg: type, …)`, and a constructor as `Class(…)`. A property
/// has no argument list.
pub(super) fn python(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut class: Option<&str> = None;
    let mut property = false;
    let mut i = 0;
    while let Some(line) = lines.get(i) {
        i += 1;
        if let Some(rest) = line.strip_prefix("class ") {
            class = rest.split(['(', ':']).next();
            continue;
        }
        let (prefix, member) = match line.strip_prefix("    ") {
            Some(member) if class.is_some() => (format!("{}.", class.unwrap_or_default()), member),
            _ if !line.starts_with(' ') && !line.is_empty() => {
                class = None;
                (String::new(), *line)
            }
            _ => continue,
        };
        if member.trim() == "@property" {
            property = true;
            continue;
        }
        let member = member.strip_prefix("async ").unwrap_or(member);
        let Some(def) = member.strip_prefix("def ") else {
            continue;
        };
        let mut whole = def.to_owned();
        while depth(&whole) > 0 {
            let Some(next) = lines.get(i) else { break };
            whole.push(' ');
            whole.push_str(next.trim());
            i += 1;
        }
        let name = whole.split('(').next().unwrap_or_default();
        if name == "__init__" {
            out.push(signature(
                prefix.trim_end_matches('.'),
                &whole[name.len()..],
            ));
        } else if !name.starts_with('_') {
            if property {
                out.push(format!("{prefix}{name}"));
            } else {
                out.push(signature(&prefix, &whole));
            }
        }
        property = false;
    }
    out.sort();
    out
}

/// `name(args) -> ret` to `<prefix>name(args)`.
fn signature(prefix: &str, text: &str) -> String {
    let open = text.find('(').unwrap_or(text.len());
    let name = &text[..open];
    let mut level = 0;
    let mut close = text.len();
    for (index, c) in text.char_indices().skip(open) {
        level += bracket(text, index, c);
        if level == 0 {
            close = index;
            break;
        }
    }
    let inner = text.get(open + 1..close).unwrap_or_default();
    let args: Vec<&str> = split_top(inner)
        .into_iter()
        .map(str::trim)
        .filter(|arg| !arg.is_empty() && *arg != "self")
        .collect();
    format!("{prefix}{name}({})", args.join(", "))
}

pub(super) fn split_top(text: &str) -> Vec<&str> {
    split_any(text, &[','])
}

/// `text` cut at each separator that stands outside brackets.
pub(super) fn split_any<'a>(text: &'a str, separators: &[char]) -> Vec<&'a str> {
    let mut parts = Vec::new();
    let mut level = 0i32;
    let mut start = 0;
    for (index, c) in text.char_indices() {
        level += bracket(text, index, c);
        if separators.contains(&c) && level == 0 {
            parts.push(&text[start..index]);
            start = index + 1;
        }
    }
    parts.push(&text[start..]);
    parts
}

/// The `>` of `=>` closes nothing.
pub(super) fn bracket(text: &str, index: usize, c: char) -> i32 {
    match c {
        '(' | '[' | '{' | '<' => 1,
        ')' | ']' | '}' | '>' if !text[..index].ends_with('=') => -1,
        _ => 0,
    }
}

fn depth(text: &str) -> i32 {
    text.chars().fold(0, |level, c| match c {
        '(' => level + 1,
        ')' => level - 1,
        _ => level,
    })
}

pub(super) fn camel(snake: &str) -> String {
    let mut out = String::new();
    let mut upper = false;
    for c in snake.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}
