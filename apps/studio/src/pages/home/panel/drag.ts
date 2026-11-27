/** What a drag over the cells tells the panel. */
export interface DragEvents {
  /** A press starts a drag. */
  begin(): void;
  /** The selection changed. Outside a drag, a key press or a button changed it. */
  changed(): void;
  /** The drag ends. */
  end(): void;
}

/** The cells from `from` to `to`: a range on a strip, a rectangle on a grid of `columns`. */
function span(from: number, to: number, columns: number | null, count: number): number[] {
  if (columns === null) {
    const [low, high] = from < to ? [from, to] : [to, from];
    return Array.from({ length: high - low + 1 }, (_, index) => low + index);
  }
  const [rowA, rowB] = [Math.floor(from / columns), Math.floor(to / columns)];
  const [colA, colB] = [from % columns, to % columns];
  const cells: number[] = [];
  for (let row = Math.min(rowA, rowB); row <= Math.max(rowA, rowB); row += 1) {
    for (let col = Math.min(colA, colB); col <= Math.max(colA, colB); col += 1) {
      const cell = row * columns + col;
      if (cell < count) cells.push(cell);
    }
  }
  return cells;
}

/** Set `picked` to `base`, with `cells` added, or taken out when `adding` is `false`. */
function reset(picked: Set<number>, base: Set<number>, cells: number[], adding: boolean): void {
  picked.clear();
  for (const cell of base) picked.add(cell);
  for (const cell of cells) {
    if (adding) picked.add(cell);
    else picked.delete(cell);
  }
}

/** Shift, Ctrl or Cmd: keep the selection. */
export const held = (event: MouseEvent | KeyboardEvent): boolean => event.shiftKey || event.ctrlKey || event.metaKey;

/** The index of the cell under the pointer, or `-1`. A touch drag keeps its first target, so read the point. */
function cellAt(cells: HTMLElement[], event: PointerEvent): number {
  const target = document.elementFromPoint(event.clientX, event.clientY);
  return target instanceof HTMLElement ? cells.indexOf(target) : -1;
}

/**
 * A press starts a new selection, and a drag selects the cells from the press to the pointer: a range on
 * a strip, a rectangle on a grid. With Shift, Ctrl or Cmd, the press keeps the selection, and the drag
 * adds the span, or takes it out when the pressed cell was selected. The selection ends with the drag,
 * after the paint, unless the press held one of those keys.
 */
export function dragSelect(
  area: HTMLElement,
  cells: HTMLElement[],
  picked: Set<number>,
  columns: number | null,
  draw: () => void,
  events: DragEvents,
): void {
  let anchor = -1;
  let last = -1;
  let adding = true;
  let keep = false;
  let base = new Set<number>();
  const at = (event: PointerEvent): number => cellAt(cells, event);
  const reach = (zone: number): void => {
    if (zone < 0 || anchor < 0 || zone === last) return;
    last = zone;
    reset(picked, base, span(anchor, zone, columns, cells.length), adding);
    draw();
    events.changed();
  };
  area.addEventListener("pointerdown", (event) => {
    const zone = at(event);
    if (zone < 0) return;
    event.preventDefault();
    keep = held(event);
    base = keep ? new Set(picked) : new Set();
    adding = !keep || !picked.has(zone);
    anchor = zone;
    last = -1;
    events.begin();
    reach(zone);
  });
  area.addEventListener("pointermove", (event) => {
    reach(at(event));
  });
  const stop = (): void => {
    if (anchor < 0) return;
    anchor = -1;
    if (!keep) {
      picked.clear();
      draw();
    }
    events.end();
  };
  area.addEventListener("pointerup", stop);
  area.addEventListener("pointercancel", stop);
  area.addEventListener("pointerleave", stop);
}

/** The release of the last of Shift, Ctrl and Cmd ends the selection. The watch stops once `node` leaves the page. */
export function clearOnRelease(node: HTMLElement, picked: Set<number>, draw: () => void): void {
  const release = (event: KeyboardEvent): void => {
    if (!node.isConnected) {
      document.removeEventListener("keyup", release);
      return;
    }
    if (held(event) || picked.size === 0) return;
    picked.clear();
    draw();
  };
  document.addEventListener("keyup", release);
}
