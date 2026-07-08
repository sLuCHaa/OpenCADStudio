use acadrust::{EntityType, Handle};

use crate::scene::model::object::{PropSection, PropValue, Property};

pub fn general_section(entity: &EntityType) -> PropSection {
    let common = entity.common();
    let linetype_display = if common.linetype.is_empty() {
        "ByLayer".to_string()
    } else {
        common.linetype.clone()
    };
    let transp_pct = (common.transparency.alpha() as f64 / 255.0 * 100.0).round() as u32;

    // Hyperlink is stored in XDATA under the "PE_URL" application.
    let hyperlink = common
        .extended_data
        .get_record("PE_URL")
        .and_then(|r| {
            r.values.iter().find_map(|v| match v {
                acadrust::xdata::XDataValue::String(s) if !s.is_empty() => Some(s.clone()),
                _ => None,
            })
        })
        .unwrap_or_default();

    let mut section = PropSection {
        title: "General".into(),
        props: vec![
            Property {
                label: "Handle".into(),
                field: "handle",
                value: PropValue::ReadOnly(common.handle.value().to_string()),
            },
            Property {
                label: "Color".into(),
                field: "color",
                value: PropValue::ColorChoice(common.color),
            },
            Property {
                label: "Layer".into(),
                field: "layer",
                value: PropValue::LayerChoice(common.layer.clone()),
            },
            Property {
                label: "Linetype".into(),
                field: "linetype",
                value: PropValue::LinetypeChoice(linetype_display),
            },
            Property {
                label: "LT Scale".into(),
                field: "linetype_scale",
                value: PropValue::EditText(format!("{:.4}", common.linetype_scale)),
            },
            Property {
                label: "Plot style".into(),
                field: "plot_style",
                value: PropValue::ReadOnly(
                    match common.plotstyle_flags {
                        0 => "ByLayer",
                        1 => "ByBlock",
                        _ => "ByColor",
                    }
                    .into(),
                ),
            },
            Property {
                label: "Lineweight".into(),
                field: "lineweight",
                value: PropValue::LwChoice(common.line_weight),
            },
            Property {
                label: "Transparency".into(),
                field: "transparency",
                value: PropValue::EditText(format!("{transp_pct}")),
            },
            Property {
                label: "Hyperlink".into(),
                field: "hyperlink",
                value: PropValue::ReadOnly(hyperlink),
            },
        ],
    };

    // Thickness (DXF 39) is a General-group property, but only the entity
    // types that carry an extrusion thickness expose it (line, circle, arc,
    // polyline, text, 2D solid, …). Show it right after Hyperlink for those.
    if let Some(t) = crate::scene::view::dispatch::entity_thickness(entity) {
        section
            .props
            .push(crate::entities::common::edit_prop("Thickness", "thickness", t));
    }

    section
}

/// The "3D Visualization" group (Material + Shadow display), common to every
/// graphical object. Material / plot-style / shadow source is flag-based; a
/// custom material handle is shown as "Custom" (name resolution needs the doc).
pub fn visualization_section(entity: &EntityType) -> Option<PropSection> {
    if matches!(
        entity,
        EntityType::Block(_)
            | EntityType::BlockEnd(_)
            | EntityType::Seqend(_)
            | EntityType::Unknown(_)
    ) {
        return None;
    }
    let common = entity.common();
    let material = match common.material_flags {
        0 => "ByLayer",
        1 => "ByBlock",
        _ => "Custom",
    };
    let shadow = match common.shadow_flags {
        0 => "Casts and Receives Shadows",
        1 => "Casts Shadows",
        2 => "Receives Shadows",
        _ => "Ignores Shadows",
    };
    Some(PropSection {
        title: "3D Visualization".into(),
        props: vec![
            Property {
                label: "Material".into(),
                field: "material",
                value: PropValue::ReadOnly(material.into()),
            },
            Property {
                label: "Shadow display".into(),
                field: "shadow_display",
                value: PropValue::ReadOnly(shadow.into()),
            },
        ],
    })
}

pub fn fallback_properties(_handle: Handle, entity: &EntityType) -> PropSection {
    PropSection {
        title: "Geometry".into(),
        props: vec![Property {
            label: "Type".into(),
            field: "type",
            value: PropValue::ReadOnly(crate::entities::names::ui_name(entity).into()),
        }],
    }
}

