use acadrust::entities::Ellipse;
use truck_modeling::{builder, BSplineCurve, Curve, Edge, KnotVec, Point3, Wire};

use crate::command::EntityTransform;
use crate::entities::common::{
    center_grip, edit_prop as edit, parse_f64, ro_prop as ro, square_grip,
};
use crate::entities::traits::TruckConvertible;
use crate::scene::convert::acad_to_truck::{TruckEntity, TruckObject};
use crate::scene::model::object::{GripApply, GripDef, PropSection};
use crate::scene::model::wire_model::SnapHint;

const TAU: f64 = std::f64::consts::TAU;

fn to_truck(ell: &Ellipse) -> TruckEntity {
    let normal = (ell.normal.x, ell.normal.y, ell.normal.z);
    let (nx, ny, nz) = normal;

    // ELLIPSE is one of the few WCS entities in DXF: `center` (code 10) and
    // `major_axis` (code 11) are world coordinates already — unlike ARC /
    // CIRCLE, whose centers are OCS. (This used to run both through the
    // arbitrary-axis OCS, which misplaced any ellipse whose normal isn't
    // Z-up — e.g. the (0,0,-1) result of a mirrored-block explode.)
    let (cwx, cwy, cwz) = (ell.center.x, ell.center.y, ell.center.z);
    let wcs_maj = glam::Vec3::new(
        ell.major_axis.x as f32,
        ell.major_axis.y as f32,
        ell.major_axis.z as f32,
    );
    let r_major = wcs_maj.length() as f64;
    let r_minor = r_major * ell.minor_axis_ratio;
    let t0 = ell.start_parameter;
    let mut t1 = ell.end_parameter;
    if t1 <= t0 {
        t1 += TAU;
    }
    let u = if r_major > 1e-9 {
        wcs_maj / wcs_maj.length()
    } else {
        glam::Vec3::X
    };
    // Minor axis direction: WCS_normal × u (both unit vectors, always perpendicular).
    let wcs_normal = glam::Vec3::new(nx as f32, ny as f32, nz as f32);
    let v_axis = wcs_normal.cross(u);
    let center_v3 = glam::DVec3::new(cwx, cwy, cwz);
    let is_closed = (t1 - t0 - TAU).abs() < 1e-6;

    if is_closed {
        let n = 16usize;
        let pts_upper: Vec<Point3> = (0..=n)
            .map(|i| {
                let t = (i as f64 / n as f64) * std::f64::consts::PI;
                let lx = (r_major * t.cos()) as f32;
                let lz = (r_minor * t.sin()) as f32;
                Point3::new(
                    cwx + (lx * u.x + lz * v_axis.x) as f64,
                    cwy + (lx * u.y + lz * v_axis.y) as f64,
                    cwz + (lx * u.z + lz * v_axis.z) as f64,
                )
            })
            .collect();
        let pts_lower: Vec<Point3> = (0..=n)
            .map(|i| {
                let t = std::f64::consts::PI + (i as f64 / n as f64) * std::f64::consts::PI;
                let lx = (r_major * t.cos()) as f32;
                let lz = (r_minor * t.sin()) as f32;
                Point3::new(
                    cwx + (lx * u.x + lz * v_axis.x) as f64,
                    cwy + (lx * u.y + lz * v_axis.y) as f64,
                    cwz + (lx * u.z + lz * v_axis.z) as f64,
                )
            })
            .collect();
        let v_pos = builder::vertex(*pts_upper.first().unwrap());
        let v_neg = builder::vertex(*pts_upper.last().unwrap());
        let kv_u = KnotVec::uniform_knot(1, n);
        let kv_l = KnotVec::uniform_knot(1, n);
        let spl_u = BSplineCurve::new(kv_u, pts_upper);
        let spl_l = BSplineCurve::new(kv_l, pts_lower);
        let edge_upper = Edge::new(&v_pos, &v_neg, Curve::BSplineCurve(spl_u));
        let edge_lower = Edge::new(&v_neg, &v_pos, Curve::BSplineCurve(spl_l));
        let wire: Wire = [edge_upper, edge_lower].into_iter().collect();
        // Quadrant points at ±major and ±minor axis endpoints in WCS.
        let q = |lx: f64, lz: f64| {
            glam::DVec3::new(
                cwx + lx * u.x as f64 + lz * v_axis.x as f64,
                cwy + lx * u.y as f64 + lz * v_axis.y as f64,
                cwz + lx * u.z as f64 + lz * v_axis.z as f64,
            )
        };
        let snap_pts = vec![
            (center_v3, SnapHint::Center),
            (q(r_major, 0.0), SnapHint::Quadrant),
            (q(-r_major, 0.0), SnapHint::Quadrant),
            (q(0.0, r_minor), SnapHint::Quadrant),
            (q(0.0, -r_minor), SnapHint::Quadrant),
        ];
        TruckEntity {
            object: TruckObject::Contour(wire),
            snap_pts,
            tangent_geoms: vec![],
            key_vertices: vec![],
            fill_tris: vec![],
        }
    } else {
        let n = 32usize;
        let ctrl_pts: Vec<Point3> = (0..=n)
            .map(|i| {
                let t = t0 + (t1 - t0) * (i as f64 / n as f64);
                let lx = (r_major * t.cos()) as f32;
                let lz = (r_minor * t.sin()) as f32;
                Point3::new(
                    cwx + (lx * u.x + lz * v_axis.x) as f64,
                    cwy + (lx * u.y + lz * v_axis.y) as f64,
                    cwz + (lx * u.z + lz * v_axis.z) as f64,
                )
            })
            .collect();
        let kv = KnotVec::uniform_knot(1, n);
        let bspline = BSplineCurve::new(kv, ctrl_pts.clone());
        let v_start = builder::vertex(*ctrl_pts.first().unwrap());
        let v_end = builder::vertex(*ctrl_pts.last().unwrap());
        let edge = Edge::new(&v_start, &v_end, Curve::BSplineCurve(bspline));
        let pt_start = ctrl_pts.first().unwrap();
        let pt_end = ctrl_pts.last().unwrap();
        let key_vertices: Vec<[f64; 3]> = vec![
            [pt_start.x, pt_start.y, pt_start.z],
            [pt_end.x, pt_end.y, pt_end.z],
        ];
        TruckEntity {
            object: TruckObject::Curve(edge),
            snap_pts: vec![(center_v3, SnapHint::Center)],
            tangent_geoms: vec![],
            key_vertices,
            fill_tris: vec![],
        }
    }
}

