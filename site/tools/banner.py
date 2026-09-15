#!/usr/bin/env python3
"""Build the README banner.

The banner is `tools/banner.svg` and `docs/assets/banner.png`, at the root of
the repository. The text is converted to paths from the woff2 files in
`src/assets/fonts/`, so no font must be installed on the machine that runs this
script.

Requirements: `pip install fonttools brotli`, and `rsvg-convert`
(`brew install librsvg`) for the PNG.

Usage: `python3 tools/banner.py`
"""

from pathlib import Path

from fontTools.pens.boundsPen import BoundsPen

from og import ACCENTS, BACKGROUND, SITE, dot_centers, draw, mono, rasterize

WIDTH, HEIGHT = 1280, 360

WORDMARK = "GOVEE TOOLKIT"
WORDMARK_SIZE = 64
# The letter spacing of the site wordmark, in em.
WORDMARK_TRACK = 0.08

# The wordmark and the dot row are one block, centered on the card. The block
# is the cap height and the dots, so the baselines come out of `banner()`.
GAP = 72

# The dot row, under the wordmark. The halo is a radial gradient, in radius
# units: a blur filter renders differently from one rasterizer to the next.
DOT_RADIUS = 16
DOT_GAP = 30
HALO_RADIUS = 3.2
# The falloff of the halo, as (offset, opacity) pairs on that radius.
HALO_STOPS = ((0.0, 0.55), (0.32, 0.2), (1.0, 0.0))

ALT = "Govee Toolkit"


def ink_center(font, text, size, track):
    """Return the center of the ink of `text`, in pixels from the pen origin.

    The side bearings of the first and the last letter are not equal, so a pen
    centered on the advance puts the ink off center.
    """
    glyphs = font.getGlyphSet()
    cmap = font.getBestCmap()
    units = font["head"].unitsPerEm
    pen_x = 0.0
    left, right = None, None
    for ch in text:
        bounds = BoundsPen(glyphs)
        glyph = glyphs[cmap[ord(ch)]]
        glyph.draw(bounds)
        if bounds.bounds:
            x0, _, x1, _ = bounds.bounds
            left = pen_x + x0 if left is None else left
            right = pen_x + x1
        pen_x += glyph.width + track * units
    return (left + right) / 2 * size / units


def banner():
    """Return the SVG source of the banner."""
    out = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}"'
        f' viewBox="0 0 {WIDTH} {HEIGHT}">',
        f"  <title>{ALT}</title>",
        f'  <rect width="{WIDTH}" height="{HEIGHT}" fill="{BACKGROUND}"/>',
    ]

    face = mono(600)
    cap = WORDMARK_SIZE * face["OS/2"].sCapHeight / face["head"].unitsPerEm
    top = (HEIGHT - (cap + GAP + 2 * DOT_RADIUS)) / 2
    wordmark_baseline = top + cap
    dot_baseline = wordmark_baseline + GAP + DOT_RADIUS

    left = WIDTH / 2 - ink_center(face, WORDMARK, WORDMARK_SIZE, WORDMARK_TRACK)
    out += draw(
        face,
        WORDMARK,
        WORDMARK_SIZE,
        left,
        wordmark_baseline,
        "#eae8e5",
        track=WORDMARK_TRACK,
    )

    out.append("  <defs>")
    for i, color in enumerate(ACCENTS):
        out.append(f'    <radialGradient id="halo{i}">')
        out += [
            f'      <stop offset="{offset:g}" stop-color="{color}"'
            f' stop-opacity="{opacity:g}"/>'
            for offset, opacity in HALO_STOPS
        ]
        out.append("    </radialGradient>")
    out.append("  </defs>")

    centers = dot_centers(WIDTH, len(ACCENTS), DOT_RADIUS, DOT_GAP)
    for i, x in enumerate(centers):
        out.append(
            f'  <circle cx="{x:g}" cy="{dot_baseline:g}"'
            f' r="{DOT_RADIUS * HALO_RADIUS:g}" fill="url(#halo{i})"/>'
        )
    for x, color in zip(centers, ACCENTS, strict=True):
        out.append(
            f'  <circle cx="{x:g}" cy="{dot_baseline:g}"'
            f' r="{DOT_RADIUS}" fill="{color}"/>'
        )

    out.append("</svg>")
    return "\n".join(out) + "\n"


def main():
    svg = SITE / "tools" / "banner.svg"
    svg.write_text(banner())
    print(f"wrote {svg.relative_to(SITE)}")
    png = Path(SITE).parent / "docs" / "assets" / "banner.png"
    png.parent.mkdir(parents=True, exist_ok=True)
    rasterize(svg, png, WIDTH, HEIGHT, BACKGROUND)


if __name__ == "__main__":
    main()
