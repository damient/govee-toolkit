import "./panel.css";
import { type Change, type Status, apply, applyOne } from "../../../core/control.ts";
import { type Memory, type ZonesPart, memory, zonesPart } from "./zones.ts";
import { explain, t } from "../../../core/i18n.ts";
import { ColorPart, DRAW_BASE, type White, startColor as start } from "./color.ts";
import { head } from "./head.ts";
import { type HomeCard, powerButton } from "../card.ts";
import { latest } from "../../../core/latest.ts";
import { BRUSH, PALETTE } from "../../../components/icon/icon.ts";
import { type Tab, type Tabs, tabs } from "./tabs.ts";
import { loadPresets } from "../../../core/presets.ts";
import type { Look } from "./presets.ts";
import { PresetsUi } from "./preset-ui.ts";
import { sheet } from "../../../components/sheet/sheet.ts";
import { element } from "../../../core/dom.ts";
import { type Slider, type Switch, slider, switchRow } from "./fields.ts";

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
  /** The brightness sliders of the tabs. */
  private readonly levels: Slider[] = [];
  /** The presets of the light, or `null` with no SKU. */
  private readonly presets: PresetsUi | null;
  /** The tabs, once built. The presets tab shows when the model has a preset. */
  private bar: Tabs | null = null;
  /** `true`: the zones are the LEDs of `Change.draw`, else the zones of `Change.segment`. */
  private readonly leds: boolean;
  /** The color of a LED that Studio did not paint, for a drawing that sends every LED. */
  private readonly base: string;
  /** Shows the state of the power button. `power` sets it. */
  private showPower: (lit: boolean) => void = () => {
    // The button does not exist before `show`.
  };
  /** Send one change and show it. */
  private readonly run = async (change: Change): Promise<void> => {
    await this.change(change);
    this.took(change);
  };
  private readonly send = latest(
    this.run,
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
    this.presets = this.presetsUi(state, layout.columns);
    if (this.presets) this.presets.view.levels = this.levels;
    const panes = this.panes(state);
    // A light with no color tab shows its brightness alone.
    const alone = panes.length === 0 ? this.level(state) : null;
    if (alone !== null) this.controls.append(alone);
    if (this.presets !== null && panes.length > 0)
      panes.push(
        this.presets.tab((shown) => {
          this.bar?.show("presets", shown);
        }),
      );
    this.bar = tabs(panes, drawn === null ? "color" : "draw", t("panel.tabs"));
    if (this.presets) this.presets.view.tabs = this.bar;
    this.controls.append(this.bar.node);
    this.on = state.on === true;
  }

  /** A brightness slider of a tab. Every tab has one, and a move shows on the others. `null` when the light has no brightness. */
  private level(state: Status): HTMLElement | null {
    if (state.range === null) return null;
    const name = t("panel.brightness");
    const made = slider(name, name, state.range, state.brightness, (value) => {
      for (const other of this.levels) if (other !== made) other.set(value);
      this.send({ brightness: value });
    });
    this.levels.push(made);
    return made.row;
  }

  private presetsUi(state: Status, columns: number | null): PresetsUi | null {
    const { sku } = this.card.found;
    if (sku === null) return null;
    const light = { sku, features: state.features, range: state.range, kept: this.kept, leds: this.leds, columns };
    return new PresetsUi(light, this.run, this.fail, () => this.card.snapshot()?.brightness ?? null);
  }

  /** The color of the whole light, then the paint of the zones when the light has zones and a color. */
  private panes(state: Status): Tab[] {
    const { color } = state.features;
    const split = this.zones !== null && color;
    const panes: Tab[] = [];
    const solid = element("div", "panel-pane");
    const white = this.white(state);
    if (color) {
      const part = new ColorPart(start(state), this.paintAll, this.fail, white, !split);
      const look = (): Look => (white !== null && part.color() === "#ffffff" ? { kelvin: white.kelvin() } : { color: part.color() });
      solid.append(part.node, this.adder(look));
      if (this.presets) Object.assign(this.presets.view, { color: part, white });
    } else if (white !== null) {
      solid.append(white.row, this.adder(() => ({ kelvin: white.kelvin() })));
      if (this.presets) this.presets.view.white = white;
    }
    const dim = solid.childElementCount > 0 ? this.level(state) : null;
    if (dim !== null) solid.prepend(dim);
    if (solid.childElementCount > 0) panes.push({ id: "color", icon: PALETTE, label: t("panel.tab.color"), node: solid });
    if (!split || this.zones === null || this.kept === null) return panes;
    const pane = element("div", "panel-pane");
    this.brush = new ColorPart(this.base, this.paint, this.fail, null, false, DRAW_BASE);
    const level = this.level(state);
    if (level !== null) pane.append(level);
    pane.append(this.brush.node, this.zones.node);
    if (!this.leds && state.features.gradient) pane.append(this.gradient(this.kept).row);
    const kept = this.kept;
    const cells = (): { cells: string[] } => ({ cells: kept.colors.map((hex) => hex ?? this.base) });
    pane.append(this.adder(cells));
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
    loadPresets().catch((failure: unknown) => {
      this.fail(explain(failure));
    });
  }

  private adder(look: () => Look): HTMLElement {
    return this.presets?.adder(look) ?? element("span", "");
  }

  /** The white temperature slider, or `null` when the mode has none. The white swatch sends its value. */
  private white(state: Status): White | null {
    const bounds = state.features.colorTemp;
    if (bounds === null) return null;
    const on = state.kelvin !== null && state.kelvin > 0;
    let kelvin = on && state.kelvin !== null ? state.kelvin : Math.round((bounds[0] + bounds[1]) / 2);
    const { row, track, set } = slider(null, t("panel.white"), bounds, kelvin, (value) => {
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
      kelvin: () => kelvin,
      set: (value) => {
        kelvin = value;
        set(value);
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