fn grips(ell: &Ellipse) -> Vec<GripDef> {
    let ctr = glam::DVec3::new(ell.center.x, ell.center.y, ell.center.z);
    let maj = glam::DVec3::new(
        ell.center.x + ell.major_axis.x,
        ell.center.y + ell.major_axis.y,
        ell.center.z + ell.major_axis.z,
    );
    let major_xy =
        ((ell.major_axis.x * ell.major_axis.x + ell.major_axis.y * ell.major_axis.y) as f64).sqrt();
    let (px, py) = if major_xy > 1e-10 {
        let s = ell.major_axis_length() * ell.minor_axis_ratio / major_xy;
        (-ell.major_axis.y * s, ell.major_axis.x * s)
    } else {
        (0.0, ell.major_axis_length() * ell.minor_axis_ratio)
    };
    let min = glam::DVec3::new(ell.center.x + px, ell.center.y + py, ell.center.z);
    vec![
        center_grip(0, ctr),
        square_grip(1, maj),
        square_grip(2, min),
    ]
}

fn properties(ell: &Ellipse) -> Vec<PropSection> {
    use crate::entities::traits::MassPropsCalc;

    let r_major = ell.major_axis_length();
    let r_minor = ell.minor_axis_length();

    // Axis frame in WCS: major-axis unit vector `u`, minor direction `v = normal × u`.
    let u = glam::DVec3::new(ell.major_axis.x, ell.major_axis.y, ell.major_axis.z);
    let u = if r_major > 1e-12 {
        u / r_major
    } else {
        glam::DVec3::X
    };
    let n = glam::DVec3::new(ell.normal.x, ell.normal.y, ell.normal.z);
    let v = n.cross(u);

    // Parametric end point in WCS: center + r_major·cos(t1)·u + r_minor·sin(t1)·v.
    let t1 = ell.end_parameter;
    let end = glam::DVec3::new(ell.center.x, ell.center.y, ell.center.z)
        + u * (r_major * t1.cos())
        + v * (r_minor * t1.sin());

    let start_angle = ell.start_parameter.to_degrees().rem_euclid(360.0);
    let end_angle = ell.end_parameter.to_degrees().rem_euclid(360.0);

    let props = ell.mass_props();

    vec![
        PropSection {
            title: "Geometry".into(),
            props: vec![
                edit("Center X", "center_x", ell.center.x),
                edit("Center Y", "center_y", ell.center.y),
                edit("Center Z", "center_z", ell.center.z),
                ro("End X", "end_x", format!("{:.4}", end.x)),
                ro("End Y", "end_y", format!("{:.4}", end.y)),
                ro("End Z", "end_z", format!("{:.4}", end.z)),
                edit("Major radius", "major_r", r_major),
                ro("Minor radius", "minor_r", format!("{r_minor:.4}")),
                edit("Radius ratio", "ratio", ell.minor_axis_ratio),
                ro("Start angle", "start_angle", format!("{start_angle:.2}")),
                ro("End angle", "end_angle", format!("{end_angle:.2}")),
                edit("Start parameter", "start_param", ell.start_parameter),
                edit("End parameter", "end_param", ell.end_parameter),
                ro("Area", "area", format!("{:.4}", props.area)),
                ro("Length", "length", format!("{:.4}", props.perimeter)),
            ],
        },
        PropSection {
            title: "Misc".into(),
            props: vec![
                edit("Normal X", "normal_x", ell.normal.x),
                edit("Normal Y", "normal_y", ell.normal.y),
                edit("Normal Z", "normal_z", ell.normal.z),
            ],
        },
    ]
}

