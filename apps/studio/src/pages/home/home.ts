import "./home.css";
import { ALL, type BarOptions, bar } from "./bar.ts";
import { type Found, type Room, devices, rooms } from "../../core/api.ts";
import { apply } from "../../core/control.ts";
import { openPanel } from "./panel/panel.ts";
import { HomeCard } from "./card.ts";
import { explain, t } from "../../core/i18n.ts";
import { readLocal, writeLocal } from "../../core/prefs.ts";
import { alert } from "../../components/alert/alert.ts";
import { loading } from "../../components/loading/loading.ts";
import { notice } from "../../components/notice/notice.ts";
import { pathOf } from "../../core/router.ts";
import { ARROW, PLUS, icon } from "../../components/icon/icon.ts";
import { element } from "../../core/dom.ts";
import { latest } from "../../core/latest.ts";

const WAND =
  "M6 21L21 6l-3-3L3 18zM15 6l3 3M9 3a2 2 0 0 0 2 2 2 2 0 0 0-2 2 2 2 0 0 0-2-2 2 2 0 0 0 2-2M19 13a2 2 0 0 0 2 2 2 2 0 0 0-2 2 2 2 0 0 0-2-2 2 2 0 0 0 2-2";

function welcome(view: HTMLElement): void {
  view.classList.add("centered");
  const start = document.createElement("a");
  start.className = "button primary";
  start.href = pathOf("setup");
  start.append(t("home.welcome.start"), icon(ARROW));
  view.append(
    notice({ icon: WAND, title: t("home.welcome.title"), body: t("home.welcome.body"), tone: "ember", level: "h1", actions: [start] }),
  );
}

const matches = (found: Found, query: string): boolean =>
  query === "" || [found.name, found.sku].some((text) => text?.toLowerCase().includes(query) === true);

function addCard(): HTMLAnchorElement {
  const link = element("a", "light-card home-add");
  link.href = pathOf("setup");
  const mark = element("span", "light-icon");
  mark.append(icon(PLUS));
  link.append(mark, element("span", "light-title", t("home.add")));
  return link;
}

/** The mean brightness of the lights shown that are on, in percent. 100 when none is on. */
function meanLevel(cards: HomeCard[]): number {
  const levels = cards.flatMap((card) => {
    const state = card.snapshot();
    return state?.on === true && state.brightness !== null ? [state.brightness] : [];
  });
  return levels.length > 0 ? Math.round(levels.reduce((sum, level) => sum + level, 0) / levels.length) : 100;
}

/** One request for each level: the lights of one range take one request together. */
async function dimAll(cards: HomeCard[], percent: number): Promise<void> {
  const groups = new Map<number, HomeCard[]>();
  for (const card of cards) {
    const level = card.aim(percent);
    if (level !== null) groups.set(level, [...(groups.get(level) ?? []), card]);
  }
  await Promise.all(
    [...groups].map(async ([level, members]) => {
      const answers = await apply(
        members.map((card) => card.found.id),
        { brightness: level },
      );
      for (const answer of answers) members.find((card) => card.found.id === answer.id)?.dimmed(answer, level);
    }),
  );
}

function together(cards: HomeCard[], shown: () => HomeCard[], box: HTMLElement): Pick<BarOptions, "refresh" | "level" | "dim"> {
  const send = latest(
    (percent: number) => dimAll(shown(), percent),
    (failure) => {
      box.append(alert({ tone: "error", title: explain(failure) }));
    },
  );
  return {
    async refresh() {
      await Promise.all(cards.map((card) => card.read()));
    },
    level: () => meanLevel(shown()),
    dim: send,
  };
}

/** The lights that the power button of the bar turned off. A reload drops them, and so does a light turned on by hand. */
let turnedOff: string[] = [];

/** Answer the IDs that took the change. A failure of the request shows in `box`. */
async function switchAll(cards: HomeCard[], on: boolean, box: HTMLElement): Promise<string[]> {
  try {
    const answers = await apply(
      cards.map((card) => card.found.id),
      { power: on },
    );
    for (const answer of answers) cards.find((card) => card.found.id === answer.id)?.powered(answer, on, true);
    return answers.filter((answer) => answer.ok).map((answer) => answer.id);
  } catch (failure) {
    box.append(alert({ tone: "error", title: explain(failure) }));
    return [];
  }
}

async function pressPower(cards: HomeCard[], shown: HomeCard[], box: HTMLElement): Promise<void> {
  const back = turnedOff.length > 0;
  const targets = back
    ? cards.filter((card) => turnedOff.includes(card.found.id))
    : shown.filter((card) => card.on() === true);
  const done = await switchAll(targets, back, box);
  turnedOff = back ? turnedOff.filter((id) => !done.includes(id)) : done;
}

function board(box: HTMLElement, lights: Found[], list: Room[]): void {
  const room = readLocal("home.room", [ALL, ...list.map((item) => item.id)], ALL);
  const grid = document.createElement("div");
  grid.className = "light-grid";
  const none = document.createElement("p");
  none.className = "home-none";
  none.textContent = t("home.none");
  const cards: HomeCard[] = [];
  const shown = (): HomeCard[] => cards.filter((card) => card.node.hidden !== true);

  const refresh = (): void => {
    if (turnedOff.length > 0) top.show("on");
    else top.show(shown().some((card) => card.on() === true) ? "off" : null);
  };

  const filter = (current: string, query: string): void => {
    writeLocal("home.room", current);
    const members = list.find((item) => item.id === current)?.devices;
    for (const card of cards) {
      card.node.hidden = (members !== undefined && !members.includes(card.found.id)) || !matches(card.found, query);
    }
    none.hidden = shown().length > 0;
    refresh();
  };

  const top = bar({
    rooms: list,
    room,
    pick: filter,
    power() {
      top.busy(true);
      void pressPower(cards, shown(), box).finally(() => {
        top.busy(false);
        refresh();
      });
    },
    ...together(cards, shown, box),
  });

  const turnedOn = (): void => {
    turnedOff = [];
    refresh();
  };
  for (const found of lights) cards.push(new HomeCard(found, { changed: refresh, open: openPanel, turnedOn }));
  grid.append(...cards.map((card) => card.node), addCard());
  box.append(top.node, grid, none);
  filter(room, "");
}

export function home(view: HTMLElement): void {
  // The answer can come after a move to another page: it then fills a detached box.
  const box = document.createElement("div");
  box.className = "home";
  box.append(loading({ block: true }));
  view.append(box);
  Promise.all([devices(), rooms()]).then(
    ([lights, list]) => {
      if (!box.isConnected) return;
      box.replaceChildren();
      if (lights.length > 0) board(box, lights, list);
      else {
        box.remove();
        welcome(view);
      }
    },
    (failure: unknown) => {
      box.replaceChildren(alert({ tone: "error", title: explain(failure) }));
    },
  );
}
