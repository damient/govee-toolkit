const SVG = "http://www.w3.org/2000/svg";

export const ALERT = "M12 8v5M12 16h.01M10.3 3.9L2.4 18a2 2 0 0 0 1.7 3h15.8a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z";
export const ARROW = "M5 12h14M13 18l6-6M13 6l6 6";
export const BACK = "M15 6l-6 6 6 6";
export const CHECK = "M5 12l5 5L20 7";
export const BRUSH = "M10 14l8.3-8.3a2 2 0 0 1 2.8 2.8L12.8 16.8M10 14a3.5 3.5 0 0 0-3.5 3.5c0 1.4-1 2.5-2.5 2.5h5a4 4 0 0 0 3.8-3.2";
export const EDIT = "M4 20h4L18.5 9.5a2.8 2.8 0 0 0-4-4L4 16v4M13.5 6.5l4 4";
export const PALETTE = "M12 21a9 9 0 1 1 9-9c0 1.7-1.3 3-3 3h-2.2a1.8 1.8 0 0 0-1.3 3A1.8 1.8 0 0 1 12 21zM7.5 12h.01M9.5 7.5h.01M14.5 7.5h.01";
export const PLUS = "M12 5v14M5 12h14";
export const REFRESH = "M20 11A8.1 8.1 0 0 0 4.5 9M4 5v4h4M4 13a8.1 8.1 0 0 0 15.5 2m.5 4v-4h-4";
export const SUN = "M12 16a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4";
export const SEARCH = "M10.5 17.5a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM20 20l-4.5-4.5";

export function icon(d: string): SVGSVGElement {
  const svg = document.createElementNS(SVG, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "1.75");
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  svg.setAttribute("aria-hidden", "true");
  const path = document.createElementNS(SVG, "path");
  path.setAttribute("d", d);
  svg.append(path);
  return svg;
}
