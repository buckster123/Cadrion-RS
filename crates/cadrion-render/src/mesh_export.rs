//! Mesh writers for agent export (STL ASCII + JSON glTF). Not OCCT tessellation.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use cadrion_kernel::Mesh;
use serde_json::json;

/// Write an ASCII STL. Preview meshes (IR-analytic) are valid input.
pub fn write_stl_ascii(path: &Path, mesh: &Mesh) -> std::io::Result<()> {
    let mut f = fs::File::create(path)?;
    writeln!(f, "solid cadrion")?;
    let p = &mesh.positions;
    for tri in mesh.indices.as_chunks::<3>().0 {
        let i0 = tri[0] as usize * 3;
        let i1 = tri[1] as usize * 3;
        let i2 = tri[2] as usize * 3;
        let ax = p[i0];
        let ay = p[i0 + 1];
        let az = p[i0 + 2];
        let bx = p[i1];
        let by = p[i1 + 1];
        let bz = p[i1 + 2];
        let cx = p[i2];
        let cy = p[i2 + 1];
        let cz = p[i2 + 2];
        let ux = bx - ax;
        let uy = by - ay;
        let uz = bz - az;
        let vx = cx - ax;
        let vy = cy - ay;
        let vz = cz - az;
        let nx = uy * vz - uz * vy;
        let ny = uz * vx - ux * vz;
        let nz = ux * vy - uy * vx;
        writeln!(f, "  facet normal {nx} {ny} {nz}")?;
        writeln!(f, "    outer loop")?;
        writeln!(f, "      vertex {ax} {ay} {az}")?;
        writeln!(f, "      vertex {bx} {by} {bz}")?;
        writeln!(f, "      vertex {cx} {cy} {cz}")?;
        writeln!(f, "    endloop")?;
        writeln!(f, "  endfacet")?;
    }
    writeln!(f, "endsolid cadrion")?;
    Ok(())
}

/// Write JSON glTF with embedded buffers. A `.glb` request becomes `.gltf` (no binary pack).
pub fn write_gltf_json(path: &Path, mesh: &Mesh) -> std::io::Result<PathBuf> {
    let out = if path.extension().and_then(|e| e.to_str()) == Some("glb") {
        path.with_extension("gltf")
    } else {
        path.to_path_buf()
    };
    let mut bin = Vec::with_capacity(mesh.positions.len() * 4);
    for f in &mesh.positions {
        bin.extend_from_slice(&f.to_le_bytes());
    }
    let b64 = base64_encode(&bin);
    let n_verts = mesh.positions.len() / 3;
    let indices: Vec<u32> = mesh.indices.clone();
    let mut idx_bin = Vec::with_capacity(indices.len() * 4);
    for i in &indices {
        idx_bin.extend_from_slice(&i.to_le_bytes());
    }
    let idx_b64 = base64_encode(&idx_bin);
    let gltf = json!({
        "asset": {"version": "2.0", "generator": "cadrion"},
        "buffers": [
            {"byteLength": bin.len(), "uri": format!("data:application/octet-stream;base64,{b64}")},
            {"byteLength": idx_bin.len(), "uri": format!("data:application/octet-stream;base64,{idx_b64}")},
        ],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": bin.len(), "target": 34962},
            {"buffer": 1, "byteOffset": 0, "byteLength": idx_bin.len(), "target": 34963},
        ],
        "accessors": [
            {
                "bufferView": 0,
                "componentType": 5126,
                "count": n_verts,
                "type": "VEC3",
            },
            {
                "bufferView": 1,
                "componentType": 5125,
                "count": indices.len(),
                "type": "SCALAR",
            }
        ],
        "meshes": [{
            "primitives": [{
                "attributes": {"POSITION": 0},
                "indices": 1,
                "mode": 4
            }]
        }],
        "nodes": [{"mesh": 0}],
        "scenes": [{"nodes": [0]}],
        "scene": 0
    });
    fs::write(&out, serde_json::to_vec_pretty(&gltf).unwrap())?;
    Ok(out)
}

fn base64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= data.len() {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | (data[i + 2] as u32);
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(T[((n >> 6) & 63) as usize] as char);
        out.push(T[(n & 63) as usize] as char);
        i += 3;
    }
    match data.len() - i {
        1 => {
            let n = (data[i] as u32) << 16;
            out.push(T[((n >> 18) & 63) as usize] as char);
            out.push(T[((n >> 12) & 63) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8);
            out.push(T[((n >> 18) & 63) as usize] as char);
            out.push(T[((n >> 12) & 63) as usize] as char);
            out.push(T[((n >> 6) & 63) as usize] as char);
            out.push('=');
        }
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadrion_kernel::Mesh;

    #[test]
    fn stl_writes_solid() {
        let mesh = Mesh {
            positions: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            normals: None,
            indices: vec![0, 1, 2],
        };
        let dir = std::env::temp_dir().join(format!("cadrion-stl-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.stl");
        write_stl_ascii(&p, &mesh).unwrap();
        let s = fs::read_to_string(&p).unwrap();
        assert!(s.starts_with("solid cadrion"));
        let _ = fs::remove_dir_all(&dir);
    }
}
