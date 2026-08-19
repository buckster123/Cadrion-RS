# Cadrion skeleton migrated from build123d-style Python (H8/H2-7 clean-room).
# Best-effort structure + params — NOT full semantic parity.
# Review numbers, placements, and booleans before fab.

P = params(
    fillet_r=2,
    height=10,
    length=50,
    width=30,
)

def gen_step():
    ext0 = box(50.0, 30.0, 10.0, at=CENTER)  # Rectangle+extrude
    # TODO fillet: e.g. body = fillet(body, radius)  # requires --kernel occt
    return solid(ext0, label="migrated")
