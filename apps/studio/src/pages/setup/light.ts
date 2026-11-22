import "./light.css";
import { type Found, identify } from "../../core/api.ts";
import { familyIcon } from "../../components/family/family.ts";
import { explain, t } from "../../core/i18n.ts";
import { loading } from "../../components/loading/loading.ts";
import { icon } from "../../shell/nav.ts";
import { tooltip } from "../../components/tooltip/tooltip.ts";

const LOCATE = "M10.5 17.5a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM20 20l-4.5-4.5";
const EDIT = "M4 20h4L18.5 9.5a2.8 2.8 0 0 0-4-4L4 16v4M13.5 6.5l4 4";
const CHECK = "M5 12l5 5L20 7";
const ONLINE = "M12 6.5a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11z";

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ""): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

function iconButton(d: string, label: string): HTMLButtonElement {
  const button = element("button", "icon-button");
  button.type = "button";
  button.setAttribute("aria-label", label);
  button.append(icon(d));
  tooltip(button, label);
  return button;
}

/** A dot that shows the light answered, and the magnifier of the action on hover and on focus. */
function rest(): SVGSVGElement[] {
  const answered = icon(ONLINE);
  answered.classList.add("rest");
  const find = icon(LOCATE);
  find.classList.add("act");
  return [answered, find];
}

/** The identify button: the light shows green for some seconds, and the button shows the wait. */
function identifyButton(found: Found, error: HTMLElement): HTMLButtonElement {
  const button = iconButton(LOCATE, t("light.identify"));
  button.classList.add("light-find");
  button.replaceChildren(...rest());
  button.addEventListener("click", () => {
    button.disabled = true;
    error.hidden = true;
    button.replaceChildren(loading({ size: "s" }));
    identify(found.id).then(
      () => {
        button.replaceChildren(icon(CHECK));
        setTimeout(() => {
          button.disabled = false;
          button.replaceChildren(...rest());
        }, 1500);
      },
      (failure: unknown) => {
        button.disabled = false;
        button.replaceChildren(...rest());
        error.hidden = false;
        error.textContent = explain(failure);
      },
    );
  });
  return button;
}

/**
 * The mark of a light that has no answer: the scan runs, the scan failed, or
 * the light did not answer. It takes the focus, so that a keyboard shows its
 * tooltip.
 */
function statusMark(text: string, content: Node): HTMLElement {
  const mark = element("span", "light-find light-offline");
  mark.tabIndex = 0;
  mark.setAttribute("role", "img");
  mark.setAttribute("aria-label", text);
  mark.append(content);
  tooltip(mark, text);
  return mark;
}

/** The status of the light. `checking` says that a scan runs. */
function status(found: Found, checking: boolean, error: HTMLElement): HTMLElement {
  if (found.online === true) return identifyButton(found, error);
  if (found.online === false) return statusMark(t("light.offline"), icon(ONLINE));
  if (checking) return statusMark(t("light.checking"), loading({ size: "s" }));
  return statusMark(t("light.unknown"), icon(ONLINE));
}

/** The state of a card that Continue saves. */
export interface Draft {
  selected: boolean;
  name: string | null;
}

/** The label of a light: its name, its SKU, or its ID. */
const label = (found: Found, name: string | null): string => name ?? found.sku ?? found.id;

/** The rename field. Enter and a click out keep the name, Esc cancels. `done` gets the name. */
function nameField(found: Found, name: string | null, done: (kept: string | null) => void): HTMLInputElement {
  const input = element("input", "light-input");
  input.value = name ?? "";
  input.placeholder = found.sku ?? found.id;
  input.maxLength = 64;
  input.setAttribute("aria-label", t("light.name"));
  let over = false;
  const finish = (kept: string | null): void => {
    if (over) return;
    over = true;
    done(kept);
  };
  input.addEventListener("keydown", (event) => {
    if (event.key === "Enter") finish(input.value.trim() || null);
    if (event.key === "Escape") finish(name);
  });
  input.addEventListener("blur", () => {
    finish(input.value.trim() || null);
  });
  return input;
}

/**
 * The name, or the SKU while the light has none, and the button that renames
 * it. A rename changes `draft`.
 */
function nameRow(found: Found, draft: Draft): HTMLElement {
  const row = element("div", "light-name");
  const show = (name: string | null): void => {
    draft.name = name;
    const edit = iconButton(EDIT, t("light.rename"));
    edit.addEventListener("click", () => {
      const input = nameField(found, name, show);
      row.replaceChildren(input);
      input.focus();
      input.select();
    });
    row.replaceChildren(element("span", "light-title", label(found, name)), edit);
  };
  show(draft.name);
  return row;
}

/** One light. A click on the card and a rename change `draft`. `checking` says that a scan runs. */
export function lightCard(found: Found, draft: Draft, checking = false): HTMLElement {
  const card = element("article", "light-card");
  card.classList.toggle("offline", found.online === false);
  const error = element("p", "light-error");
  error.hidden = true;
  const check = element("input", "light-check");
  check.type = "checkbox";
  check.checked = draft.selected;
  check.setAttribute("aria-label", t("light.select", { name: label(found, draft.name) }));
  const mark = (): void => {
    card.classList.toggle("selected", check.checked);
    draft.selected = check.checked;
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
  card.classList.toggle("selected", draft.selected);
  const top = element("div", "light-top");
  const shape = element("span", "light-icon");
  shape.append(familyIcon(found.family), check);
  const sku = element("div", "light-sku", found.sku ?? "");
  top.append(shape, sku, status(found, checking, error));
  card.append(top, nameRow(found, draft), error);
  return card;
}
