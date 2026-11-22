// A drag with the pointer events, so that a mouse, a finger and a pen use
// one path. The drag and drop of HTML does not start from a touch on iOS.

/** A move shorter than this, in pixels, stays a click. */
const THRESHOLD = 5;

/** The drop target under a point: an element with `data-drop` inside `root`. */
function targetAt(root: HTMLElement, x: number, y: number): HTMLElement | null {
  const under = document.elementFromPoint(x, y);
  const target = under?.closest<HTMLElement>("[data-drop]") ?? null;
  return target !== null && root.contains(target) ? target : null;
}

function swallow(click: Event): void {
  click.stopPropagation();
  click.preventDefault();
}

/** One press on a draggable node, until the release. */
class Gesture {
  private ghost: HTMLElement | null = null;
  private over: HTMLElement | null = null;
  private readonly node: HTMLElement;
  private readonly root: HTMLElement;
  private readonly down: PointerEvent;
  private readonly box: DOMRect;
  private readonly drop: (target: HTMLElement) => void;

  constructor(node: HTMLElement, root: HTMLElement, down: PointerEvent, drop: (target: HTMLElement) => void) {
    this.node = node;
    this.root = root;
    this.down = down;
    this.drop = drop;
    this.box = node.getBoundingClientRect();
  }

  /** Follow the pointer until the release. */
  listen(): void {
    this.node.setPointerCapture(this.down.pointerId);
    this.node.addEventListener("pointermove", this.move);
    this.node.addEventListener("pointerup", this.end);
    this.node.addEventListener("pointercancel", this.end);
  }

  private hover(target: HTMLElement | null): void {
    if (target === this.over) return;
    this.over?.classList.remove("over");
    target?.classList.add("over");
    this.over = target;
  }

  /** The copy of the node that follows the pointer. `null` while the move stays under the threshold. */
  private follow(event: PointerEvent): HTMLElement | null {
    if (this.ghost !== null) return this.ghost;
    if (Math.hypot(event.clientX - this.down.clientX, event.clientY - this.down.clientY) < THRESHOLD) return null;
    const ghost = this.node.cloneNode(true);
    if (!(ghost instanceof HTMLElement)) return null;
    ghost.classList.add("drag-ghost");
    ghost.style.width = `${this.box.width}px`;
    document.body.append(ghost);
    this.node.classList.add("drag-source");
    this.root.classList.add("dragging");
    this.ghost = ghost;
    return ghost;
  }

  private readonly move = (event: PointerEvent): void => {
    const ghost = this.follow(event);
    if (ghost === null) return;
    const x = event.clientX - (this.down.clientX - this.box.left);
    const y = event.clientY - (this.down.clientY - this.box.top);
    ghost.style.transform = `translate(${x}px, ${y}px)`;
    this.hover(targetAt(this.root, event.clientX, event.clientY));
  };

  private readonly end = (event: PointerEvent): void => {
    const { node } = this;
    node.removeEventListener("pointermove", this.move);
    node.removeEventListener("pointerup", this.end);
    node.removeEventListener("pointercancel", this.end);
    if (this.ghost === null) return;
    this.ghost.remove();
    node.classList.remove("drag-source");
    this.root.classList.remove("dragging");
    const target = this.over;
    this.hover(null);
    // The click that follows the release, in the same task, is part of the drag.
    node.addEventListener("click", swallow, { capture: true });
    setTimeout(() => {
      node.removeEventListener("click", swallow, { capture: true });
    });
    if (event.type === "pointerup" && target !== null) this.drop(target);
  };
}

/**
 * Let `node` move to a drop target of `root`. `drop` gets the target under
 * the pointer at the release. The click that ends a drag does not reach
 * `node`. An element with `data-nodrag` in `node` starts no drag. `root`
 * carries `dragging` and the target under the pointer carries `over` while
 * the drag runs.
 */
export function draggable(node: HTMLElement, root: HTMLElement, drop: (target: HTMLElement) => void): void {
  node.addEventListener("pointerdown", (down) => {
    if (down.button !== 0 || !down.isPrimary) return;
    if (down.target instanceof Element && down.target.closest("[data-nodrag]")) return;
    // No text selection while the pointer moves. WebKit reads only the prefixed `user-select`.
    down.preventDefault();
    new Gesture(node, root, down, drop).listen();
  });
}
