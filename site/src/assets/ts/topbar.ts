// The state of the top bar on the home page. There the hero draws the beam
// and the bar sits over it, so the bar starts bare: no ground and no line.
// Both arrive as soon as the reader leaves the top of the page.

// How far the reader must scroll, in pixels.
const DEPTH = 70;

export function topbar(el: HTMLElement): void {
  if (!document.querySelector(".hero")) return;

  // `scrollY` is read straight in the handler: it costs no layout, and a
  // frame in flight would leave the bar wrong while frames are throttled.
  const read = () => el.toggleAttribute("data-past-top", scrollY > DEPTH);
  addEventListener("scroll", read, { passive: true });
  read();
}
