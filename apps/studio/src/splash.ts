/** The splash stays at least this long, so the wave runs once in full. */
const MIN_MS = 1400;

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

// A computed length is always in pixels: "40px".
const px = (value: string): number => Number(value.slice(0, -2));

/**
 * Size `line` so that it spans the letters of `title`, end to end. The box of
 * `title` also holds its padding and the spacing after its last letter.
 */
function fit(title: HTMLElement, line: HTMLElement): void {
  line.style.fontSize = "";
  const style = getComputedStyle(title);
  const letters = title.getBoundingClientRect().width - px(style.paddingLeft) - px(style.letterSpacing);
  const ratio = letters / line.getBoundingClientRect().width;
  line.style.fontSize = `${px(getComputedStyle(line).fontSize) * ratio}px`;
}

export interface Splash {
  /**
   * Show the splash, run `task` and hide the splash when it ends, after `ms`
   * at least. When `task` fails, the splash stays and the error goes to the
   * caller.
   */
  hold<T>(task: () => Promise<T>, ms?: number): Promise<T>;
  /** Stop the wave and show `text` as an error. */
  fail(text: string): void;
}

export function splash(element: HTMLElement, line: HTMLElement): Splash {
  const title = element.querySelector<HTMLElement>(".splash-title");
  const sub = element.querySelector<HTMLElement>(".splash-sub");
  if (title && sub) {
    const align = (): void => {
      fit(title, sub);
    };
    // The widths are right only once the faces have loaded.
    void document.fonts.ready.then(align);
    addEventListener("resize", align);
  }
  const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
  return {
    async hold<T>(task: () => Promise<T>, ms = MIN_MS): Promise<T> {
      delete element.dataset["state"];
      const shown = wait(still ? 0 : ms);
      try {
        const value = await task();
        await shown;
        element.dataset["state"] = "done";
        return value;
      } catch (error) {
        await shown;
        throw error;
      }
    },
    fail(text) {
      element.dataset["state"] = "error";
      line.textContent = text;
    },
  };
}
