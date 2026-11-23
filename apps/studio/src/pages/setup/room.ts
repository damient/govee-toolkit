import { type Found, type Room, labelOf } from "../../core/api.ts";
import { familyIcon } from "../../components/family/family.ts";
import { menu } from "../../components/menu/menu.ts";
import { t } from "../../core/i18n.ts";
import { tooltip } from "../../components/tooltip/tooltip.ts";
import { EDIT, icon } from "../../components/icon/icon.ts";
import { element } from "../../core/dom.ts";

const TRASH = "M4 7h16M10 11v6M14 11v6M5 7l1 12a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2l1-12M9 7V4h6v3";
const CLOSE = "M6 6l12 12M18 6L6 18";

export interface RoomActions {
  /** Answer an error to show, or `null` when the name is kept. */
  rename: (name: string) => string | null;
  remove: () => void;
  /** Take the light `id` out of the room. */
  take: (id: string) => void;
  /** Make a chip of a light: the step gives it the drag and the pick. */
  chip: (light: Found, room: number) => HTMLElement;
}

export function iconButton(d: string, label: string): HTMLButtonElement {
  const button = element("button", "icon-button");
  button.type = "button";
  button.setAttribute("aria-label", label);
  button.append(icon(d));
  tooltip(button, label);
  return button;
}

export function chipFace(light: Found): HTMLElement[] {
  const mark = element("span", "chip-icon");
  mark.append(familyIcon(light.family));
  return [mark, element("span", "chip-label", labelOf(light))];
}

/** A name field. Enter and a click out keep the name, Esc cancels. `done` gets `null` on a cancel. */
export function nameInput(value: string, done: (name: string | null) => void): HTMLInputElement {
  const input = element("input", "room-input");
  input.value = value;
  input.maxLength = 64;
  input.placeholder = t("setup.rooms.placeholder");
  input.setAttribute("aria-label", t("setup.rooms.name"));
  let over = false;
  const finish = (name: string | null): void => {
    if (over) return;
    over = true;
    done(name);
  };
  input.addEventListener("keydown", (event) => {
    if (event.key === "Enter") finish(input.value.trim());
    if (event.key === "Escape") finish(null);
  });
  input.addEventListener("blur", () => {
    finish(input.value.trim());
  });
  return input;
}

function head(room: Room, actions: RoomActions): HTMLElement {
  const row = element("div", "room-head");
  const error = element("p", "room-error");
  error.hidden = true;
  const edit = (): void => {
    const input = nameInput(room.name, (name) => {
      const problem = name === null || name === "" ? null : actions.rename(name);
      error.hidden = problem === null;
      error.textContent = problem ?? "";
      show();
    });
    row.replaceChildren(input);
    input.focus();
    input.select();
  };
  const show = (): void => {
    const count = room.devices.length;
    const badge = element("span", "room-count", String(count));
    badge.setAttribute("aria-label", t(count === 1 ? "setup.rooms.count.one" : "setup.rooms.count.other", { count: String(count) }));
    const actionsMenu = menu({
      label: t("setup.rooms.menu"),
      items: [
        { text: t("setup.rooms.rename"), icon: EDIT, run: edit },
        { text: t("setup.rooms.delete"), icon: TRASH, run: actions.remove, tone: "danger" },
      ],
    });
    row.replaceChildren(element("h3", "room-name", room.name), badge, actionsMenu);
  };
  show();
  const wrap = element("div", "room-top");
  wrap.append(row, error);
  return wrap;
}

/** `index` is the place of the room in the draft. */
export function roomCard(room: Room, index: number, lights: Map<string, Found>, actions: RoomActions): HTMLElement {
  const card = element("section", "room-card");
  card.dataset.drop = String(index);
  const list = element("div", "room-lights");
  for (const id of room.devices) {
    const light = lights.get(id);
    if (light === undefined) continue;
    const chip = actions.chip(light, index);
    const take = iconButton(CLOSE, t("setup.rooms.remove", { name: labelOf(light), room: room.name }));
    take.classList.add("chip-take");
    take.dataset.nodrag = "";
    take.addEventListener("click", (event) => {
      event.stopPropagation();
      actions.take(id);
    });
    chip.append(take);
    list.append(chip);
  }
  if (list.childElementCount === 0) list.append(element("p", "room-drop", t("setup.rooms.drop")));
  card.append(head(room, actions), list);
  return card;
}
