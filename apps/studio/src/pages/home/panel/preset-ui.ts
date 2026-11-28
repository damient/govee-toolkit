import { type Preset, addPreset, deletePreset, updatePreset } from "../../../core/presets.ts";
import { type Look, type PresetLight, playPreset } from "./presets.ts";
import { BOOKMARK, icon } from "../../../components/icon/icon.ts";
import { askName, confirmDelete } from "./preset-form.ts";
import { explain, t } from "../../../core/i18n.ts";
import type { Change } from "../../../core/control.ts";
import { type ColorPart, type White, WHITE } from "./color.ts";
import type { Slider } from "./fields.ts";
import type { Tab, Tabs } from "./tabs.ts";
import { element } from "../../../core/dom.ts";
import { presetsTab } from "./preset-list.ts";


/** The parts of the panel that an edit sets to the values of a preset. The panel fills them as it builds them. */
export interface PresetView {
  color: ColorPart | null;
  white: White | null;
  /** The brightness sliders of the tabs. */
  levels: Slider[];
  tabs: Tabs | null;
}

/**
 * The presets of one light: the tab, the button under each tab, and the edit
 * of a preset. An edit applies the preset, opens its tab with its values, and
 * makes the buttons save into the preset.
 */
export class PresetsUi {
  readonly view: PresetView = { color: null, white: null, levels: [], tabs: null };
  private readonly light: PresetLight;
  private readonly run: (change: Change) => Promise<void>;
  private readonly fail: (message: string) => void;
  private readonly brightness: () => number | null;
  private editing: Preset | null = null;
  private readonly refreshers: (() => void)[] = [];

  /** `run` sends one change and shows it. `brightness` gives the level of the light, in percent. */
  constructor(
    light: PresetLight,
    run: (change: Change) => Promise<void>,
    fail: (message: string) => void,
    brightness: () => number | null,
  ) {
    this.light = light;
    this.run = run;
    this.fail = fail;
    this.brightness = brightness;
  }

  tab(shown: (on: boolean) => void): Tab {
    return presetsTab(
      this.light,
      {
        apply: (preset) => {
          playPreset(preset, this.light, this.run, this.fail);
        },
        edit: (preset) => {
          this.edit(preset);
        },
        remove: (preset) => {
          this.remove(preset);
        },
      },
      shown,
    );
  }

  /** The buttons under the content of a tab. They save `look` and the brightness. */
  adder(look: () => Look): HTMLElement {
    const wrap = element("div", "panel-preset-actions");
    const save = element("button", "button xs");
    save.type = "button";
    const cancel = element("button", "button xs", t("modal.cancel"));
    cancel.type = "button";
    const refresh = (): void => {
      save.replaceChildren(icon(BOOKMARK), t(this.editing === null ? "panel.preset.add" : "panel.preset.update"));
      cancel.hidden = this.editing === null;
    };
    save.addEventListener("click", () => {
      this.save(look());
    });
    cancel.addEventListener("click", () => {
      this.stop();
    });
    this.refreshers.push(refresh);
    refresh();
    wrap.append(save, cancel);
    return wrap;
  }

  private save(look: Look): void {
    const editing = this.editing;
    const level = this.brightness();
    const dim = level === null ? {} : { brightness: Math.round(level) };
    // A drawing belongs to the model that shows it.
    const sku = look.cells === undefined ? (editing?.sku ?? this.light.sku) : this.light.sku;
    askName(editing?.name ?? null)
      .then(async (name) => {
        if (name === null) return;
        const preset = { name, sku, ...look, ...dim };
        if (editing === null) await addPreset(preset);
        else {
          await updatePreset(editing.id, preset);
          this.stop();
        }
      })
      .catch((failure: unknown) => {
        this.fail(explain(failure));
      });
  }

  private edit(preset: Preset): void {
    playPreset(preset, this.light, this.run, this.fail);
    const { color, white, levels, tabs } = this.view;
    tabs?.select(preset.cells === undefined ? "color" : "draw");
    if (preset.color !== undefined) color?.show(preset.color);
    if (preset.kelvin !== undefined) {
      white?.set(preset.kelvin);
      color?.show(WHITE);
    }
    const { brightness } = preset;
    if (brightness !== undefined) for (const level of levels) level.set(brightness);
    this.editing = preset;
    for (const refresh of this.refreshers) refresh();
  }

  private stop(): void {
    this.editing = null;
    for (const refresh of this.refreshers) refresh();
  }

  private remove(preset: Preset): void {
    confirmDelete(preset.name)
      .then(async (yes) => {
        if (!yes) return;
        await deletePreset(preset.id);
        if (this.editing?.id === preset.id) this.stop();
      })
      .catch((failure: unknown) => {
        this.fail(explain(failure));
      });
  }
}
