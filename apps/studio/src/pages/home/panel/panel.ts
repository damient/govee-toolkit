import "./panel.css";
import { type Change, type Status, apply, applyOne } from "../../../core/control.ts";
import { type Memory, type ZonesPart, memory, zonesPart } from "./zones.ts";
import { explain, t } from "../../../core/i18n.ts";
import { ColorPart, type White } from "./color.ts";
import { type HomeCard, powerButton } from "../card.ts";
import { familyIcon } from "../../../components/family/family.ts";
import { latest } from "../../../core/latest.ts";
import { sheet } from "../../../components/sheet/sheet.ts";
import { tooltip } from "../../../components/tooltip/tooltip.ts";
import { element } from "../../../core/dom.ts";

interface Switch {
  row: HTMLElement;
  set(on: boolean): void;
}

function switchRow(text: string, on: boolean, click: (on: boolean) => void): Switch {
  const row = element("label", "panel-row");
  const button = element("button", "switch");
  button.type = "button";
  button.setAttribute("role", "switch");
  const set = (next: boolean): void => {
    button.setAttribute("aria-checked", String(next));
  };
  button.addEventListener("click", () => {
    click(button.getAttribute("aria-checked") !== "true");
  });
  set(on);
  row.append(element("span", "panel-label", text), button);
  return { row, set };
}

interface Slider {
  row: HTMLElement;
  /** The bar under the knob, as the color bar of a card. */
  track: HTMLElement;
}

/** A value out of the bounds shows at the nearest bound. A `null` text names the slider for a screen reader only. */
function slider(
  text: string | null,
  name: string,
  [low, high]: [number, number],
  level: number | null,
  change: (level: number) => void,
): Slider {
  const row = element("label", "panel-field");
  const track = element("span", "home-swatch panel-track");
  const input = element("input", "home-level");
  input.setAttribute("aria-label", name);
  input.type = "range";
  input.min = String(low);
  input.max = String(high);
  input.value = String(Math.min(high, Math.max(low, level ?? high)));
  input.addEventListener("input", () => {
    change(Number(input.value));
  });
  track.append(input);
  if (text !== null) row.append(element("span", "panel-label", text));
  row.append(track);
  return { row, track };
}

function head(card: HomeCard): Node[] {
  const shape = element("span", "light-icon");
  shape.append(familyIcon(card.found.family));
  const text = element("div", "home-text");
  text.append(element("h2", "light-title panel-name", card.name), element("span", "home-sku", card.found.sku ?? ""));
  return [shape, text];
}

function start(state: Status): string {
  const hex = state.color?.toLowerCase() ?? "#000000";
  return hex === "#000000" ? "#ffffff" : hex;
}

/** It waits for the library view. */
function libraryButton(): HTMLButtonElement {
  const button = element("button", "button", t("panel.library"));
  button.type = "button";
  button.setAttribute("aria-disabled", "true");
  tooltip(button, t("panel.soon"));
  return button;
}

/** Each change goes to the light at once, with one request in flight at most. */
class LightPanel {
  private readonly id: string;
  private readonly error = element("p", "light-error");
  private readonly controls = element("div", "panel-controls");
  private readonly kept: Memory | null;
  private readonly zones: ZonesPart | null;
  /** Shows the state of the power button. `power` sets it. */
  private showPower: (lit: boolean) => void = () => {
    // The button does not exist before `show`.
  };
  private readonly send = latest(
    async (change: Change) => {
      await this.change(change);
      this.took(change);
    },
    (failure) => {
      this.fail(explain(failure));
    },
  );

  private readonly card: HomeCard;
  /** The state of the power switch when the panel opens. */
  private readonly on: boolean;

