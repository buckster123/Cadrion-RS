//! Software z-buffer rasterizer (orthographic).

use cadrion_kernel::{Mesh, Point3};

use crate::views::Camera;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
}

#[derive(Debug, Clone)]
pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA8
}

impl Framebuffer {
    pub fn new(width: u32, height: u32, clear: Rgba) -> Self {
        let n = (width * height) as usize;
        let mut pixels = Vec::with_capacity(n * 4);
        for _ in 0..n {
            pixels.extend_from_slice(&[clear.r, clear.g, clear.b, clear.a]);
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    pub fn set(&mut self, x: i32, y: i32, c: Rgba) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        self.pixels[i] = c.r;
        self.pixels[i + 1] = c.g;
        self.pixels[i + 2] = c.b;
        self.pixels[i + 3] = c.a;
    }

    /// RGBA8 → RGB8 for GIF.
    pub fn to_rgb8(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity((self.width * self.height * 3) as usize);
        for px in self.pixels.chunks_exact(4) {
            out.push(px[0]);
            out.push(px[1]);
            out.push(px[2]);
        }
        out
    }
}

#[derive(Clone, Copy)]
struct V3 {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(Clone, Copy)]
struct Pt {
    x: f64,
    y: f64,
    z: f64,
}

/// Render mesh with simple Lambertian shading.
pub fn render_mesh(mesh: &Mesh, cam: &Camera, width: u32, height: u32) -> Framebuffer {
    let bg = Rgba::rgb(24, 26, 32);
    let mut fb = Framebuffer::new(width, height, bg);
    let mut zbuf = vec![f64::INFINITY; (width * height) as usize];

    let eye = v3(cam.eye);
    let target = v3(cam.target);
    let up = v3(cam.up);
    let f = norm(sub(target, eye));
    let r = norm(cross(f, up));
    let u = norm(cross(r, f));

    let he = cam.half_extent;
    let w = width as f64;
    let h = height as f64;
    let light = norm(V3 {
        x: 0.35,
        y: -0.55,
        z: 0.75,
    });

    let pos = &mesh.positions;
    for tri in mesh.indices.chunks_exact(3) {
        let a = load(pos, tri[0]);
        let b = load(pos, tri[1]);
        let c = load(pos, tri[2]);
        let n = norm(cross(sub(b, a), sub(c, a)));
        let ndotl = (n.x * light.x + n.y * light.y + n.z * light.z).clamp(0.0, 1.0);
        let shade = 0.22 + 0.78 * ndotl;
        let col = Rgba::rgb(
            (40.0 + 150.0 * shade) as u8,
            (50.0 + 160.0 * shade) as u8,
            (70.0 + 170.0 * shade) as u8,
        );

        let pa = project(a, eye, f, r, u, he, w, h);
        let pb = project(b, eye, f, r, u, he, w, h);
        let pc = project(c, eye, f, r, u, he, w, h);
        fill_triangle(&mut fb, &mut zbuf, pa, pb, pc, col);
    }

    // Footer bar (annotation strip)
    let bar_h = (height / 18).max(10);
    for y in (height - bar_h)..height {
        for x in 0..width {
            fb.set(x as i32, y as i32, Rgba::rgb(16, 16, 20));
        }
    }
    fb
}

fn v3(p: Point3) -> V3 {
    V3 {
        x: p.x,
        y: p.y,
        z: p.z,
    }
}

fn load(pos: &[f32], idx: u32) -> V3 {
    let i = idx as usize * 3;
    V3 {
        x: pos[i] as f64,
        y: pos[i + 1] as f64,
        z: pos[i + 2] as f64,
    }
}

fn sub(a: V3, b: V3) -> V3 {
    V3 {
        x: a.x - b.x,
        y: a.y - b.y,
        z: a.z - b.z,
    }
}

fn cross(a: V3, b: V3) -> V3 {
    V3 {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}

fn norm(v: V3) -> V3 {
    let l = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt().max(1e-12);
    V3 {
        x: v.x / l,
        y: v.y / l,
        z: v.z / l,
    }
}

#[allow(clippy::too_many_arguments)]
fn project(p: V3, eye: V3, f: V3, r: V3, u: V3, he: f64, w: f64, h: f64) -> Pt {
    let v = sub(p, eye);
    let cx = v.x * r.x + v.y * r.y + v.z * r.z;
    let cy = v.x * u.x + v.y * u.y + v.z * u.z;
    let cz = v.x * f.x + v.y * f.y + v.z * f.z;
    let aspect = w / h.max(1.0);
    let ndc_x = cx / (he * aspect);
    let ndc_y = cy / he;
    Pt {
        x: (ndc_x * 0.5 + 0.5) * (w - 1.0),
        y: (1.0 - (ndc_y * 0.5 + 0.5)) * (h - 1.0),
        z: cz,
    }
}

fn fill_triangle(fb: &mut Framebuffer, zbuf: &mut [f64], a: Pt, b: Pt, c: Pt, col: Rgba) {
    let min_x = a.x.min(b.x).min(c.x).floor() as i32;
    let max_x = a.x.max(b.x).max(c.x).ceil() as i32;
    let min_y = a.y.min(b.y).min(c.y).floor() as i32;
    let max_y = a.y.max(b.y).max(c.y).ceil() as i32;
    let area = edge(a, b, c);
    if area.abs() < 1e-9 {
        return;
    }
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if x < 0 || y < 0 || x >= fb.width as i32 || y >= fb.height as i32 {
                continue;
            }
            let p = Pt {
                x: x as f64 + 0.5,
                y: y as f64 + 0.5,
                z: 0.0,
            };
            let w0 = edge(b, c, p);
            let w1 = edge(c, a, p);
            let w2 = edge(a, b, p);
            if (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0) || (w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0) {
                let b0 = w0 / area;
                let b1 = w1 / area;
                let b2 = w2 / area;
                // perspective-correct not needed (ortho)
                let z = b0 * a.z + b1 * b.z + b2 * c.z;
                let zi = (y as u32 * fb.width + x as u32) as usize;
                if z < zbuf[zi] {
                    zbuf[zi] = z;
                    fb.set(x, y, col);
                }
            }
        }
    }
}

