import "../../components/light/light.css";
import { type Found, identify, labelOf } from "../../core/api.ts";
import { familyIcon } from "../../components/family/family.ts";
import { explain, t } from "../../core/i18n.ts";
import { loading } from "../../components/loading/loading.ts";
import { CHECK, EDIT, SEARCH, icon } from "../../components/icon/icon.ts";
import { element } from "../../core/dom.ts";
import { iconButton } from "./room.ts";
import { statusMark } from "../../components/light/mark.ts";

const ONLINE = "M12 6.5a5.5 5.5 0 1 0 0 11 5.5 5.5 0 0 0 0-11z";

function rest(): SVGSVGElement[] {
  const answered = icon(ONLINE);
  answered.classList.add("rest");
  const find = icon(SEARCH);
  find.classList.add("act");
  return [answered, find];
}

function identifyButton(found: Found, error: HTMLElement): HTMLButtonElement {
  const button = iconButton(SEARCH, t("light.identify"));
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

function status(found: Found, checking: boolean, error: HTMLElement): HTMLElement {
  if (found.online === true) return identifyButton(found, error);
  if (found.online === false) return statusMark(t("light.offline"), icon(ONLINE));
  if (checking) return statusMark(t("light.checking"), loading({ size: "s" }));
  return statusMark(t("light.unknown"), icon(ONLINE));
}

export interface Draft {
  selected: boolean;
  name: string | null;
}

/** Enter and a click out keep the name, Esc cancels. */
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
    row.replaceChildren(element("span", "light-title", labelOf({ ...found, name })), edit);
  };
  show(draft.name);
  return row;
}

export function lightCard(found: Found, draft: Draft, checking = false): HTMLElement {
  const card = element("article", "light-card");
  card.classList.toggle("offline", found.online === false);
  const error = element("p", "light-error");
  error.hidden = true;
  const check = element("input", "light-check");
  check.type = "checkbox";
  check.checked = draft.selected;
  check.setAttribute("aria-label", t("light.select", { name: labelOf({ ...found, name: draft.name }) }));
  const mark = (): void => {
    card.classList.toggle("selected", check.checked);
    draft.selected = check.checked;
  };
  check.addEventListener("change", mark);
  // Read the event path: a control can swap its content on the click and detach the target.
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
