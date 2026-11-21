// The build writes the strings of the page language into `#site-strings`.

let table: Record<string, string> | undefined;

function load(): Record<string, string> {
  if (table) return table;
  try {
    const parsed: unknown = JSON.parse(document.querySelector("#site-strings")?.textContent ?? "{}");
    table = {};
    if (typeof parsed === "object" && parsed !== null) {
      for (const [key, value] of Object.entries(parsed)) if (typeof value === "string") table[key] = value;
    }
  } catch {
    table = {};
  }
  return table;
}

/** The text of `key` (`client.<key>` in `content/i18n/`), with each `{name}` filled. */
export function say(key: string, vars: Record<string, string | number> = {}): string {
  const text = load()[key] ?? key;
  return text.replaceAll(/\{(\w+)\}/gu, (all, name: string) => (name in vars ? String(vars[name]) : all));
}