fn apply_geom_prop(ell: &mut Ellipse, field: &str, value: &str) {
    let Some(v) = parse_f64(value) else {
        return;
    };
    match field {
        "center_x" => ell.center.x = v,
        "center_y" => ell.center.y = v,
        "center_z" => ell.center.z = v,
        "major_r" => {
            let cur = ell.major_axis_length();
            if cur > 1e-12 && v > 0.0 {
                let s = v / cur;
                ell.major_axis.x *= s;
                ell.major_axis.y *= s;
                ell.major_axis.z *= s;
            }
        }
        "ratio" => ell.minor_axis_ratio = v,
        "start_param" => ell.start_parameter = v,
        "end_param" => ell.end_parameter = v,
        "normal_x" => ell.normal.x = v,
        "normal_y" => ell.normal.y = v,
        "normal_z" => ell.normal.z = v,
        _ => {}
    }
}

fn apply_grip(ell: &mut Ellipse, grip_id: usize, apply: GripApply) {
    match (grip_id, apply) {
        (0, GripApply::Translate(d)) => {
            ell.center.x += d.x as f64;
            ell.center.y += d.y as f64;
            ell.center.z += d.z as f64;
        }
        (0, GripApply::Absolute(p)) => {
            ell.center.x = p.x as f64;
            ell.center.y = p.y as f64;
            ell.center.z = p.z as f64;
        }
        (1, GripApply::Absolute(p)) => {
            ell.major_axis.x = p.x as f64 - ell.center.x;
            ell.major_axis.y = p.y as f64 - ell.center.y;
            ell.major_axis.z = p.z as f64 - ell.center.z;
        }
        (2, GripApply::Absolute(p)) => {
            let major_len = ell.major_axis_length();
            if major_len > 1e-10 {
                let dx = p.x as f64 - ell.center.x;
                let dy = p.y as f64 - ell.center.y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= major_len {
                    // Minor axis stays the shorter one: just update the ratio.
                    ell.minor_axis_ratio = (dist / major_len).clamp(0.001, 1.0);
                } else if dist > 1e-10 {
                    // Dragged past the major length. Rather than clamping (which
                    // pins the minor grip at the major radius), rotate the
                    // definition: the minor direction becomes the new major axis
                    // so the stored ratio stays <= 1.0 as the file format requires.
                    let perp_x = -ell.major_axis.y;
                    let perp_y = ell.major_axis.x;
                    let perp_len = (perp_x * perp_x + perp_y * perp_y).sqrt();
                    if perp_len > 1e-10 {
                        let s = dist / perp_len;
                        ell.major_axis.x = perp_x * s;
                        ell.major_axis.y = perp_y * s;
                        ell.minor_axis_ratio = (major_len / dist).clamp(0.001, 1.0);
                    }
                }
            }
        }
        _ => {}
    }
}

