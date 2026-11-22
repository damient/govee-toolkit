import { t } from "./i18n.ts";

export interface Loading {
  /** `m` is the default. `block` makes it `l`. */
  size?: "s" | "m" | "l";
  /** A text beside the dots, which a screen reader also reads. */
  text?: string;
  /** A card in the center of the free space, for a view that waits for its content. */
  block?: boolean;
}

/** The three dots of the logo, in a wave, for a wait. */
export function loading(options: Loading = {}): HTMLElement {
  const box = document.createElement("span");
  box.className = `loading ${options.size ?? (options.block === true ? "l" : "m")}`;
  box.setAttribute("role", "status");
  const dots = document.createElement("span");
  dots.className = "dots wave";
  dots.setAttribute("aria-hidden", "true");
  dots.append(document.createElement("i"), document.createElement("i"), document.createElement("i"));
  box.append(dots);
  if (options.text === undefined) box.setAttribute("aria-label", t("loading.label"));
  else {
    const text = document.createElement("span");
    text.className = "loading-text";
    text.textContent = options.text;
    box.append(text);
  }
  if (options.block !== true) return box;
  const card = document.createElement("div");
  card.className = "loading-card";
  card.append(box);
  const fill = document.createElement("div");
  fill.className = "fill";
  fill.append(card);
  return fill;
}
