import "./rooms.css";
import { type Found, type Room, devices, rooms, saveRooms } from "../../core/api.ts";
import { chipFace, nameInput, roomCard } from "./room.ts";
import { element } from "../../core/dom.ts";
import { explain, t } from "../../core/i18n.ts";
import { go, pathOf } from "../../core/router.ts";
import { draggable } from "./drag.ts";
import { help } from "../../components/help/help.ts";
import { loading } from "../../components/loading/loading.ts";
import { notice } from "../../components/notice/notice.ts";
import { ALERT, CHECK, REFRESH, icon } from "../../components/icon/icon.ts";

const BULB = "M9 18h6M10 22h4M12 2a7 7 0 0 0-4 12.7V16h8v-1.3A7 7 0 0 0 12 2z";
const PLUS = "M12 5v14M5 12h14";

/** A light on its way: from the pool when `room` is `null`, or from the room at that index. */
interface Source {
  id: string;
  room: number | null;
}

const keyOf = (source: Source): string => `${source.room ?? "pool"}:${source.id}`;
const sameName = (a: string, b: string): boolean => a.localeCompare(b, undefined, { sensitivity: "accent" }) === 0;

function button(text: string, d: string, primary = false): HTMLButtonElement {
  const node = element("button", primary ? "button primary" : "button");
  node.type = "button";
  node.append(icon(d), text);
  return node;
}

/** Every change stays in the page until Finish. */
class Board {
  readonly root = element("div", "rooms-board");
  readonly draft: Room[];
  private readonly lights: Found[];
  private readonly byId: Map<string, Found>;
  private picked: Source | null = null;

  constructor(lights: Found[], saved: Room[]) {
    this.lights = lights;
    this.byId = new Map(lights.map((light) => [light.id, light]));
    this.draft = saved.map((room) => ({ ...room, devices: room.devices.filter((id) => this.byId.has(id)) }));
    this.root.addEventListener("click", this.click);
    this.root.addEventListener("keydown", this.key);
    this.render();
  }

  private focus(selector: string): void {
    this.root.querySelector<HTMLElement>(selector)?.focus();
  }

  /** Take the light `id` out of the room at `index`. The pool (`null`) keeps every light. */
  private out(index: number | null, id: string): void {
    const list = index === null ? undefined : this.draft[index]?.devices;
    const at = list?.indexOf(id) ?? -1;
    if (list !== undefined && at >= 0) list.splice(at, 1);
  }

  /** Put the light of `source` where `target` says. From the pool it adds, from a room it moves. */
  private place(source: Source, target: HTMLElement): void {
    const to = target.dataset.drop;
    if (to === "pool") {
      this.out(source.room, source.id);
    } else if (to !== undefined && Number(to) !== source.room) {
      const room = this.draft[Number(to)];
      if (room !== undefined) {
        this.out(source.room, source.id);
        if (!room.devices.includes(source.id)) room.devices.push(source.id);
      }
    }
    this.picked = null;
    this.render();
  }

  /** Pick the light of `source`, or drop it when it is the picked one. */
  private choose(source: Source): void {
    this.picked = this.picked !== null && keyOf(this.picked) === keyOf(source) ? null : source;
    this.render();
    this.focus(`[data-chip="${CSS.escape(keyOf(source))}"]`);
  }

  private readonly chip = (light: Found, room: number | null): HTMLElement => {
    const source: Source = { id: light.id, room };
    const node = element("div", "chip");
    node.tabIndex = 0;
    node.setAttribute("role", "button");
    node.dataset.chip = keyOf(source);
    node.setAttribute("aria-pressed", String(this.picked !== null && keyOf(this.picked) === keyOf(source)));
    node.append(...chipFace(light));
    node.addEventListener("click", (event) => {
      event.stopPropagation();
      this.choose(source);
    });
    node.addEventListener("keydown", (event) => {
      if (event.target !== node || (event.key !== "Enter" && event.key !== " ")) return;
      event.preventDefault();
      event.stopPropagation();
      this.choose(source);
    });
    draggable(node, this.root, (target) => {
      this.place(source, target);
    });
    return node;
  };

  /** Every light, with the number of its rooms. */
  private pool(): HTMLElement {
    const aside = element("aside", "rooms-pool");
    aside.dataset.drop = "pool";
    const h2 = element("h2", "section-title", t("setup.rooms.lights"));
    h2.append(help(t("setup.rooms.help")));
    const list = element("div", "pool-lights");
    for (const light of this.lights) {
      const node = this.chip(light, null);
      const count = this.draft.filter((room) => room.devices.includes(light.id)).length;
      if (count > 0) {
        const badge = element("span", "chip-badge", String(count));
        badge.setAttribute("aria-label", count === 1 ? t("setup.rooms.in.one") : t("setup.rooms.in", { count: String(count) }));
        node.append(badge);
      } else {
        node.classList.add("loose");
      }
      list.append(node);
    }
    aside.append(h2, list);
    return aside;
  }

