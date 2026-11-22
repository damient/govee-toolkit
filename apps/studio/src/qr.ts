import { encode } from "uqr";

const SVG = "http://www.w3.org/2000/svg";
// `QrCodeDataType.Position` in uqr: a module of the three finder squares.
const POSITION = 2;
const DOT = 0.43;

function rect(x: number, y: number, size: number, radius: number): SVGRectElement {
  const element = document.createElementNS(SVG, "rect");
  for (const [name, value] of Object.entries({ x, y, width: size, height: size, rx: radius })) {
    element.setAttribute(name, String(value));
  }
  return element;
}

/**
 * Draw `text` as a QR code in the current color: round dots for the data and
 * rounded finder squares. The caller puts it on a light ground with a margin.
 */
export function qr(text: string): SVGSVGElement {
  const code = encode(text, { ecc: "M", border: 0 });
  const svg = document.createElementNS(SVG, "svg");
  svg.setAttribute("viewBox", `0 0 ${code.size} ${code.size}`);
  svg.setAttribute("fill", "currentColor");
  svg.setAttribute("role", "img");

  let d = "";
  for (const [y, row] of code.data.entries()) {
    for (const [x, dark] of row.entries()) {
      if (!dark || Number(code.types[y]?.[x]) === POSITION) continue;
      d += `M${x + 0.5 - DOT} ${y + 0.5}a${DOT} ${DOT} 0 1 0 ${2 * DOT} 0a${DOT} ${DOT} 0 1 0 ${-2 * DOT} 0`;
    }
  }
  const dots = document.createElementNS(SVG, "path");
  dots.setAttribute("d", d);
  svg.append(dots);

  const far = code.size - 7;
  for (const [x, y] of [[0, 0], [far, 0], [0, far]] as const) {
    const ring = rect(x + 0.5, y + 0.5, 6, 1.8);
    ring.setAttribute("fill", "none");
    ring.setAttribute("stroke", "currentColor");
    svg.append(ring, rect(x + 2, y + 2, 3, 0.9));
  }
  return svg;
}
