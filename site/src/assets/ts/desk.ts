// The fader desk of the DMX pages, with the slot rules of `docs/dmx.md`:
// - dimmer 0 is off, 1 to 255 is the brightness.
// - white 0 sends no white, so the color shows. White 1 to 255 is a white
//   temperature that replaces the color.
// - the mode channel changes nothing that a lamp shows.

// A typical white range. Each model declares its own in its device file.
const KELVIN: [number, number] = [2000, 9000];

const clamp = (v: number): number => Math.min(255, Math.max(0, v));

/** An approximate sRGB color for a white temperature (Tanner Helland's fit). */
function kelvinRgb(kelvin: number): number[] {
  const t = kelvin / 100;
  const r = t <= 66 ? 255 : clamp(329.7 * (t - 60) ** -0.1332);
  const g = t <= 66 ? clamp(99.47 * Math.log(t) - 161.12) : clamp(288.12 * (t - 60) ** -0.0755);
  const b = t >= 66 ? 255 : clamp(138.52 * Math.log(t - 10) - 305.04);
  return [r, g, b];
}

export function desk(root: HTMLElement): void {
  const inputs = [...root.querySelectorAll<HTMLInputElement>("input[data-channel]")];
  const level = (key: string): number => Number(inputs.find((i) => i.dataset.channel === key)?.value ?? 0);

  const paint = () => {
    const dim = level("dim") / 255;
    const white = level("white");
    const [low, high] = KELVIN;
    const base = white > 0
      ? kelvinRgb(low + ((white - 1) * (high - low)) / 254)
      : [level("r"), level("g"), level("b")];
    const [r, g, b] = base.map((v) => Math.round(v * dim));
    root.style.setProperty("--lamp", `rgb(${r} ${g} ${b})`);
    root.style.setProperty("--lamp-glow", String(dim));
  };

  for (const input of inputs) {
    const fader = input.closest<HTMLElement>(".fader");
    const out = fader?.querySelector(".value");
    const show = () => {
      fader?.style.setProperty("--level", String(Number(input.value) / 255));
      if (out) out.textContent = input.value;
    };
    input.addEventListener("input", () => {
      show();
      paint();
    });
    show();
  }
  paint();
}
