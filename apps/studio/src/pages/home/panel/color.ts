import { FAVORITES, load, prefs, save } from "../../../core/prefs.ts";
import { type Wheel, wheel } from "./wheel.ts";
import { explain, t } from "../../../core/i18n.ts";
import { tooltip } from "../../../components/tooltip/tooltip.ts";
import type { Status } from "../../../core/control.ts";
import { CLOSE, STAR, icon } from "../../../components/icon/icon.ts";

export const WHITE = "#ffffff";
const BASE = ["#ff0000", "#ff7a00", "#ffd400", "#00ff14", "#0050ff", "#8a2bff", "#ff2bd6", WHITE];
/** The base colors of a drawing. Black leaves a LED unlit. */
export const DRAW_BASE = [...BASE, "#000000"];

/** Fires when the favorites change, so that every color part shows the same list. */
const changes = new EventTarget();

function announce(): void {
  changes.dispatchEvent(new Event("favorites"));
}

/** The color that the panel shows first: the color of the light, or white when it reports black (white mode or off). */
export function startColor(state: Status): string {
  const hex = state.color?.toLowerCase() ?? "#000000";
  return hex === "#000000" ? WHITE : hex;
}

/** The white of a light that has a white temperature: the slider shows while it is picked. */
export interface White {
  row: HTMLElement;
  /** The light is in white mode when the panel opens. */
  on: boolean;
  /** A pick of the white swatch. */
  pick(): void;
  /** The temperature of the slider, in kelvin. */
  kelvin(): number;
  /** Move the slider to `kelvin`, and send nothing. */
  set(kelvin: number): void;
}

function recolor(button: HTMLElement, hex: string): void {
  button.style.background = hex;
  button.dataset["color"] = hex;
  button.setAttribute("aria-label", t("panel.pick", { color: hex }));
}

function swatch(hex: string, pick: (hex: string) => void): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "panel-swatch";
  recolor(button, hex);
  button.addEventListener("click", () => {
    pick(button.dataset["color"] ?? hex);
  });
  return button;
}

/** A favorite swatch. The cross removes the color from the favorites. */
function favorite(hex: string, pick: (hex: string) => void, remove: (hex: string) => void): HTMLElement {
  const wrap = document.createElement("span");
  wrap.className = "panel-favorite";
  const cross = document.createElement("button");
  cross.type = "button";
  cross.className = "panel-remove";
  cross.setAttribute("aria-label", t("panel.favorite.remove"));
  cross.append(icon(CLOSE));
  cross.addEventListener("click", () => {
    remove(hex);
  });
  wrap.append(swatch(hex, pick), cross);
  return wrap;
}

function wheelToggle(wheelNode: HTMLElement, changed: () => void): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "panel-swatch panel-custom";
  const set = (open: boolean): void => {
    wheelNode.hidden = !open;
    button.setAttribute("aria-expanded", String(open));
  };
  button.setAttribute("aria-label", t("panel.custom"));
  tooltip(button, t("panel.custom"));
  button.addEventListener("click", () => {
    set(button.getAttribute("aria-expanded") !== "true");
    changed();
  });
  set(false);
  return button;
}

/** Redraw `part` when the favorites change. A weak reference lets a closed panel go before the next change. */
function follow(part: ColorPart): void {
  const weak = new WeakRef(part);
  const redraw = (): void => {
    const live = weak.deref();
    if (live?.node.isConnected === true) live.draw();
    else changes.removeEventListener("favorites", redraw);
  };
  changes.addEventListener("favorites", redraw);
}

/** The favorites are a setting of the server, so every light shows them. */
export class ColorPart {
  readonly node = document.createElement("div");
  private current: string;
  private readonly circle: Wheel;
  private readonly swatches = document.createElement("div");
  private readonly favorites = document.createElement("div");
  private readonly custom: HTMLButtonElement;
  private readonly keep = document.createElement("button");
  private readonly pick: (hex: string) => void;
  private readonly failed: (message: string) => void;
  private readonly white: White | null;
  private readonly base: readonly string[];
  /** The first swatch when it shows a color that no other swatch has. A drag of the wheel recolors it. */
  private extra: HTMLElement | null = null;

