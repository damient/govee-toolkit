import { FAVORITES, load, prefs, save } from "../../../core/prefs.ts";
import { type Wheel, wheel } from "./wheel.ts";
import { explain, t } from "../../../core/i18n.ts";
import { tooltip } from "../../../components/tooltip/tooltip.ts";

const WHITE = "#ffffff";
const BASE = ["#ff0000", "#ff7a00", "#ffd400", "#00ff14", "#0050ff", "#8a2bff", "#ff2bd6", WHITE];
/** The base colors of a drawing. Black leaves a LED unlit. */
export const DRAW_BASE = [...BASE, "#000000"];

/** The white of a light that has a white temperature: the slider shows while it is picked. */
export interface White {
  row: HTMLElement;
  /** The light is in white mode when the panel opens. */
  on: boolean;
  /** A pick of the white swatch. */
  pick(): void;
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

/** The favorites are a setting of the server, so every light shows them. */
export class ColorPart {
  readonly node = document.createElement("div");
  private current: string;
  private readonly circle: Wheel;
  private readonly swatches = document.createElement("div");
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
    this.keep.addEventListener("click", () => {
      this.toggle();
    });
    const label = document.createElement("span");
    label.className = "panel-label";
    label.textContent = t("panel.color");
    if (titled) this.node.append(label);
    this.node.append(this.swatches, this.circle.node, this.keep);
    if (white) {
      white.row.hidden = !white.on;
      this.node.append(white.row);
    }
    this.circle.set(this.current);
    this.draw();
  }

  /** The color that the swatches and the wheel show. */
  color(): string {
    return this.current;
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

  /** Add the current color to the favorites, or remove it. */
  private toggle(): void {
    const list = prefs().favorites;
    const next = list.includes(this.current) ? list.filter((hex) => hex !== this.current) : [...list, this.current];
    save({ favorites: next }).then(
      () => {
        this.draw();
      },
      (failure: unknown) => {
        this.failed(explain(failure));
        // `save` shows the change at once: read the list that the server kept.
        load().then(
          () => {
            this.draw();
          },
          () => null,
        );
      },
    );
    this.draw();
  }

  /** A base color that is a favorite shows once, among the favorites. */
  private draw(): void {
    const favorites = prefs().favorites;
    const colors = [...favorites, ...this.base.filter((hex) => !favorites.includes(hex))];
    const extra = !colors.includes(this.current);
    if (extra) colors.unshift(this.current);
    const nodes = colors.map((hex) => swatch(hex, this.choose));
    this.extra = extra ? (nodes[0] ?? null) : null;
    this.swatches.replaceChildren(...nodes, this.custom);
    this.mark();
  }

  /**
   * Ring the swatch of the current color, and name what the favorite button
   * does. The button shows with the wheel, and for a favorite color.
   */
  private mark(): void {
    const listed = [...this.swatches.querySelectorAll<HTMLElement>(".panel-swatch")];
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
    const kept = prefs().favorites.includes(this.current);
    this.keep.textContent = t(kept ? "panel.favorite.remove" : "panel.favorite.add");
    this.keep.disabled = !kept && prefs().favorites.length >= FAVORITES;
    this.keep.hidden = !kept && this.circle.node.hidden;
  }
}
