import { help } from "../../../components/help/help.ts";
import { t } from "../../../core/i18n.ts";
import { type DragEvents, clearOnRelease, dragSelect, held } from "./drag.ts";

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
    kept = {
      colors: Array.from({ length: count }, () => null),
      gradient: kept?.gradient ?? false,
    };
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

/** The zones of `segment`, or the LEDs of `draw`. */
export interface Layout {
  /** `true`: each cell is one LED of a drawing. */
  leds: boolean;
  /** The column count of a grid, row 0 at the top. `null` shows one row. */
  columns: number | null;
}

function textButton(text: string, click: (event: MouseEvent) => void): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "button xs";
  button.textContent = text;
  button.addEventListener("click", click);
  return button;
}

/** The narrowest cell of a strip, in CSS pixels. Under it, the strip goes on to the next row. */
const CELL_MIN = 24;

/** A computed length such as `10px`. `0` for `normal`. */
const px = (value: string): number => Number(value.replace("px", "")) || 0;

/**
 * Fold a strip that does not fit into rows, as the strip folds back: each second row runs from right
 * to left. The last cell of a row carries the `turn` mark, which draws the link to the row under it.
 */
function snake(area: HTMLElement, cells: HTMLElement[]): void {
  const fold = (): void => {
    const style = getComputedStyle(area);
    const width = area.clientWidth - px(style.paddingLeft) - px(style.paddingRight);
    const gap = px(style.columnGap);
    if (width <= 0) return;
    const columns = Math.max(1, Math.min(cells.length, Math.floor((width + gap) / (CELL_MIN + gap))));
    const rows = Math.ceil(cells.length / columns);
    area.style.setProperty("--zones", String(columns));
    area.classList.toggle("folded", rows > 1);
    cells.forEach((cell, index) => {
      const row = Math.floor(index / columns);
      const place = index % columns;
      const back = row % 2 === 1;
      cell.style.gridRow = String(row + 1);
      cell.style.gridColumn = String(back ? columns - place : place + 1);
      const turn = place === columns - 1 && row < rows - 1;
      cell.classList.toggle("turn", turn);
      cell.classList.toggle("left", turn && back);
    });
  };
  new ResizeObserver(fold).observe(area);
}

/** A pointer selects in `dragSelect`. `toggle` runs for a key press, which has no pointer and arrives with `detail` 0. */
function cellButton(name: string, toggle: (event: MouseEvent) => void): HTMLButtonElement {
  const cell = document.createElement("button");
  cell.type = "button";
  cell.className = "panel-zone";
  cell.setAttribute("aria-label", name);
  cell.addEventListener("click", (event) => {
    if (event.detail === 0) toggle(event);
  });
  return cell;
}

function headRow(leds: boolean, ...buttons: HTMLButtonElement[]): HTMLElement {
  const head = document.createElement("div");
  head.className = "panel-row";
  const label = document.createElement("h3");
  label.className = "panel-label";
  label.textContent = t(leds ? "panel.cells" : "panel.zones");
  const title = document.createElement("span");
  title.className = "panel-title";
  title.append(label, help(t(leds ? "panel.cells.help" : "panel.zones.help")));
  head.append(title, ...buttons);
  return head;
}

export function zonesPart(kept: Memory, layout: Layout, events: DragEvents): ZonesPart {
  const picked = new Set<number>();
  const node = document.createElement("div");
  node.className = "panel-zones";
  const area = document.createElement("div");
  area.className = layout.columns === null ? "panel-strip" : "panel-strip panel-grid";
  area.style.setProperty("--zones", String(layout.columns ?? kept.colors.length));
  const cells = kept.colors.map((_, zone) =>
    cellButton(t(layout.leds ? "panel.cell" : "panel.zone", { n: String(zone + 1) }), (event) => {
      if (!held(event)) picked.clear();
      if (picked.has(zone)) picked.delete(zone);
      else picked.add(zone);
      paint(event);
    }),
  );
  const draw = (): void => {
    cells.forEach((cell, zone) => {
      cell.style.background = kept.colors[zone] ?? "";
      cell.setAttribute("aria-pressed", String(picked.has(zone)));
    });
  };
  /** Paint the selection, then end it unless Shift, Ctrl or Cmd is held. */
  const paint = (event: MouseEvent): void => {
    draw();
    events.changed();
    if (held(event)) return;
    picked.clear();
    draw();
  };
  dragSelect(area, cells, picked, layout.columns, draw, events);
  clearOnRelease(node, picked, draw);
  const all = textButton(t("panel.zones.all"), (event) => {
    for (let zone = 0; zone < cells.length; zone += 1) picked.add(zone);
    paint(event);
  });
  const none = textButton(t("panel.zones.none"), () => {
    picked.clear();
    draw();
  });
  area.append(...cells);
  if (layout.columns === null) snake(area, cells);
  node.append(headRow(layout.leds, all, none), area);
  draw();
  return {
    node,
    selected: () => kept.colors.map((_, zone) => zone).filter((zone) => picked.has(zone)),
    draw,
  };
}
