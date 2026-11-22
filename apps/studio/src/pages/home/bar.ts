import type { Room } from "../../core/api.ts";
import { icon } from "../../shell/nav.ts";
import { loading } from "../../components/loading/loading.ts";
import { pathOf } from "../../core/router.ts";
import { POWER } from "./card.ts";
import { t } from "../../core/i18n.ts";
import { tooltip } from "../../components/tooltip/tooltip.ts";

const SEARCH = "M10.5 17.5a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM20 20l-4.5-4.5";
const PENCIL = "M4 20h4L18.5 9.5a2.8 2.8 0 0 0-4-4L4 16v4M13.5 6.5l4 4";

/** The value of the "All" tab. A room ID is never empty. */
export const ALL = "";

export interface Bar {
  node: HTMLElement;
  /** Show the action of the power button. `null` disables it. */
  show(action: Action): void;
  /** Show the wait of the power button. */
  busy(on: boolean): void;
}

/** `off` turns off the lights shown, `on` turns back on the lights that `off` turned off. */
export type Action = "off" | "on" | null;

export interface BarOptions {
  rooms: Room[];
  room: string;
  /** A new tab or a new search. */
  pick(room: string, query: string): void;
  /** A click on the power button. */
  power(): void;
}

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ""): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

/** The tabs of the rooms, as the segmented control of Settings. */
function tabs(rooms: Room[], current: string, pick: (room: string) => void): HTMLElement {
  const group = element("div", "seg home-rooms");
  group.setAttribute("role", "radiogroup");
  group.setAttribute("aria-label", t("home.rooms"));
  const entries = [{ id: ALL, name: t("home.all") }, ...rooms];
  for (const room of entries) {
    const label = element("label", "");
    const input = element("input", "");
    input.type = "radio";
    input.name = "home-room";
    input.value = room.id;
    input.checked = room.id === current;
    input.addEventListener("change", () => {
      pick(room.id);
    });
    label.append(input, room.name);
    group.append(label);
  }
  return group;
}

function search(find: (query: string) => void): HTMLElement {
  const box = element("label", "home-search");
  const input = element("input", "");
  input.type = "search";
  input.placeholder = t("home.search");
  input.setAttribute("aria-label", t("home.search"));
  input.addEventListener("input", () => {
    find(input.value.trim().toLowerCase());
  });
  box.append(icon(SEARCH), input);
  return box;
}

/** The power button. `set` shows its action. */
function powerButton(click: () => void): { button: HTMLButtonElement; set: (action: Action) => void } {
  const button = element("button", "home-power");
  button.type = "button";
  button.append(icon(POWER));
  button.addEventListener("click", click);
  const tip = tooltip(button, t("home.power.off"));
  const set = (action: Action): void => {
    const label = t(action === "on" ? "home.power.on" : "home.power.off");
    button.disabled = action === null;
    // Cyan while it turns lights off: some of them are on.
    button.setAttribute("aria-pressed", String(action === "off"));
    button.setAttribute("aria-label", label);
    tip(label);
  };
  return { button, set };
}

function setupLink(): HTMLAnchorElement {
  const link = element("a", "icon-button home-setup");
  link.href = pathOf("setup");
  link.setAttribute("aria-label", t("home.setup"));
  link.append(icon(PENCIL));
  tooltip(link, t("home.setup"));
  return link;
}

/** The tabs of the rooms on the left. The search, the power button and the link to the setup on the right. */
export function bar(options: BarOptions): Bar {
  let room = options.room;
  let query = "";
  let action: Action = null;
  const { button, set } = powerButton(() => {
    options.power();
  });
  set(null);
  const end = element("div", "home-end");
  end.append(
    search((value) => {
      query = value;
      options.pick(room, query);
    }),
    button,
    setupLink(),
  );
  const pick = (value: string): void => {
    room = value;
    options.pick(room, query);
  };
  const node = element("div", "home-bar");
  node.append(options.rooms.length > 0 ? tabs(options.rooms, room, pick) : element("span", ""), end);
  return {
    node,
    show(next) {
      action = next;
      set(next);
    },
    busy(on) {
      button.replaceChildren(on ? loading({ size: "s" }) : icon(POWER));
      if (on) button.disabled = true;
      else set(action);
    },
  };
}
