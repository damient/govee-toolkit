import "../../components/light/light.css";
import { type Found, type Powered, type Status, power, setBrightness, status } from "../../core/api.ts";
import { explain, t } from "../../core/i18n.ts";
import { familyIcon } from "../../components/family/family.ts";
import { icon } from "../../shell/nav.ts";
import { loading } from "../../components/loading/loading.ts";
import { tooltip } from "../../components/tooltip/tooltip.ts";

export const POWER = "M7 6a7.75 7.75 0 1 0 10 0M12 4v8";
/** The light needs a moment after power on before its state gives the color. */
const SETTLE_MS = 300;

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, text = ""): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  node.className = className;
  node.textContent = text;
  return node;
}

/** The color of a white temperature, for the swatch. The steps are approximate. */
function white(kelvin: number): string {
  if (kelvin < 3500) return "#ffc98a";
  if (kelvin < 5000) return "#ffe7c4";
  return "#e8f0ff";
}

/** The color that the light shows, or `null` when it is off or does not say. */
function shown(state: Status): string | null {
  if (state.on !== true) return null;
  if (state.kelvin !== null && state.kelvin > 0) return white(state.kelvin);
  const hex = state.color?.replace(/^#/u, "").toLowerCase() ?? "000000";
  return hex === "000000" ? null : `#${hex}`;
}

/** The mark of a light that has no state: the request runs, or the light did not answer. */
function mark(text: string, content: Node): HTMLElement {
  const node = element("span", "light-find light-offline");
  node.tabIndex = 0;
  node.setAttribute("role", "img");
  node.setAttribute("aria-label", text);
  node.append(content);
  tooltip(node, text);
  return node;
}

/**
 * One configured light of the home view. The card asks the light for its
 * state when it is made. `changed` runs after each change of state.
 */
export class HomeCard {
  readonly found: Found;
  readonly node = element("article", "light-card home-card");
  private readonly slot = element("span", "home-slot");
  /** On the icon: green when the light answers, gray when it does not. */
  private readonly dot = element("span", "home-dot");
  private readonly tip: (text: string) => void;
  private readonly level = element("input", "home-level");
  private readonly swatch = element("span", "home-swatch");
  private readonly error = element("p", "light-error");
  private readonly name: string;
  private readonly changed: () => void;
  /** `null` until the light answers, and while it does not answer. */
  private state: Status | null = null;
  private sending = false;
  /** The level that waits for the request in flight. */
  private next: number | null = null;

  constructor(found: Found, changed: () => void) {
    this.found = found;
    this.changed = changed;
    this.name = found.name ?? found.sku ?? found.id;
    const top = element("div", "light-top home-top");
    const shape = element("span", "light-icon");
    this.dot.tabIndex = 0;
    this.dot.setAttribute("role", "img");
    this.dot.setAttribute("aria-label", t("light.checking"));
    this.tip = tooltip(this.dot, t("light.checking"));
    shape.append(familyIcon(found.family), this.dot);
    const text = element("div", "home-text");
    text.append(element("span", "light-title", this.name), element("span", "home-sku", found.sku ?? ""));
    top.append(shape, text, this.slot);
    this.error.hidden = true;
    this.level.type = "range";
    this.level.hidden = true;
    this.level.setAttribute("aria-label", t("home.light.level", { name: this.name }));
    this.level.addEventListener("input", () => {
      this.send(Number(this.level.value));
    });
    // The color bar is the track of the slider: the knob alone shows.
    this.swatch.append(this.level);
    this.node.append(top, this.swatch, this.error);
    this.slot.append(mark(t("light.checking"), loading({ size: "s" })));
    this.read();
  }

  /** Ask the light for its state. */
  private read(): void {
    status(this.found.id).then(
      (answer) => {
        this.settle(answer);
      },
      () => {
        this.settle(null);
      },
    );
  }

  /** The power state, or `null` while it is not known. */
  on(): boolean | null {
    return this.state?.on ?? null;
  }

  /**
   * Show what `power` answered for this light. `wanted` is the state that it
   * asked for. A light turned on is read again, for its color.
   */
  apply(answer: Powered, wanted: boolean): void {
    if (answer.ok) {
      this.state = { brightness: null, color: null, kelvin: null, range: null, ...this.state, on: wanted };
      this.draw();
      if (wanted) {
        setTimeout(() => {
          this.read();
        }, SETTLE_MS);
      }
    } else {
      this.fail(answer.error ?? "");
    }
    this.changed();
  }

  /**
   * Send `level` while the knob moves. One request is in flight at most: the
   * last level waits for the answer, and the levels between are dropped.
   */
  private send(level: number): void {
    if (this.sending) {
      this.next = level;
      return;
    }
    this.sending = true;
    void setBrightness(this.found.id, level)
      .then(
        () => {
          this.error.hidden = true;
          if (this.state) this.state = { ...this.state, brightness: level };
        },
        (failure: unknown) => {
          this.next = null;
          this.fail(explain(failure));
        },
      )
      .finally(() => {
        this.sending = false;
        const next = this.next;
        this.next = null;
        if (next !== null && next !== level) this.send(next);
      });
  }

  private settle(state: Status | null): void {
    this.state = state;
    this.draw();
    this.changed();
  }

  private fail(message: string): void {
    this.draw();
    this.error.hidden = false;
    this.error.textContent = message;
  }

  private draw(): void {
    const state = this.state;
    this.error.hidden = true;
    this.node.classList.toggle("offline", state === null);
    this.node.classList.toggle("lit", state?.on === true);
    this.swatch.style.background = (state === null ? null : shown(state)) ?? "";
    this.level.hidden = (state?.range ?? null) === null;
    const reach = t(state === null ? "light.offline" : "home.light.online");
    this.dot.classList.toggle("online", state !== null);
    this.dot.setAttribute("aria-label", reach);
    this.tip(reach);
    if (state === null) {
      this.slot.replaceChildren();
      return;
    }
    if (state.range !== null) {
      const [low, high] = state.range;
      this.level.min = String(low);
      this.level.max = String(high);
      this.level.value = String(Math.min(high, Math.max(low, state.brightness ?? high)));
      this.level.disabled = state.on !== true;
    }
    this.slot.replaceChildren(this.toggle(state.on === true));
  }

  private toggle(on: boolean): HTMLButtonElement {
    const button = element("button", "home-power");
    button.type = "button";
    button.setAttribute("aria-pressed", String(on));
    const label = t(on ? "home.light.off" : "home.light.on", { name: this.name });
    button.setAttribute("aria-label", label);
    button.append(icon(POWER));
    tooltip(button, label);
    button.addEventListener("click", () => {
      button.disabled = true;
      button.replaceChildren(loading({ size: "s" }));
      power([this.found.id], !on).then(
        ([answer]) => {
          if (answer) this.apply(answer, !on);
        },
        (failure: unknown) => {
          this.fail(explain(failure));
        },
      );
    });
    return button;
  }
}
