"""H3-9 fixture: Circle + extrude → cylinder (clean-room public API shape)."""
from build123d import *

radius = 8.0
height = 24.0

with BuildSketch() as sk:
    Circle(radius)

with BuildPart() as part:
    extrude(amount=height)
