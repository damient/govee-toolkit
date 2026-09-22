// The category select ships `hidden`: without this script it filters nothing,
// so it stays out of the page.
export function filter(input: HTMLInputElement): void {
  const rows = [...document.querySelectorAll<HTMLTableRowElement>("[data-rows] tr")];
  const count = document.querySelector<HTMLElement>("[data-filter-count]");
  const category = document.querySelector<HTMLSelectElement>("[data-filter-category]");
  const total = rows.length;

  const apply = () => {
    const needle = input.value.trim().toLowerCase();
    const group = category?.value ?? "";
    let shown = 0;
    for (const row of rows) {
      const match = (!needle || (row.dataset.search ?? "").includes(needle))
        && (!group || row.dataset.category === group);
      if (row.hidden === match) row.hidden = !match;
      if (match) shown += 1;
    }
    if (count) {
      count.textContent = needle || group
        ? `${shown} of ${total} models match.`
        : `${total} models ${count.dataset.filterScope ?? "in the catalog"}.`;
    }
  };

  input.addEventListener("input", apply);
  if (category) {
    category.hidden = false;
    category.addEventListener("change", apply);
  }
  apply();
}


// The link in the row header stays the target, so the keyboard and a middle
// click reach the page through it. A row without one leads nowhere.
export function rowLink(body: HTMLElement): void {
  body.addEventListener("click", (event) => {
    if (!(event.target instanceof Element) || event.target.closest("a")) return;
    if (String(getSelection())) return;
    const row = event.target.closest("tr");
    const link = row?.querySelector<HTMLAnchorElement>("th a[href]");
    if (link) link.click();
  });
}
