#!/usr/bin/env python3
"""Build the Open Graph card and the touch icon.

The card is `tools/og.svg` and `src/assets/og.png`. The text is converted to
paths from the woff2 files in `src/assets/fonts/`, so no font must be installed
on the machine that runs this script.

Requirements: `pip install fonttools brotli`, and `rsvg-convert`
(`brew install librsvg`) for the PNG.

Usage: `python3 tools/og.py`
"""

import shutil
import subprocess
import sys
from pathlib import Path

from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.misc.transform import Transform
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

SITE = Path(__file__).resolve().parent.parent
FONTS = SITE / "src" / "assets" / "fonts"

WIDTH, HEIGHT = 1200, 630
LEFT = 88
BACKGROUND = "#0a0d14"

# The three accent colors of the footer dots, in the order the footer uses.
ACCENTS = ("#ff3d00", "#00c2b5", "#7c5cff")

EYEBROW = "GOVEE TOOLKIT"
HEADLINE = ("Control your Govee lights", "from your own machine.")
# Every baseline, so that the margin over the eyebrow and the margin under the
# dots stay equal.
EYEBROW_BASELINE = 138
HEADLINE_BASELINE = 264
HEADLINE_STEP = 88

# The dot row, centered on the card.
DOT_RADIUS = 16
DOT_GAP = 24
DOT_BASELINE = 497

ALT = "govee-toolkit — control your Govee lights from your own machine"


def sans(weight):
    """Return IBM Plex Sans at one weight of its variable axis."""
    font = TTFont(FONTS / "ibm-plex-sans-latin.woff2")
    return instancer.instantiateVariableFont(font, {"wght": weight}, inplace=False)


def mono(weight):
    """Return IBM Plex Mono at one of the three weights the site ships."""
    return TTFont(FONTS / f"ibm-plex-mono-{weight}-latin.woff2")


def advance(font, text, track):
    """Return the pen advance of `text`, in pixels, at a size of 1 unit per em."""
    glyphs = font.getGlyphSet()
    cmap = font.getBestCmap()
    width = sum(glyphs[cmap[ord(ch)]].width for ch in text)
    return width + track * (len(text) - 1) * font["head"].unitsPerEm


def draw(font, text, size, x, baseline, fill, track=0.0, align="left"):
    """Return one `<path>` per glyph of `text`, with the text on its baseline.

    `track` is the letter spacing, in em. `align` is `left` for a pen that
    starts at `x`, or `right` for a pen that ends at `x`.
    """
    glyphs = font.getGlyphSet()
    cmap = font.getBestCmap()
    scale = size / font["head"].unitsPerEm
    if align == "right":
        x -= advance(font, text, track) * scale

    out = []
    pen_x = 0.0
    for ch in text:
        name = cmap[ord(ch)]
        path = SVGPathPen(glyphs, ntos=lambda v: f"{v:.2f}")
        transform = Transform(scale, 0, 0, -scale, x + pen_x * scale, baseline)
        glyphs[name].draw(TransformPen(path, transform))
        commands = path.getCommands()
        if commands:
            out.append(f'  <path fill="{fill}" d="{commands}"/>')
        pen_x += glyphs[name].width + track * font["head"].unitsPerEm
    return out


def card():
    """Return the SVG source of the card."""
    out = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}"'
        f' viewBox="0 0 {WIDTH} {HEIGHT}">',
        f"  <title>{ALT}</title>",
        f'  <rect width="{WIDTH}" height="{HEIGHT}" fill="{BACKGROUND}"/>',
    ]

    out += draw(mono(500), EYEBROW, 30, LEFT, EYEBROW_BASELINE, "#9694a0", track=0.1333)
    for i, line in enumerate(HEADLINE):
        color = "#eae8e5" if i == 0 else ACCENTS[0]
        y = HEADLINE_BASELINE + i * HEADLINE_STEP
        out += draw(sans(600), line, 74, LEFT, y, color, track=-0.0188)

    step = 2 * DOT_RADIUS + DOT_GAP
    first = WIDTH / 2 - step * (len(ACCENTS) - 1) / 2
    for i, color in enumerate(ACCENTS):
        out.append(
            f'  <circle cx="{first + i * step:g}" cy="{DOT_BASELINE}"'
            f' r="{DOT_RADIUS}" fill="{color}"/>'
        )

    out.append("</svg>")
    return "\n".join(out) + "\n"


def rasterize(source, target, width, height, background=None):
    """Convert one SVG file to a PNG file with `rsvg-convert`."""
    if shutil.which("rsvg-convert") is None:
        sys.exit("rsvg-convert is missing: brew install librsvg")
    command = ["rsvg-convert", "-w", str(width), "-h", str(height)]
    if background:
        command += ["-b", background]
    command += ["-f", "png", "-o", str(target), str(source)]
    subprocess.run(command, check=True)
    print(f"wrote {target.relative_to(SITE)}")


def touch_icon():
    """Render the 180px touch icon from the favicon.

    The corners are square: iOS applies its own mask, and a rounded source
    rounds them twice.
    """
    assets = SITE / "src" / "assets"
    square = assets / ".touch-icon.svg"
    square.write_text((assets / "favicon.svg").read_text().replace(' rx="7"', ""))
    try:
        rasterize(square, assets / "apple-touch-icon.png", 180, 180)
    finally:
        square.unlink()


def main():
    svg = SITE / "tools" / "og.svg"
    svg.write_text(card())
    print(f"wrote {svg.relative_to(SITE)}")
    rasterize(svg, SITE / "src" / "assets" / "og.png", WIDTH, HEIGHT, BACKGROUND)
    touch_icon()


if __name__ == "__main__":
    main()