  /** The card that adds a room. A name that another room has shows an error. */
  private adder(): HTMLElement {
    const card = element("div", "room-card room-new");
    const error = element("p", "room-error", t("setup.rooms.taken"));
    error.hidden = true;
    const add = button(t("setup.rooms.add"), PLUS);
    add.classList.add("xs");
    const input = nameInput("", () => {});
    const submit = (): void => {
      const name = input.value.trim();
      if (name === "") return;
      if (this.draft.some((room) => sameName(room.name, name))) {
        error.hidden = false;
        return;
      }
      this.draft.push({ id: "", name, devices: [] });
      this.render();
      this.focus(".room-new input");
    };
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter") submit();
    });
    input.addEventListener("input", () => {
      error.hidden = true;
    });
    add.addEventListener("click", submit);
    card.append(element("h3", "room-name", t("setup.rooms.new")), input, error, add);
    return card;
  }

  private card(room: Room, index: number): HTMLElement {
    const card = roomCard(room, index, this.byId, {
      rename: (name) => {
        if (this.draft.some((other, at) => at !== index && sameName(other.name, name))) return t("setup.rooms.taken");
        room.name = name;
        this.render();
        return null;
      },
      remove: () => {
        this.draft.splice(index, 1);
        this.picked = null;
        this.render();
      },
      take: (id) => {
        this.out(index, id);
        this.picked = null;
        this.render();
      },
      chip: this.chip,
    });
    if (this.picked !== null) card.tabIndex = 0;
    return card;
  }

  private render(): void {
    const grid = element("div", "rooms-grid");
    grid.append(...this.draft.map((room, index) => this.card(room, index)), this.adder());
    this.root.classList.toggle("picking", this.picked !== null);
    this.root.replaceChildren(this.pool(), grid);
  }

  /** A click on a target places the picked light. */
  private readonly click = (event: MouseEvent): void => {
    if (this.picked === null || !(event.target instanceof Element)) return;
    if (event.target.closest("button, input")) return;
    const target = event.target.closest<HTMLElement>("[data-drop]");
    if (target !== null) this.place(this.picked, target);
  };

  /** Enter on a target places the picked light. Esc drops it. */
  private readonly key = (event: KeyboardEvent): void => {
    if (this.picked === null) return;
    if (event.key === "Escape") {
      const key = keyOf(this.picked);
      this.picked = null;
      this.render();
      this.focus(`[data-chip="${CSS.escape(key)}"]`);
    } else if (event.key === "Enter" && event.target instanceof HTMLElement && event.target.dataset.drop !== undefined) {
      this.place(this.picked, event.target);
    }
  };
}

function finishRow(board: Board): HTMLElement {
  const error = element("p", "error");
  error.hidden = true;
  const finish = button(t("setup.rooms.finish"), CHECK, true);
  finish.addEventListener("click", () => {
    finish.disabled = true;
    error.hidden = true;
    saveRooms(board.draft).then(
      () => {
        go(pathOf("home"));
      },
      (failure: unknown) => {
        finish.disabled = false;
        error.hidden = false;
        error.textContent = explain(failure);
      },
    );
  });
  const row = element("div", "buttons end");
  row.append(error, finish);
  return row;
}

function filled(card: HTMLElement): HTMLElement {
  const fill = element("div", "fill");
  fill.append(card);
  return fill;
}

export function roomsStep(): HTMLElement {
  const section = element("section", "scan rooms");
  const run = (): void => {
    section.replaceChildren(loading({ block: true, text: t("setup.loading") }));
    Promise.all([devices(), rooms()]).then(
      ([lights, saved]) => {
        if (lights.length > 0) {
          const board = new Board(lights, saved);
          section.replaceChildren(board.root, finishRow(board));
          return;
        }
        const back = element("a", "button", t("setup.rooms.back"));
        back.href = pathOf("setup");
        section.replaceChildren(filled(notice({ icon: BULB, title: t("setup.rooms.none"), body: t("setup.rooms.none.body"), actions: [back] })));
      },
      (failure: unknown) => {
        const again = button(t("setup.again"), REFRESH);
        again.addEventListener("click", run);
        section.replaceChildren(filled(notice({ icon: ALERT, title: t("setup.failed"), body: explain(failure), actions: [again] })));
      },
    );
  };
  run();
  return section;
}
