import { help } from "../../../components/help/help.ts";
import { t } from "../../../core/i18n.ts";

/** What the panel keeps for a light while Studio is open. A reload drops it. */
export interface Memory {
  /** The color of each zone that Studio painted, or `null`. */
  colors: (string | null)[];
  gradient: boolean;
}

const memories = new Map<string, Memory>();

export function memory(id: string, count: number): Memory {
  let kept = memories.get(id);
  if (kept?.colors.length !== count) {
    kept = { colors: Array.from({ length: count }, () => null), gradient: kept?.gradient ?? false };
    memories.set(id, kept);
  }
  return kept;
}

export interface ZonesPart {
  node: HTMLElement;
  /** The zones selected, zero-based. Empty: a pick paints the whole light. */
  selected(): number[];
  /** Show the colors of `kept`. */
  draw(): void;
}

function textButton(text: string, click: () => void): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "button xs";
  button.textContent = text;
  button.addEventListener("click", click);
  return button;
}

export function zonesPart(kept: Memory): ZonesPart {
  const picked = new Set<number>();
  const node = document.createElement("div");
  node.className = "panel-zones";
  const head = document.createElement("div");
  head.className = "panel-row";
  const label = document.createElement("h3");
  label.className = "panel-label";
  label.textContent = t("panel.zones");
  const strip = document.createElement("div");
  strip.className = "panel-strip";
  strip.style.setProperty("--zones", String(kept.colors.length));

  const cells = kept.colors.map((_, zone) => {
    const cell = document.createElement("button");
    cell.type = "button";
    cell.className = "panel-zone";
    cell.setAttribute("aria-label", t("panel.zone", { n: String(zone + 1) }));
    cell.addEventListener("click", () => {
      if (picked.has(zone)) picked.delete(zone);
      else picked.add(zone);
      draw();
    });
    return cell;
  });
  const draw = (): void => {
    cells.forEach((cell, zone) => {
      cell.style.background = kept.colors[zone] ?? "";
      cell.setAttribute("aria-pressed", String(picked.has(zone)));
    });
  };
  const all = textButton(t("panel.zones.all"), () => {
    for (let zone = 0; zone < cells.length; zone += 1) picked.add(zone);
    draw();
  });
  const none = textButton(t("panel.zones.none"), () => {
    picked.clear();
    draw();
  });
  const title = document.createElement("span");
  title.className = "panel-title";
  title.append(label, help(t("panel.zones.help")));
  head.append(title, all, none);
  strip.append(...cells);
  node.append(head, strip);
  draw();
  return { node, selected: () => kept.colors.map((_, zone) => zone).filter((zone) => picked.has(zone)), draw };
}
