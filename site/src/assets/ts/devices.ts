export function filter(input: HTMLInputElement): void {
  const rows = [...document.querySelectorAll<HTMLTableRowElement>("[data-rows] tr")];
  const count = document.querySelector("[data-filter-count]");
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
        : `${total} models in the catalog.`;
    }
  };

  input.addEventListener("input", apply);
  apply();
}


// The link in the first cell stays the target, so the keyboard and a middle
// click reach the page through it.
export function rowLink(body: HTMLElement): void {
  body.addEventListener("click", (event) => {
    if (!(event.target instanceof Element) || event.target.closest("a")) return;
    if (String(getSelection())) return;
    const row = event.target.closest("tr");
    const link = row?.querySelector<HTMLAnchorElement>("a[href]");
    if (link) link.click();
  });
}
