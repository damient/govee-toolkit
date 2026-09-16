// The examples of `content/reference.json`, filled in. The reference page and
// a model page read the same entry: each page supplies the values it knows,
// and this fills the placeholders with them.

// A `{name}` the map holds. An unknown name is left alone, so a Rust format
// string inside an example survives.
const PLACEHOLDER = /\{(\w+)\}/g;

const fill = (source, values) =>
  source.replace(PLACEHOLDER, (all, name) => (name in values ? String(values[name]) : all));

/**
 * The examples of one entry, in every language it carries. `values` fills the
 * placeholders, and falls back to the `values` of the entry. `can` is the set
 * of names the page satisfies, or `null` where it satisfies every name: an
 * `adds` block joins the example only where its `needs` are in that set.
 */
export function examples(entry, values, can) {
  const all = { ...entry.values, ...values };
  const out = {};
  for (const [lang, source] of Object.entries(entry.examples)) out[lang] = fill(source, all);
  for (const add of entry.adds ?? []) {
    if (can && !(add.needs ?? []).every((name) => can.has(name))) continue;
    for (const [lang, source] of Object.entries(add.examples)) {
      if (lang in out) out[lang] += `\n${fill(source, all)}`;
    }
  }
  return out;
}
