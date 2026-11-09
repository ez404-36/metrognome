#!/usr/bin/env python3
"""Render the Takto app icons from the brand logomark.

The generated PNGs are committed to the repository so packaging (cargo-deb,
AppImage) does not need an SVG rasterizer at build time. Re-run this script
after changing the logomark geometry:

    python3 scripts/generate-icons.py

Uses the system librsvg and cairo libraries directly, without extra Python
packages. On Debian/Ubuntu they are provided by librsvg2, libcairo2, and
libglib2.0-0.

The PNG assets are circular app icons with a white background; their corners
remain transparent.
"""

from __future__ import annotations

import ctypes
import ctypes.util
import os
from ctypes import POINTER, Structure, c_char_p, c_double, c_int, c_void_p
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
LOGO = ROOT / "assets" / "brand" / "icons" / "logo.svg"
OUT_DIR = ROOT / "assets" / "brand" / "icons"
SIZES = (64, 128, 256)
CONTENT_SCALE = 0.90

CAIRO_FORMAT_ARGB32 = 0
CAIRO_STATUS_SUCCESS = 0


class RsvgRectangle(Structure):
    _fields_ = [
        ("x", c_double),
        ("y", c_double),
        ("width", c_double),
        ("height", c_double),
    ]


def _load_library(name: str) -> ctypes.CDLL:
    path = ctypes.util.find_library(name)
    if not path:
        raise RuntimeError(f"required system library was not found: {name}")
    return ctypes.CDLL(path)


def _configure_libraries() -> tuple[ctypes.CDLL, ctypes.CDLL, ctypes.CDLL]:
    cairo = _load_library("cairo")
    rsvg = _load_library("rsvg-2")
    gobject = _load_library("gobject-2.0")

    cairo.cairo_image_surface_create.restype = c_void_p
    cairo.cairo_image_surface_create.argtypes = [c_int, c_int, c_int]
    cairo.cairo_create.restype = c_void_p
    cairo.cairo_create.argtypes = [c_void_p]
    cairo.cairo_surface_write_to_png.restype = c_int
    cairo.cairo_surface_write_to_png.argtypes = [c_void_p, c_char_p]
    cairo.cairo_set_source_rgb.restype = None
    cairo.cairo_set_source_rgb.argtypes = [c_void_p, c_double, c_double, c_double]
    cairo.cairo_arc.restype = None
    cairo.cairo_arc.argtypes = [
        c_void_p,
        c_double,
        c_double,
        c_double,
        c_double,
        c_double,
    ]
    cairo.cairo_fill.restype = None
    cairo.cairo_fill.argtypes = [c_void_p]
    cairo.cairo_destroy.argtypes = [c_void_p]
    cairo.cairo_surface_destroy.argtypes = [c_void_p]

    rsvg.rsvg_handle_new_from_file.restype = c_void_p
    rsvg.rsvg_handle_new_from_file.argtypes = [c_char_p, POINTER(c_void_p)]
    rsvg.rsvg_handle_render_document.restype = c_int
    rsvg.rsvg_handle_render_document.argtypes = [
        c_void_p,
        c_void_p,
        POINTER(RsvgRectangle),
        POINTER(c_void_p),
    ]

    gobject.g_object_unref.argtypes = [c_void_p]
    return cairo, rsvg, gobject


def render(size: int) -> None:
    cairo, rsvg, gobject = _configure_libraries()

    error = c_void_p()
    handle = rsvg.rsvg_handle_new_from_file(os.fsencode(LOGO), ctypes.byref(error))
    if not handle:
        raise RuntimeError(f"librsvg could not load {LOGO}")

    surface = cairo.cairo_image_surface_create(CAIRO_FORMAT_ARGB32, size, size)
    context = cairo.cairo_create(surface)
    # Keep the artwork clear of the circular edge instead of letting its
    # rightmost toe and stick tips touch the white boundary.
    inner_size = size * CONTENT_SCALE
    margin = (size - inner_size) / 2
    viewport = RsvgRectangle(
        x=margin,
        y=margin,
        width=inner_size,
        height=inner_size,
    )

    try:
        # Keep the square PNG corners transparent while giving the circular
        # icon an opaque white surface.
        cairo.cairo_set_source_rgb(context, 1.0, 1.0, 1.0)
        cairo.cairo_arc(
            context,
            size / 2,
            size / 2,
            size / 2,
            0.0,
            2 * 3.141592653589793,
        )
        cairo.cairo_fill(context)

        if not rsvg.rsvg_handle_render_document(
            handle, context, ctypes.byref(viewport), ctypes.byref(error)
        ):
            raise RuntimeError(f"librsvg could not render {LOGO.name} at {size}px")

        destination = OUT_DIR / f"takto-{size}.png"
        status = cairo.cairo_surface_write_to_png(surface, os.fsencode(destination))
        if status != CAIRO_STATUS_SUCCESS:
            raise RuntimeError(f"cairo returned PNG status {status} at {size}px")
    finally:
        cairo.cairo_destroy(context)
        cairo.cairo_surface_destroy(surface)
        gobject.g_object_unref(handle)


def main() -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for size in SIZES:
        render(size)
        print(f"wrote {(OUT_DIR / f'takto-{size}.png').relative_to(ROOT)}")


if __name__ == "__main__":
    main()