  constructor(card: HomeCard, state: Status) {
    this.card = card;
    this.id = card.found.id;
    this.error.hidden = true;
    const count = state.features.segments;
    this.kept = count === null ? null : memory(this.id, count);
    this.zones = this.kept === null ? null : zonesPart(this.kept);
    const white = this.white(state);
    if (state.features.color) this.controls.append(new ColorPart(start(state), this.paint, this.fail, white).node);
    else if (white !== null) this.controls.append(white.row);
    if (state.range !== null) {
      const name = t("panel.brightness");
      const { row } = slider(name, name, state.range, state.brightness, (level) => {
        this.send({ brightness: level });
      });
      this.controls.append(row);
    }
    if (this.zones !== null) this.controls.append(this.zones.node);
    if (this.kept !== null && state.features.gradient) this.controls.append(this.gradient(this.kept).row);
    this.on = state.on === true;
  }

  show(): void {
    const panel = sheet({ head: head(this.card), end: [this.power(this.on)], label: this.card.name });
    panel.body.append(this.controls, this.error);
    panel.foot.append(libraryButton());
  }

  /** The white temperature slider, or `null` when the mode has none. The white swatch sends its value. */
  private white(state: Status): White | null {
    const bounds = state.features.colorTemp;
    if (bounds === null) return null;
    const on = state.kelvin !== null && state.kelvin > 0;
    let kelvin = on && state.kelvin !== null ? state.kelvin : Math.round((bounds[0] + bounds[1]) / 2);
    const { row, track } = slider(null, t("panel.white"), bounds, kelvin, (value) => {
      kelvin = value;
      this.send({ kelvin });
    });
    track.classList.add("panel-white");
    return {
      row,
      on,
      pick: () => {
        this.send({ kelvin });
      },
    };
  }

  private readonly fail = (message: string): void => {
    this.error.hidden = false;
    this.error.textContent = message;
  };

  /** Paint the zones selected, or the whole light when none is. */
  private readonly paint = (hex: string): void => {
    const picked = this.zones?.selected() ?? [];
    if (picked.length === 0 || this.kept === null) this.send({ color: hex });
    else this.send({ segment: { zones: picked, color: hex }, gradient: this.kept.gradient });
  };

  /** Show a change that the light took, on the card and on the zones. */
  private took(change: Change): void {
    this.error.hidden = true;
    if (change.brightness !== undefined) this.card.update({ brightness: change.brightness });
    if (change.kelvin !== undefined) this.card.update({ kelvin: change.kelvin });
    if (change.color !== undefined) {
      this.card.update({ color: change.color, kelvin: 0 });
      this.kept?.colors.fill(change.color);
    }
    const segment = change.segment;
    if (segment !== undefined && this.kept !== null) {
      for (const zone of segment.zones) this.kept.colors[zone] = segment.color;
    }
    this.zones?.draw();
  }

  /** Send `change`. A light that is off turns on first, in the same request. */
  private async change(change: Change): Promise<void> {
    const wake = this.card.on() === false;
    await applyOne(this.id, wake ? { ...change, power: true } : change);
    if (!wake) return;
    this.card.powered({ id: this.id, ok: true, error: null }, true);
    this.showPower(true);
  }

  /** The power button of the head, as the one of the card. */
  private power(on: boolean): HTMLButtonElement {
    const { button, set } = powerButton(this.card.name, (wanted) =>
      apply([this.id], { power: wanted }).then(
        ([answer]) => {
          if (!answer) return;
          this.card.powered(answer, wanted);
          if (answer.ok) {
            this.error.hidden = true;
            set(wanted);
          } else this.fail(answer.error ?? "");
        },
        (failure: unknown) => {
          this.fail(explain(failure));
        },
      ),
    );
    this.showPower = set;
    set(on);
    return button;
  }

  private gradient(kept: Memory): Switch {
    const row = switchRow(t("panel.gradient"), kept.gradient, (on) => {
      this.change({ gradient: on }).then(
        () => {
          this.error.hidden = true;
          kept.gradient = on;
          row.set(on);
        },
        (failure: unknown) => {
          this.fail(explain(failure));
        },
      );
    });
    return row;
  }
}

/** A light that does not answer opens nothing. */
export function openPanel(card: HomeCard): void {
  const state = card.snapshot();
  if (state !== null) new LightPanel(card, state).show();
}
