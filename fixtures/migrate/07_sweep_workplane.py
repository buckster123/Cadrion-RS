"""H5-9 fixture: Box recovers; Workplane/faces/sweep/scale are structured refuses.

Clean-room public API shape only — not a third-party file.
"""
from build123d import *

length = 40.0
width = 20.0
height = 10.0

with BuildPart() as part:
    Box(length, width, height)
    # CadQuery-shaped leftovers agents still paste:
    # Workplane("XY").faces(">Z").sweep(...).scale(2)
    sweep(path)  # noqa: F821 — intentional unmapped
