import "../../components/light/light.css";
import { NONE, type Outcome, type Status, apply, applyOne, status } from "../../core/control.ts";
import { type Found, labelOf } from "../../core/api.ts";
import { explain, t } from "../../core/i18n.ts";
import { familyIcon } from "../../components/family/family.ts";
import { latest } from "../../core/latest.ts";
import { loading } from "../../components/loading/loading.ts";
import { tooltip } from "../../components/tooltip/tooltip.ts";
import { icon } from "../../components/icon/icon.ts";
import { element } from "../../core/dom.ts";
import { statusMark } from "../../components/light/mark.ts";

export const POWER = "M7 6a7.75 7.75 0 1 0 10 0M12 4v8";
/** What a click on a card leaves to the element itself. */
const CONTROLS = "button, input, [tabindex]";

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

/** A power button and the function that shows the state of the light on it. */
export interface PowerButton {
  button: HTMLButtonElement;
  set: (on: boolean) => void;
}

/**
 * The power button of a light: cyan while the light is on. A click gives
 * `press` the wanted state, and the button stays disabled until it settles.
 * `spinner` shows a spinner in place of the icon meanwhile.
 */
export function powerButton(name: string, press: (wanted: boolean) => Promise<void>, spinner = false): PowerButton {
  const button = element("button", "home-power");
  button.type = "button";
  button.append(icon(POWER));
  const tip = tooltip(button, "");
  const set = (on: boolean): void => {
    const label = t(on ? "home.light.off" : "home.light.on", { name });
    button.setAttribute("aria-pressed", String(on));
    button.setAttribute("aria-label", label);
    tip(label);
  };
  button.addEventListener("click", () => {
    const wanted = button.getAttribute("aria-pressed") !== "true";
    button.disabled = true;
    if (spinner) button.replaceChildren(loading({ size: "s" }));
    void press(wanted).finally(() => {
      button.disabled = false;
      if (spinner) button.replaceChildren(icon(POWER));
    });
  });
  return { button, set };
}

/** What a card tells the home view. */
export interface CardEvents {
  /** After each change of state. */
  changed(): void;
  /** A click on a light that answered. */
  open(card: HomeCard): void;
  /** The user turned the light on, from the card or from its panel. */
  turnedOn(): void;
}

/** One configured light of the home view. The card asks the light for its state when it is made. */
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
  readonly name: string;
  private readonly title = element("button", "light-title home-open");
  private readonly events: CardEvents;
  private readonly power: PowerButton;
  /** `null` until the light answers, and while it does not answer. */
  private state: Status | null = null;
  /** Sends the level while the knob moves. */
  private readonly send = latest(
    async (level: number) => {
      await applyOne(this.found.id, { brightness: level });
      this.error.hidden = true;
      if (this.state) this.state = { ...this.state, brightness: level };
    },
    (failure) => {
      this.fail(explain(failure));
    },
  );

  constructor(found: Found, events: CardEvents) {
    this.found = found;
    this.events = events;
    this.name = labelOf(found);
    this.power = this.powerButton();
    this.title.type = "button";
    this.title.textContent = this.name;
    this.title.disabled = true;
    this.title.setAttribute("aria-label", t("home.light.open", { name: this.name }));
    this.title.addEventListener("click", () => {
      this.events.open(this);
    });
    // The title button serves the keyboard. A click elsewhere on the card does the same, outside its
    // controls. The gesture decides where it starts: a slider drag ends its click on the card, and the
    // power button replaces its icon before the click reaches the card.
    let fromControl = false;
    this.node.addEventListener("pointerdown", (event) => {
      fromControl = event.composedPath().some((item) => item instanceof Element && item.matches(CONTROLS));
    });
    this.node.addEventListener("click", (event) => {
      const control = fromControl || event.composedPath().some((item) => item instanceof Element && item.matches(CONTROLS));
      fromControl = false;
      if (!control && this.state !== null) this.events.open(this);
    });
    const top = element("div", "light-top home-top");
    const shape = element("span", "light-icon");
    this.dot.tabIndex = 0;
    this.dot.setAttribute("role", "img");
    this.dot.setAttribute("aria-label", t("light.checking"));
    this.tip = tooltip(this.dot, t("light.checking"));
    shape.append(familyIcon(found.family), this.dot);
    const text = element("div", "home-text");
    text.append(this.title, element("span", "home-sku", found.sku ?? ""));
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
    this.slot.append(statusMark(t("light.checking"), loading({ size: "s" })));
    this.read();
  }

  private powerButton(): PowerButton {
    const press = (wanted: boolean): Promise<void> =>
      apply([this.found.id], { power: wanted }).then(
        ([answer]) => {
          if (answer) this.powered(answer, wanted);
        },
        (failure: unknown) => {
          this.fail(explain(failure));
        },
      );
    return powerButton(this.name, press, true);
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

  /** The state, or `null` while the light does not answer. */
  snapshot(): Status | null {
    return this.state;
  }

  /** Show a change that the panel sent and the light took. */
  update(change: Partial<Status>): void {
    if (this.state === null) return;
    this.state = { ...this.state, ...change };
    this.draw();
    this.events.changed();
  }

  /**
   * Show what `apply` answered to a power change of this light. `wanted` is the state that it
   * asked for. `bar` is `true` for a change
   * from the power button of the bar: any other power on runs `events.turnedOn`.
   */
  powered(answer: Outcome, wanted: boolean, bar = false): void {
    if (answer.ok && wanted && !bar) this.events.turnedOn();
    if (answer.ok) {
      this.state = {
        brightness: null,
        color: null,
        kelvin: null,
        range: null,
        features: NONE,
        ...this.state,
        on: wanted,
      };
      // A light that is off still reports its color: the state of the last
      // read holds it. A read just after power on can still answer `on: false`.
      this.draw();
    } else {
      this.fail(answer.error ?? "");
    }
    this.events.changed();
  }

  private settle(state: Status | null): void {
    this.state = state;
    this.draw();
    this.events.changed();
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
    this.title.disabled = state === null;
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
    this.power.set(state.on === true);
    if (this.power.button.parentNode !== this.slot) this.slot.replaceChildren(this.power.button);
  }

}
