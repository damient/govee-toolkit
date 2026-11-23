import "./home.css";
import { ALL, bar } from "./bar.ts";
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
import { ARROW, icon } from "../../components/icon/icon.ts";

const WAND =
  "M6 21L21 6l-3-3L3 18zM15 6l3 3M9 3a2 2 0 0 0 2 2 2 2 0 0 0-2 2 2 2 0 0 0-2-2 2 2 0 0 0 2-2M19 13a2 2 0 0 0 2 2 2 2 0 0 0-2 2 2 2 0 0 0-2-2 2 2 0 0 0 2-2";

/** The welcome card of a first run: it opens the setup. */
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

/**
 * The IDs of the lights that the power button of the bar turned off, which
 * it turns back on. They stay while Studio is open. A reload drops them, and
 * so does a light that the user turns on from its card or its panel.
 */
let turnedOff: string[] = [];

/** Turn `cards` on or off, and answer the IDs that took the change. A failure of the request shows in `box`. */
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

/** Turn back on the lights in `turnedOff`, or else turn off the lights of `shown` that are on and keep them in `turnedOff`. */
async function pressPower(cards: HomeCard[], shown: HomeCard[], box: HTMLElement): Promise<void> {
  const back = turnedOff.length > 0;
  const targets = back
    ? cards.filter((card) => turnedOff.includes(card.found.id))
    : shown.filter((card) => card.on() === true);
  const done = await switchAll(targets, back, box);
  turnedOff = back ? turnedOff.filter((id) => !done.includes(id)) : done;
}

/** The lights of the configuration, the tabs of the rooms, the search and the power button. */
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
  });

  // A light turned on by hand ends the "turn back on" of the bar: its next press turns off again.
  const turnedOn = (): void => {
    turnedOff = [];
    refresh();
  };
  for (const found of lights) cards.push(new HomeCard(found, { changed: refresh, open: openPanel, turnedOn }));
  grid.append(...cards.map((card) => card.node));
  box.append(top.node, grid, none);
  filter(room, "");
}

/** Draw the Home view: the configured lights, or the welcome card when there is none. */
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
