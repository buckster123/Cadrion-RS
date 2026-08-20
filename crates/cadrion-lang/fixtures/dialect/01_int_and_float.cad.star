# H5-7: ints and floats both become IR f64.
def gen_step():
    return solid(box(10, 20, 30.0, at=CENTER), label="cube")
