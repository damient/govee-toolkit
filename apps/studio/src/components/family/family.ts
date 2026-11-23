declare const STUDIO_FAMILY_ICONS: Record<string, string>;

const SVG = "http://www.w3.org/2000/svg";

/** The shapes are build-time data, not text from the network. */
export function familyIcon(family: string | null): SVGSVGElement {
  const inner = (family === null ? undefined : STUDIO_FAMILY_ICONS[family]) ?? STUDIO_FAMILY_ICONS["other"] ?? "";
  const source = `<svg xmlns="${SVG}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${inner}</svg>`;
  const root = new DOMParser().parseFromString(source, "image/svg+xml").documentElement;
  if (root instanceof SVGSVGElement) return root;
  return document.createElementNS(SVG, "svg");
}
