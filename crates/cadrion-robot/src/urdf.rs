//! URDF XML writer.

use crate::model::{Geometry, Origin, RobotSpec};

fn fmt3(a: [f64; 3]) -> String {
    format!("{} {} {}", a[0], a[1], a[2])
}

fn fmt4(a: [f64; 4]) -> String {
    format!("{} {} {} {}", a[0], a[1], a[2], a[3])
}

fn origin_xml(o: &Origin, indent: &str) -> String {
    if o.xyz == [0.0, 0.0, 0.0] && o.rpy == [0.0, 0.0, 0.0] {
        return String::new();
    }
    format!(
        "{indent}<origin xyz=\"{}\" rpy=\"{}\"/>\n",
        fmt3(o.xyz),
        fmt3(o.rpy)
    )
}

fn geom_xml(g: &Geometry, indent: &str) -> String {
    match g {
        Geometry::Box { size } => {
            format!("{indent}<geometry>\n{indent}  <box size=\"{}\"/>\n{indent}</geometry>\n", fmt3(*size))
        }
        Geometry::Cylinder { radius, length } => format!(
            "{indent}<geometry>\n{indent}  <cylinder radius=\"{radius}\" length=\"{length}\"/>\n{indent}</geometry>\n"
        ),
        Geometry::Sphere { radius } => {
            format!("{indent}<geometry>\n{indent}  <sphere radius=\"{radius}\"/>\n{indent}</geometry>\n")
        }
        Geometry::Mesh { filename, scale } => format!(
            "{indent}<geometry>\n{indent}  <mesh filename=\"{filename}\" scale=\"{}\"/>\n{indent}</geometry>\n",
            fmt3(*scale)
        ),
    }
}

/// Emit URDF 1.0 XML string.
pub fn write_urdf(robot: &RobotSpec) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\"?>\n");
    out.push_str(&format!("<robot name=\"{}\">\n", xml_escape(&robot.name)));

    for link in &robot.links {
        out.push_str(&format!("  <link name=\"{}\">\n", xml_escape(&link.name)));
        let i = &link.inertial;
        out.push_str("    <inertial>\n");
        out.push_str(&origin_xml(
            &Origin {
                xyz: i.origin_xyz,
                rpy: i.origin_rpy,
            },
            "      ",
        ));
        out.push_str(&format!("      <mass value=\"{}\"/>\n", i.mass));
        out.push_str(&format!(
            "      <inertia ixx=\"{}\" ixy=\"{}\" ixz=\"{}\" iyy=\"{}\" iyz=\"{}\" izz=\"{}\"/>\n",
            i.ixx, i.ixy, i.ixz, i.iyy, i.iyz, i.izz
        ));
        out.push_str("    </inertial>\n");

        for v in &link.visual {
            out.push_str("    <visual>\n");
            out.push_str(&origin_xml(&v.origin, "      "));
            out.push_str(&geom_xml(&v.geometry, "      "));
            if let Some(m) = &v.material {
                out.push_str(&format!(
                    "      <material name=\"{}\">\n",
                    xml_escape(&m.name)
                ));
                if let Some(rgba) = m.rgba {
                    out.push_str(&format!("        <color rgba=\"{}\"/>\n", fmt4(rgba)));
                }
                out.push_str("      </material>\n");
            }
            out.push_str("    </visual>\n");
        }
        for c in &link.collision {
            out.push_str("    <collision>\n");
            out.push_str(&origin_xml(&c.origin, "      "));
            out.push_str(&geom_xml(&c.geometry, "      "));
            out.push_str("    </collision>\n");
        }
        out.push_str("  </link>\n");
    }

    for j in &robot.joints {
        out.push_str(&format!(
            "  <joint name=\"{}\" type=\"{}\">\n",
            xml_escape(&j.name),
            j.joint_type.as_urdf()
        ));
        out.push_str(&origin_xml(&j.origin, "    "));
        out.push_str(&format!(
            "    <parent link=\"{}\"/>\n",
            xml_escape(&j.parent)
        ));
        out.push_str(&format!("    <child link=\"{}\"/>\n", xml_escape(&j.child)));
        out.push_str(&format!("    <axis xyz=\"{}\"/>\n", fmt3(j.axis)));
        if j.lower.is_some() || j.upper.is_some() || j.effort.is_some() || j.velocity.is_some() {
            out.push_str(&format!(
                "    <limit lower=\"{}\" upper=\"{}\" effort=\"{}\" velocity=\"{}\"/>\n",
                j.lower.unwrap_or(0.0),
                j.upper.unwrap_or(0.0),
                j.effort.unwrap_or(0.0),
                j.velocity.unwrap_or(0.0)
            ));
        }
        out.push_str("  </joint>\n");
    }

    out.push_str("</robot>\n");
    out
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
