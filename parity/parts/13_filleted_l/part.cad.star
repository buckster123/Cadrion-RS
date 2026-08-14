P = params(
    leg=40.0,
    width=20.0,
    thick=6.0,
    fillet_r=1.0,
)

def gen_step():
    # H3-7 hard golden: two-plate L union then all-edge fillet (OCCT-only).
    horiz = box(P.leg, P.width, P.thick, at=(P.leg / 2.0, 0.0, P.thick / 2.0))
    vert = box(P.thick, P.width, P.leg, at=(P.thick / 2.0, 0.0, P.leg / 2.0))
    body = union(horiz, vert)
    body = fillet(body, radius=P.fillet_r)
    return solid(body, label="filleted_l")
