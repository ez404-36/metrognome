#!/usr/bin/env python3
"""Add the rounded GNOME-style window frame and drop shadow to a screenshot.

Usage:

    python3 scripts/frame-screenshot.py RAW_SCREENSHOT.png main-window.png

The source must be the flat GTK render (without compositor margins). The output
keeps a transparent background so the shadow looks correct in light and dark
README themes.
"""

from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter


RADIUS = 24
BORDER = 2
MARGIN_X = 72
MARGIN_TOP = 64
MARGIN_BOTTOM = 84
SHEET_COLOR = (250, 250, 250, 255)
BORDER_COLOR = (222, 220, 217, 255)
SHADOW_COLOR = (17, 17, 20, 255)
SHADOW_OPACITY = 68


def framed_image(source: Image.Image) -> Image.Image:
    width, height = source.size
    canvas_size = (
        width + MARGIN_X * 2,
        height + MARGIN_TOP + MARGIN_BOTTOM,
    )
    sheet_rect = (
        MARGIN_X,
        MARGIN_TOP,
        MARGIN_X + width,
        MARGIN_TOP + height,
    )

    shadow = Image.new("L", canvas_size, 0)
    shadow_draw = ImageDraw.Draw(shadow)
    shadow_draw.rounded_rectangle(
        (
            sheet_rect[0] + 10,
            sheet_rect[1] + 16,
            sheet_rect[2] + 2,
            sheet_rect[3] + 18,
        ),
        radius=RADIUS + 8,
        fill=SHADOW_OPACITY,
    )
    shadow = shadow.filter(ImageFilter.GaussianBlur(22))

    canvas = Image.new("RGBA", canvas_size, (0, 0, 0, 0))
    shadow_layer = Image.new("RGBA", canvas_size, SHADOW_COLOR)
    canvas.paste(shadow_layer, (0, 0), shadow)

    interior_mask = Image.new("L", source.size, 0)
    ImageDraw.Draw(interior_mask).rounded_rectangle(
        (BORDER, BORDER, width - BORDER, height - BORDER),
        radius=RADIUS - BORDER,
        fill=255,
    )

    window = Image.new("RGBA", source.size, (0, 0, 0, 0))
    ImageDraw.Draw(window).rounded_rectangle(
        (0, 0, width, height),
        radius=RADIUS,
        fill=SHEET_COLOR,
        outline=BORDER_COLOR,
        width=BORDER,
    )
    window.paste(source, (BORDER, BORDER), interior_mask)
    canvas.alpha_composite(window, (MARGIN_X, MARGIN_TOP))
    return canvas


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path, help="flat screenshot without compositor margins")
    parser.add_argument("output", type=Path, help="framed PNG to write")
    arguments = parser.parse_args()

    source = Image.open(arguments.input).convert("RGBA")
    framed_image(source).save(arguments.output, optimize=True)


if __name__ == "__main__":
    main()
