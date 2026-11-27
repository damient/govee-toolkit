import "./panel.css";
import { type Change, type Status, apply, applyOne } from "../../../core/control.ts";
import { type Memory, type ZonesPart, memory, zonesPart } from "./zones.ts";
import { explain, t } from "../../../core/i18n.ts";
import { ColorPart, DRAW_BASE, type White } from "./color.ts";
import { type HomeCard, powerButton } from "../card.ts";
import { familyIcon } from "../../../components/family/family.ts";
import { latest } from "../../../core/latest.ts";
import { BRUSH, PALETTE } from "../../../components/icon/icon.ts";
import { type Tab, tabs } from "./tabs.ts";
import { sheet } from "../../../components/sheet/sheet.ts";
import { tooltip } from "../../../components/tooltip/tooltip.ts";
import { element } from "../../../core/dom.ts";
import { type Switch, slider, switchRow } from "./fields.ts";

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
  /** The colors of the zone tab. A zone that the selection takes gets its color at once. */
  private brush: ColorPart | null = null;
  /** The colors at the start of a drag. A LED that the drag takes out of the selection gets its color back. */
  private before: (string | null)[] | null = null;
  /** `true`: the zones are the LEDs of `Change.draw`, else the zones of `Change.segment`. */
  private readonly leds: boolean;
  /** The color of a LED that Studio did not paint, for a drawing that sends every LED. */
  private readonly base: string;
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
    const { draw } = state.features;
    this.leds = draw !== null;
    this.base = start(state);
    const count = draw?.cells ?? state.features.segments;
    this.kept = count === null ? null : memory(this.id, count);
    const drawn = this.card.drawingNow();
    if (this.leds && this.kept !== null && drawn?.length === count) this.kept.colors = [...drawn];
    const layout = { leds: this.leds, columns: draw?.grid?.[0] ?? null };
    this.zones =
      this.kept === null
        ? null
        : zonesPart(this.kept, layout, {
            begin: () => {
              this.before = this.kept === null ? null : [...this.kept.colors];
            },
            changed: () => {
              if (this.brush !== null) this.paint(this.brush.color());
            },
            end: () => {
              this.before = null;
            },
          });
    // Brightness applies to every tab, so it shows over the tabs.
    if (state.range !== null) {
      const name = t("panel.brightness");
      const { row } = slider(name, name, state.range, state.brightness, (level) => {
        this.send({ brightness: level });
      });
      this.controls.append(row);
    }
    const panes = this.panes(state);
    this.controls.append(tabs(panes, drawn === null ? "color" : "draw", t("panel.tabs")));
    this.on = state.on === true;
  }

  /** The color of the whole light, then the paint of the zones when the light has zones and a color. */
  private panes(state: Status): Tab[] {
    const { color } = state.features;
    const split = this.zones !== null && color;
    const panes: Tab[] = [];
    const solid = element("div", "panel-pane");
    const white = this.white(state);
    if (color) solid.append(new ColorPart(start(state), this.paintAll, this.fail, white, !split).node);
    else if (white !== null) solid.append(white.row);
    if (solid.childElementCount > 0)
      panes.push({
        id: "color",
        icon: PALETTE,
        label: t("panel.tab.color"),
        node: solid,
      });
    if (!split || this.zones === null || this.kept === null) return panes;
    const pane = element("div", "panel-pane");
    this.brush = new ColorPart(this.base, this.paint, this.fail, null, false, DRAW_BASE);
    pane.append(this.brush.node, this.zones.node);
    if (!this.leds && state.features.gradient) pane.append(this.gradient(this.kept).row);
    const label = t(this.leds ? "panel.tab.draw" : "panel.tab.zones");
    panes.push({ id: "draw", icon: BRUSH, label, node: pane });
    return panes;
  }

  show(): void {
    const panel = sheet({
      head: head(this.card),
      end: [this.power(this.on)],
      label: this.card.name,
    });
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

  private readonly paintAll = (hex: string): void => {
    this.send({ color: hex });
  };

  /** Paint the zones selected with `hex`. With no zone selected, `hex` only becomes the brush. A drawing sends every LED: the others would go black. */
  private readonly paint = (hex: string): void => {
    const kept = this.kept;
    if (kept === null) return;
    const picked = this.zones?.selected() ?? [];
    if (picked.length === 0) return;
    if (this.leds) {
      const chosen = new Set(picked);
      this.send({
        draw: (this.before ?? kept.colors).map((color, cell) => (chosen.has(cell) ? hex : (color ?? this.base))),
      });
    } else
      this.send({
        segment: { zones: picked, color: hex },
        gradient: kept.gradient,
      });
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
    if (change.color !== undefined || change.kelvin !== undefined) this.card.drawn(null);
    if (change.draw !== undefined) {
      this.card.drawn(change.draw);
      if (this.kept !== null) this.kept.colors = [...change.draw];
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
