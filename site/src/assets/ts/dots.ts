// Three dots play one move, then rest for a random pause. The moves are the
// `data-move` rules in `dots.css`. A click plays a move at once.
// - `data-dots="click"` plays a move on a click only.
// - `data-dots-open` names the move that plays when the page opens.

const MOVES = ["wave", "hop", "jump", "flare", "blink", "squash"];
const SINGLE: ReadonlySet<string> = new Set(["hop", "blink"]);

// The rest between two moves, in milliseconds.
const REST_MIN = 2500;
const REST_MAX = 9000;
const OPEN_DELAY = 300;

const pick = <T>(items: readonly T[]): T | undefined => items[Math.floor(Math.random() * items.length)];

function start(el: HTMLElement, items: readonly HTMLElement[], move: string): void {
  if (SINGLE.has(move)) {
    const one = pick(items);
    if (one) one.dataset.on = "";
  }
  el.dataset.move = move;
}

export function dots(el: HTMLElement): void {
  if (matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  const items = [...el.querySelectorAll<HTMLElement>("i")];
  let last = "";
  let timer = 0;
  let playing = false;

  const end = () => {
    playing = false;
    delete el.dataset.move;
    for (const item of items) delete item.dataset.on;
    rest();
  };

  const play = (first?: string) => {
    // A hidden tab skips the move: the timers there run late and in bursts.
    if (document.hidden) {
      rest();
      return;
    }
    last = first ?? pick(MOVES.filter((m) => m !== last)) ?? "wave";
    start(el, items, last);
    playing = true;
    const runs = el.getAnimations({ subtree: true }).map((a) => a.finished);
    void Promise.allSettled(runs).then(end);
  };

  function rest(): void {
    if (el.dataset.dots === "click") return;
    timer = setTimeout(() => {
      play();
    }, REST_MIN + Math.random() * (REST_MAX - REST_MIN));
  }

  el.dataset.live = "";
  el.addEventListener("click", () => {
    if (playing) return;
    clearTimeout(timer);
    play();
  });

  // The delay lets the reader see the dots before the move.
  const open = el.dataset.dotsOpen;
  if (open === undefined) rest();
  else timer = setTimeout(() => {
    play(open);
  }, OPEN_DELAY);
}