fn edge(a: Pt, b: Pt, c: Pt) -> f64 {
    (c.x - a.x) * (b.y - a.y) - (c.y - a.y) * (b.x - a.x)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::{camera_for_view, ViewName};
    use cadrion_kernel::Point3;

    fn unit_box_mesh() -> Mesh {
        // 2x2x2 box centered at origin
        let hx = 1.0f32;
        let corners = [
            [-hx, -hx, -hx],
            [hx, -hx, -hx],
            [hx, hx, -hx],
            [-hx, hx, -hx],
            [-hx, -hx, hx],
            [hx, -hx, hx],
            [hx, hx, hx],
            [-hx, hx, hx],
        ];
        let mut positions = Vec::new();
        for c in corners {
            positions.extend_from_slice(&c);
        }
        let faces = [
            [0, 1, 2, 3], // -Z
            [4, 7, 6, 5], // +Z
            [0, 4, 5, 1], // -Y
            [2, 6, 7, 3], // +Y
            [0, 3, 7, 4], // -X
            [1, 5, 6, 2], // +X
        ];
        let mut indices = Vec::new();
        for f in faces {
            indices.extend_from_slice(&[f[0], f[1], f[2], f[0], f[2], f[3]]);
        }
        Mesh {
            positions,
            indices,
            normals: None,
        }
    }

    #[test]
    fn renders_nonzero_pixels() {
        let mesh = unit_box_mesh();
        let cam = camera_for_view(ViewName::Iso, Point3::ORIGIN, 1.5);
        let fb = render_mesh(&mesh, &cam, 64, 64);
        let lit = fb
            .pixels
            .chunks_exact(4)
            .filter(|p| p[0] > 30 || p[1] > 30 || p[2] > 40)
            .count();
        assert!(lit > 50, "expected shaded pixels, got {lit}");
    }
}
