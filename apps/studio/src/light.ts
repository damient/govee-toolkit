import { type Found, identify, rename } from "./api.ts";
import { familyIcon } from "./family.ts";
import { explain, t } from "./i18n.ts";
import { loading } from "./loading.ts";
import { icon } from "./nav.ts";

const LOCATE = "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM9.5 9.5a2.5 2.5 0 1 1 3.5 2.3c-.6.3-1 .9-1 1.6v.6M12 17h.01";
const EDIT = "M4 20h4L18.5 9.5a2.8 2.8 0 0 0-4-4L4 16v4M13.5 6.5l4 4";
const CHECK = "M5 12l5 5L20 7";

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ""): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

function iconButton(d: string, label: string): HTMLButtonElement {
  const button = element("button", "icon-button");
  button.type = "button";
  button.title = label;
  button.setAttribute("aria-label", label);
  button.append(icon(d));
  return button;
}

/** The identify button: the light shows green for some seconds, and the button shows the wait. */
function identifyButton(found: Found, error: HTMLElement): HTMLButtonElement {
  const button = iconButton(LOCATE, t("light.identify"));
  button.addEventListener("click", () => {
    button.disabled = true;
    error.hidden = true;
    button.replaceChildren(loading({ size: "s" }));
    identify(found.id).then(
      () => {
        button.replaceChildren(icon(CHECK));
        setTimeout(() => {
          button.disabled = false;
          button.replaceChildren(icon(LOCATE));
        }, 1500);
      },
      (failure: unknown) => {
        button.disabled = false;
        button.replaceChildren(icon(LOCATE));
        error.hidden = false;
        error.textContent = explain(failure);
      },
    );
  });
  return button;
}

/**
 * The rename field. Enter and a click out save, Esc cancels. `done` gets the
 * name that the server keeps, or the name as it was.
 */
function nameField(found: Found, name: string | null, error: HTMLElement, done: (kept: string | null) => void): HTMLInputElement {
  const input = element("input", "light-input");
  input.value = name ?? "";
  input.placeholder = found.sku;
  input.maxLength = 64;
  input.setAttribute("aria-label", t("light.name"));
  let over = false;
  const save = (): void => {
    if (over) return;
    over = true;
    const wanted = input.value.trim();
    if (wanted === (name ?? "")) {
      done(name);
      return;
    }
    input.disabled = true;
    rename(found.id, wanted).then(done, (failure: unknown) => {
      error.hidden = false;
      error.textContent = explain(failure);
      done(name);
    });
  };
  input.addEventListener("keydown", (event) => {
    if (event.key === "Enter") save();
    if (event.key !== "Escape") return;
    over = true;
    done(name);
  });
  input.addEventListener("blur", save);
  return input;
}

/**
 * The name, or the SKU while the light has none, and the button that renames
 * it. `sku` shows the SKU under a name.
 */
function nameRow(found: Found, sku: HTMLElement, error: HTMLElement): HTMLElement {
  const row = element("div", "light-name");
  const show = (name: string | null): void => {
    sku.textContent = name === null ? "" : found.sku;
    const edit = iconButton(EDIT, t("light.rename"));
    edit.addEventListener("click", () => {
      const input = nameField(found, name, error, show);
      row.replaceChildren(input);
      input.focus();
      input.select();
    });
    row.replaceChildren(element("span", "light-title", name ?? found.sku), edit);
  };
  show(found.name);
  return row;
}

/** One light that answered the scan. `select` reports a change of the check box. */
export function lightCard(found: Found, selected: boolean, select: (on: boolean) => void): HTMLElement {
  const card = element("article", "light-card");
  const error = element("p", "light-error");
  error.hidden = true;
  const check = element("input", "light-check");
  check.type = "checkbox";
  check.checked = selected;
  check.setAttribute("aria-label", t("light.select", { name: found.name ?? found.sku }));
  const mark = (): void => {
    card.classList.toggle("selected", check.checked);
    select(check.checked);
  };
  check.addEventListener("change", mark);
  // A click on the card, out of its controls, toggles the selection. The path
  // of the event is read, because a control can swap its content on the click
  // and detach the target before the card hears it.
  card.addEventListener("click", (event) => {
    if (event.composedPath().some((node) => node instanceof Element && node.matches("button, input"))) return;
    check.checked = !check.checked;
    mark();
  });
  card.classList.toggle("selected", selected);
  const top = element("div", "light-top");
  const shape = element("span", "light-icon");
  shape.append(familyIcon(found.family));
  top.append(shape, identifyButton(found, error), check);
  const sku = element("div", "light-sku");
  card.append(top, nameRow(found, sku, error), sku, error);
  return card;
}
