import { home } from "./home.ts";
import { watchCamera } from "./insets.ts";
import { type Language, explain, language, load, t, translate } from "./i18n.ts";
import { mobile } from "./mobile.ts";
import { mount, type Page, type Route } from "./nav.ts";
import { load as loadPrefs, save } from "./prefs.ts";
import { rail } from "./rail.ts";
import { settings } from "./settings.ts";
import { rooms, setup } from "./setup.ts";
import { soon } from "./soon.ts";
import { splash } from "./splash.ts";
import { applyTheme, paint } from "./theme.ts";

/** A change of language shows the splash at least this long. */
const LANGUAGE_MS = 700;

const empty = (): void => {};

const LIGHTS = "M9 18h6M10 21h4M12 3a6 6 0 0 0-4 10.5c.7.7 1 1.5 1 2.5h6c0-1 .3-1.8 1-2.5A6 6 0 0 0 12 3z";
const LIBRARY = "M5 4h4v16H5zM9 4h4v16H9zM14 5l4-1 3 15-4 1z";
const LIVE = "M4 6a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2zM10 8.5v7l6-3.5z";
const STAGE = "M4 4h16v16H4zM8 4v16M16 4v16M4 8h4M4 16h4M16 8h4M16 16h4M4 12h16";

const top: Page[] = [
  { id: "home", label: "nav.home", render: home, icon: "M5 12H3l9-9 9 9h-2M5 12v7a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-7M9 21v-6h6v6" },
  {
    id: "lights",
    label: "nav.lights",
    icon: LIGHTS,
    render: (view) => {
      soon(view, { title: "nav.lights", body: "soon.lights", icon: LIGHTS });
    },
  },
  {
    id: "library",
    label: "nav.library",
    icon: LIBRARY,
    render: (view) => {
      soon(view, { title: "nav.library", body: "soon.library", icon: LIBRARY });
    },
  },
  {
    id: "live",
    label: "nav.live",
    icon: LIVE,
    render: (view) => {
      soon(view, { title: "nav.live", body: "soon.live", icon: LIVE });
    },
  },
  {
    id: "stage",
    label: "nav.stage",
    icon: STAGE,
    // The 3D view and the timeline need the room of a tablet at least.
    min: "tablet",
    render: (view) => {
      soon(view, { title: "nav.stage", body: "soon.stage", icon: STAGE });
    },
  },
];

const bottom: Page[] = [
  {
    id: "mobile",
    label: "nav.mobile",
    min: "desktop",
    render: mobile,
    icon: "M6 5a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2zM11 4h2M12 17v.01",
  },
  {
    id: "settings",
    label: "nav.settings",
    width: "narrow",
    render: (view) => {
      settings(view, change);
    },
    icon: "M12 9a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM10.3 4.3a1.7 1.7 0 0 1 3.4 0 1.7 1.7 0 0 0 2.6 1.1 1.7 1.7 0 0 1 2.3 2.3 1.7 1.7 0 0 0 1.1 2.6 1.7 1.7 0 0 1 0 3.4 1.7 1.7 0 0 0-1.1 2.6 1.7 1.7 0 0 1-2.3 2.3 1.7 1.7 0 0 0-2.6 1.1 1.7 1.7 0 0 1-3.4 0 1.7 1.7 0 0 0-2.6-1.1 1.7 1.7 0 0 1-2.3-2.3 1.7 1.7 0 0 0-1.1-2.6 1.7 1.7 0 0 1 0-3.4 1.7 1.7 0 0 0 1.1-2.6 1.7 1.7 0 0 1 2.3-2.3 1.7 1.7 0 0 0 2.6-1.1z",
  },
];

const hidden: Route[] = [
  { id: "setup", label: "nav.setup", render: setup },
  { id: "setup/rooms", label: "nav.setup", render: rooms },
];

function byId(id: string): HTMLElement {
  const element = document.querySelector<HTMLElement>(`#${id}`);
  if (!element) throw new Error(`#${id} is missing from index.html`);
  return element;
}

paint();
watchCamera();
const screen = splash(byId("splash"), byId("splash-note"));
let refresh = empty;
let relabelRail = empty;

async function relabel(code: Language): Promise<void> {
  await load(code);
  translate();
  relabelRail();
  refresh();
}

async function boot(): Promise<void> {
  await loadPrefs();
  applyTheme();
  await load(language());
  translate();
  const app = document.querySelector<HTMLElement>(".app");
  const button = document.querySelector<HTMLButtonElement>("#collapse");
  if (app && button) relabelRail = rail(app, button);
  refresh = mount(byId("nav"), byId("view"), top, bottom, hidden);
}

function fail(error: unknown): void {
  screen.fail(t("splash.error", { reason: explain(error) }));
}

function change(code: Language): void {
  const task = async (): Promise<void> => {
    await save({ language: code });
    await relabel(code);
  };
  screen.hold(task, LANGUAGE_MS).catch(fail);
}

// `npm run dev` sends this event when a file in `locales/` changes.
addEventListener("studio:locale", () => {
  relabel(language()).catch(fail);
});

try {
  await screen.hold(boot);
} catch (error) {
  fail(error);
}
