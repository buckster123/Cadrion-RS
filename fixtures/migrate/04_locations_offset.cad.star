# Cadrion skeleton migrated from build123d-style Python (H8/H2-7 clean-room).
# Best-effort structure + params — NOT full semantic parity.
# Review numbers, placements, and booleans before fab.

P = params()

def gen_step():
    box0 = box(40.0, 20.0, 8.0, at=CENTER)  # Box positional
    cyl1 = cylinder(4.0, 12.0, at=CENTER)  # Cylinder positional
    tr2 = translate(box0, 25, 0, 0)  # from Locations
    u3 = union(tr2, cyl1)
    return solid(u3, label="migrated")
