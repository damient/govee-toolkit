import type { Room } from "../../core/api.ts";
import { loading } from "../../components/loading/loading.ts";
import { POWER } from "./card.ts";
import { t } from "../../core/i18n.ts";
import { tooltip } from "../../components/tooltip/tooltip.ts";
import { REFRESH, icon } from "../../components/icon/icon.ts";
import { brightness, search, toolButton } from "./tools.ts";
import { element } from "../../core/dom.ts";

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
  /** Read the state of every light again. The promise settles when all have answered. */
  refresh(): Promise<void>;
  /** The start value of the brightness slider, in percent. */
  level(): number;
  /** A move of the brightness slider, in percent. */
  dim(percent: number): void;
}

/** The rooms as tabs, and as a dropdown that takes their place at phone width. */
function roomPicker(rooms: Room[], current: string, pick: (room: string) => void): HTMLElement {
  const wrap = element("div", "home-rooms");
  const group = element("div", "seg home-tabs");
  group.setAttribute("role", "radiogroup");
  group.setAttribute("aria-label", t("home.rooms"));
  const list = element("select", "home-select");
  list.setAttribute("aria-label", t("home.rooms"));
  const radios: HTMLInputElement[] = [];
  for (const room of [{ id: ALL, name: t("home.all") }, ...rooms]) {
    const label = element("label", "");
    const input = element("input", "");
    input.type = "radio";
    input.name = "home-room";
    input.value = room.id;
    input.checked = room.id === current;
    input.addEventListener("change", () => {
      list.value = room.id;
      pick(room.id);
    });
    radios.push(input);
    label.append(input, room.name);
    group.append(label);
    const option = element("option", "", room.name);
    option.value = room.id;
    list.append(option);
  }
  list.value = current;
  list.addEventListener("change", () => {
    for (const radio of radios) radio.checked = radio.value === list.value;
    pick(list.value);
  });
  wrap.append(group, list);
  return wrap;
}

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

function refreshButton(click: () => void): HTMLButtonElement {
  const button = toolButton(REFRESH, t("home.refresh"));
  button.addEventListener("click", click);
  return button;
}

export function bar(options: BarOptions): Bar {
  let room = options.room;
  let query = "";
  let action: Action = null;
  const { button, set } = powerButton(() => {
    options.power();
  });
  set(null);
  const reload = refreshButton(() => {
    reload.disabled = true;
    reload.classList.add("spin");
    void options.refresh().finally(() => {
      reload.disabled = false;
      reload.classList.remove("spin");
    });
  });
  const end = element("div", "home-end");
  end.append(
    search((value) => {
      query = value;
      options.pick(room, query);
    }),
    brightness(
      () => options.level(),
      (percent) => {
        options.dim(percent);
      },
    ),
    reload,
    button,
  );
  const pick = (value: string): void => {
    room = value;
    options.pick(room, query);
  };
  const node = element("div", "home-bar");
  node.append(options.rooms.length > 0 ? roomPicker(options.rooms, room, pick) : element("span", ""), end);
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
