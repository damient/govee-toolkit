import { toast } from "./toast.ts";

// The category select ships `hidden`: without this script it navigates nowhere.
// Each option is the URL of a category page, and the search carries over in `q`.
export function filter(input: HTMLInputElement): void {
  const rows = [...document.querySelectorAll<HTMLTableRowElement>("[data-rows] tr")];
  const count = document.querySelector<HTMLElement>("[data-filter-count]");
  const category = document.querySelector<HTMLSelectElement>("[data-filter-category]");
  const total = rows.length;

  const apply = () => {
    const needle = input.value.trim().toLowerCase();
    let shown = 0;
    for (const row of rows) {
      const match = !needle || (row.dataset.search ?? "").includes(needle);
      if (row.hidden === match) row.hidden = !match;
      if (match) shown += 1;
    }
    if (count) {
      count.textContent = needle
        ? `${shown} of ${total} models match.`
        : `${total} models ${count.dataset.filterScope ?? "in the catalog"}.`;
    }
  };

  const query = new URLSearchParams(location.search).get("q");
  if (query !== null) input.value = query;
  input.addEventListener("input", apply);
  if (category) {
    category.hidden = false;
    category.addEventListener("change", () => {
      const needle = input.value.trim();
      location.assign(needle ? `${category.value}?q=${encodeURIComponent(needle)}` : category.value);
    });
    // A page restored from the history keeps the option the reader left it with.
    addEventListener("pageshow", () => {
      const own = [...category.options].find((o) => o.defaultSelected);
      if (own) category.value = own.value;
    });
  }
  apply();
}

// A plain click on the SKU copies it; the link serves the keyboard and every other click.
export function rowLink(body: HTMLElement): void {
  body.addEventListener("click", (event) => {
    if (!(event.target instanceof Element)) return;
    const sku = event.target.closest("th[scope='row']");
    if (sku && event.detail > 0 && !(event.metaKey || event.ctrlKey || event.shiftKey || event.altKey)) {
      event.preventDefault();
      void copySku(sku.textContent?.trim() ?? "");
      return;
    }
    if (event.target.closest("a")) return;
    if (String(getSelection())) return;
    const row = event.target.closest("tr");
    const link = row?.querySelector<HTMLAnchorElement>("th a[href]");
    if (link) link.click();
  });
}

async function copySku(sku: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(sku);
    toast(`Reference ${sku} copied`);
  } catch {
    toast("Copy failed");
  }
}
