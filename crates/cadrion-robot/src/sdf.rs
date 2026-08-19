//! Minimal SDFormat model writer.

use crate::model::{Geometry, RobotSpec};
use crate::urdf::write_urdf;

/// Write a simple SDF 1.8 model embedding the robot as a model with links/joints.
/// Alpha: emits a thin SDF that includes the URDF-equivalent structure.
pub fn write_sdf(robot: &RobotSpec) -> String {
    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\"?>\n");
    out.push_str("<sdf version=\"1.8\">\n");
    out.push_str(&format!("  <model name=\"{}\">\n", xml_escape(&robot.name)));
    out.push_str("    <static>false</static>\n");

    for link in &robot.links {
        out.push_str(&format!("    <link name=\"{}\">\n", xml_escape(&link.name)));
        let i = &link.inertial;
        out.push_str("      <inertial>\n");
        out.push_str(&format!("        <mass>{}</mass>\n", i.mass));
        out.push_str(&format!(
            "        <inertia>\n          <ixx>{}</ixx><ixy>{}</ixy><ixz>{}</ixz>\n          <iyy>{}</iyy><iyz>{}</iyz><izz>{}</izz>\n        </inertia>\n",
            i.ixx, i.ixy, i.ixz, i.iyy, i.iyz, i.izz
        ));
        out.push_str("      </inertial>\n");
        if let Some(v) = link.visual.first() {
            out.push_str("      <visual name=\"visual\">\n");
            out.push_str("        <geometry>\n");
            out.push_str(&sdf_geom(&v.geometry));
            out.push_str("        </geometry>\n");
            out.push_str("      </visual>\n");
        }
        if let Some(c) = link.collision.first() {
            out.push_str("      <collision name=\"collision\">\n");
            out.push_str("        <geometry>\n");
            out.push_str(&sdf_geom(&c.geometry));
            out.push_str("        </geometry>\n");
            out.push_str("      </collision>\n");
        }
        out.push_str("    </link>\n");
    }

    for j in &robot.joints {
        let jtype = match j.joint_type {
            crate::model::JointType::Fixed => "fixed",
            crate::model::JointType::Revolute | crate::model::JointType::Continuous => "revolute",
            crate::model::JointType::Prismatic => "prismatic",
            _ => "fixed",
        };
        out.push_str(&format!(
            "    <joint name=\"{}\" type=\"{jtype}\">\n",
            xml_escape(&j.name)
        ));
        out.push_str(&format!(
            "      <parent>{}</parent>\n",
            xml_escape(&j.parent)
        ));
        out.push_str(&format!("      <child>{}</child>\n", xml_escape(&j.child)));
        out.push_str(&format!(
            "      <axis>\n        <xyz>{} {} {}</xyz>\n      </axis>\n",
            j.axis[0], j.axis[1], j.axis[2]
        ));
        out.push_str(&format!(
            "      <pose>{} {} {} {} {} {}</pose>\n",
            j.origin.xyz[0],
            j.origin.xyz[1],
            j.origin.xyz[2],
            j.origin.rpy[0],
            j.origin.rpy[1],
            j.origin.rpy[2]
        ));
        out.push_str("    </joint>\n");
    }

    out.push_str("  </model>\n");
    out.push_str("</sdf>\n");
    let _ = write_urdf(robot); // keep dependency warm for consistency helpers
    out
}

fn sdf_geom(g: &Geometry) -> String {
    match g {
        Geometry::Box { size } => {
            format!(
                "          <box><size>{} {} {}</size></box>\n",
                size[0], size[1], size[2]
            )
        }
        Geometry::Cylinder { radius, length } => format!(
            "          <cylinder><radius>{radius}</radius><length>{length}</length></cylinder>\n"
        ),
        Geometry::Sphere { radius } => {
            format!("          <sphere><radius>{radius}</radius></sphere>\n")
        }
        Geometry::Mesh { filename, scale } => format!(
            "          <mesh><uri>{filename}</uri><scale>{} {} {}</scale></mesh>\n",
            scale[0], scale[1], scale[2]
        ),
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