fn apply_transform(ell: &mut Ellipse, t: &EntityTransform) {
    crate::scene::view::transform::apply_standard_entity_transform(ell, t, |entity, p1, p2| {
        crate::scene::view::transform::reflect_xy_point(
            &mut entity.center.x,
            &mut entity.center.y,
            p1,
            p2,
        );
        crate::scene::view::transform::reflect_xy_point(
            &mut entity.major_axis.x,
            &mut entity.major_axis.y,
            p1,
            p2,
        );
    });
}

impl TruckConvertible for Ellipse {
    fn to_truck(&self, _document: &acadrust::CadDocument) -> Option<TruckEntity> {
        Some(to_truck(self))
    }
}

crate::impl_entity_basics!(Ellipse);

impl crate::entities::traits::MassPropsCalc for acadrust::entities::Ellipse {
    fn mass_props(&self) -> crate::entities::traits::MassProps {
        use std::f64::consts::{PI, TAU};
        let e = self;
        let a = (e.major_axis.x.powi(2) + e.major_axis.y.powi(2)).sqrt();
        let b = a * e.minor_axis_ratio;
        let t0 = e.start_parameter;
        let t1 = {
            let mut t = e.end_parameter;
            if t <= t0 {
                t += TAU;
            }
            t
        };
        let span = t1 - t0;
        let is_full = (span - TAU).abs() < 1e-6;
        let area = if is_full {
            PI * a * b
        } else {
            // Sector area of ellipse approximated via 256-pt integration
            let n = 256usize;
            let mut s = 0.0f64;
            for k in 0..n {
                let t = t0 + span * (k as f64 / n as f64);
                let tp = t0 + span * ((k + 1) as f64 / n as f64);
                let nx = e.major_axis.x / a;
                let ny = e.major_axis.y / a;
                let x0 = a * t.cos() * nx - b * t.sin() * ny;
                let y0 = a * t.cos() * ny + b * t.sin() * nx;
                let x1 = a * tp.cos() * nx - b * tp.sin() * ny;
                let y1 = a * tp.cos() * ny + b * tp.sin() * nx;
                s += x0 * y1 - x1 * y0;
            }
            (s / 2.0).abs()
        };
        // Arc length via 256-pt numerical integration
        let nx = e.major_axis.x / a.max(1e-12);
        let ny = e.major_axis.y / a.max(1e-12);
        let perimeter = {
            let n = 256usize;
            let mut len = 0.0f64;
            for k in 0..n {
                let t = t0 + span * (k as f64 / n as f64);
                let tp = t0 + span * ((k + 1) as f64 / n as f64);
                let x0 = e.center.x + a * t.cos() * nx - b * t.sin() * ny;
                let y0 = e.center.y + a * t.cos() * ny + b * t.sin() * nx;
                let x1 = e.center.x + a * tp.cos() * nx - b * tp.sin() * ny;
                let y1 = e.center.y + a * tp.cos() * ny + b * tp.sin() * nx;
                len += (x1 - x0).hypot(y1 - y0);
            }
            len
        };
        crate::entities::traits::MassProps {
            area,
            perimeter,
            cx: e.center.x,
            cy: e.center.y,
        }
    }
}
