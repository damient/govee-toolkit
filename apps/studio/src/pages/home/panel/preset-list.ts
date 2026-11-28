import { type Preset, onPresets, presets } from "../../../core/presets.ts";
import { type PresetLight, fits, snapshot } from "./presets.ts";
import { BOOKMARK, EDIT, TRASH } from "../../../components/icon/icon.ts";
import type { Tab } from "./tabs.ts";
import { element } from "../../../core/dom.ts";
import { menu } from "../../../components/menu/menu.ts";
import { t } from "../../../core/i18n.ts";

/** What a row of the list does with its preset. */
export interface PresetActions {
  apply(preset: Preset): void;
  edit(preset: Preset): void;
  remove(preset: Preset): void;
}

function row(preset: Preset, columns: number | null, actions: PresetActions): HTMLElement {
  const wrap = element("div", "panel-preset");
  const play = element("button", "panel-preset-play");
  play.type = "button";
  play.append(snapshot(preset, columns), element("span", "panel-preset-title", preset.name));
  play.addEventListener("click", () => {
    actions.apply(preset);
  });
  const more = menu({
    label: t("panel.preset.menu", { name: preset.name }),
    items: [
      {
        text: t("panel.preset.edit"),
        icon: EDIT,
        run: () => {
          actions.edit(preset);
        },
      },
      {
        text: t("panel.preset.delete"),
        icon: TRASH,
        tone: "danger",
        run: () => {
          actions.remove(preset);
        },
      },
    ],
  });
  wrap.append(play, more);
  return wrap;
}

/** The tab of the presets that the light can apply. `shown` gets `true` while the tab has a preset. */
export function presetsTab(light: PresetLight, actions: PresetActions, shown: (on: boolean) => void): Tab {
  const node = element("div", "panel-pane panel-presets");
  let count = 0;
  const draw = (): void => {
    const list = presets().filter((preset) => fits(preset, light));
    node.replaceChildren(...list.map((preset) => row(preset, light.columns, actions)));
    count = list.length;
    shown(count > 0);
  };
  const stop = onPresets(() => {
    if (node.isConnected) draw();
    else stop();
  });
  draw();
  return { id: "presets", icon: BOOKMARK, label: t("panel.tab.presets"), node, hidden: count === 0 };
}