  /** `failed` shows an error of the save of the favorites. `titled` `false` leaves the title to a tab. `base` follows the favorites. */
  constructor(
    start: string,
    pick: (hex: string) => void,
    failed: (message: string) => void,
    white: White | null,
    titled = true,
    base: readonly string[] = BASE,
  ) {
    this.white = white;
    this.base = base;
    this.current = white?.on === true ? WHITE : start;
    this.pick = pick;
    this.failed = failed;
    this.node.className = "panel-color";
    this.swatches.className = "panel-swatches";
    this.swatches.setAttribute("aria-label", t("panel.colors"));
    this.favorites.className = "panel-swatches";
    this.circle = wheel((hex) => {
      this.current = hex;
      this.mark();
      if (white) white.row.hidden = true;
      pick(hex);
    });
    this.custom = wheelToggle(this.circle.node, () => {
      this.mark();
    });
    this.keep.type = "button";
    this.keep.className = "button xs panel-keep";
    this.keep.append(icon(STAR), t("panel.favorite.add"));
    this.keep.addEventListener("click", () => {
      this.store([...prefs().favorites, this.current]);
    });
    const label = document.createElement("span");
    label.className = "panel-label";
    label.textContent = t("panel.color");
    if (titled) this.node.append(label);
    this.node.append(this.swatches, this.favorites, this.circle.node, this.keep);
    if (white) {
      white.row.hidden = !white.on;
      this.node.append(white.row);
    }
    this.circle.set(this.current);
    this.draw();
    follow(this);
  }

  /** The color that the swatches and the wheel show. */
  color(): string {
    return this.current;
  }

  /** Show `hex` as the current color, and send nothing. */
  show(hex: string): void {
    this.current = hex;
    this.circle.set(hex);
    this.mark();
    if (this.white) this.white.row.hidden = hex !== WHITE;
  }

  private readonly choose = (hex: string): void => {
    this.current = hex;
    this.circle.set(hex);
    this.mark();
    const white = this.white;
    if (white) white.row.hidden = hex !== WHITE;
    if (white && hex === WHITE) white.pick();
    else this.pick(hex);
  };

  private readonly remove = (hex: string): void => {
    this.store(prefs().favorites.filter((kept) => kept !== hex));
  };

  private store(next: string[]): void {
    save({ favorites: next }).then(
      () => {
        announce();
      },
      (failure: unknown) => {
        this.failed(explain(failure));
        // `save` shows the change at once: read the list that the server kept.
        load().then(
          () => {
            announce();
          },
          () => null,
        );
      },
    );
    announce();
  }

  /** The base colors on the first row. The color of the wheel and the favorites on the second row. */
  draw(): void {
    const favorites = prefs().favorites;
    const extra = !this.base.includes(this.current) && !favorites.includes(this.current);
    this.extra = extra ? swatch(this.current, this.choose) : null;
    this.swatches.replaceChildren(...this.base.map((hex) => swatch(hex, this.choose)), this.custom);
    const added = favorites.map((hex) => favorite(hex, this.choose, this.remove));
    this.favorites.replaceChildren(...(this.extra ? [this.extra] : []), ...added);
    this.favorites.hidden = this.favorites.childElementCount === 0;
    this.mark();
  }

  /** Ring the swatch of the current color. The favorite button shows with the wheel, for a new color. */
  private mark(): void {
    const listed = [...this.node.querySelectorAll<HTMLElement>(".panel-swatch")];
    if (!listed.some((button) => button.dataset["color"] === this.current)) {
      if (this.extra === null) {
        this.draw();
        return;
      }
      recolor(this.extra, this.current);
    }
    for (const button of listed) {
      button.setAttribute("aria-pressed", String(button.dataset["color"] === this.current));
    }
    const known = prefs().favorites.includes(this.current) || this.base.includes(this.current);
    this.keep.disabled = prefs().favorites.length >= FAVORITES;
    this.keep.hidden = known || this.circle.node.hidden;
  }
}
