// Summarise linetype usage in a DWG/DXF: which layers use which linetype,
// and which entities carry an explicit (non-bylayer) dashed linetype.
//
// cargo run --release --example inspect_lt -- <file>

use acadrust::entities::EntityType;
use acadrust::io::dwg::DwgReader;
use std::collections::BTreeMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).expect("usage: inspect_lt <file>");
    let doc = DwgReader::from_file(&path)?.read()?;

    println!("─── layers (linetype) ───────────────────");
    let dashed_lts: std::collections::BTreeSet<String> = doc
        .line_types
        .iter()
        .filter(|lt| !lt.elements.is_empty() && !lt.is_continuous())
        .map(|lt| lt.name.clone())
        .collect();
    println!("  dashed linetypes in table: {:?}", dashed_lts);
    println!();

    let mut dashed_layers: BTreeMap<String, String> = BTreeMap::new();
    for l in doc.layers.iter() {
        if dashed_lts.contains(&l.line_type) {
            dashed_layers.insert(l.name.clone(), l.line_type.clone());
        }
    }
    println!("  layers using a dashed linetype:");
    if dashed_layers.is_empty() {
        println!("    (none)");
    }
    for (n, lt) in &dashed_layers {
        println!("    \"{}\" → \"{}\"", n, lt);
    }
    println!();

    // Per-entity explicit dashed override (lt field non-empty and non-byLayer/byBlock/Continuous).
    let mut explicit_dashed: u32 = 0;
    let mut by_layer_in_dashed_layer: u32 = 0;
    let mut by_layer_in_continuous: u32 = 0;
    let mut by_kind: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut sample_explicit: Vec<String> = vec![];
    let mut sample_layer_inh: Vec<String> = vec![];

    for e in doc.entities() {
        let c = e.common();
        let kind: &'static str = match e {
            EntityType::Line(_) => "Line",
            EntityType::LwPolyline(_) => "LwPolyline",
            EntityType::Polyline(_) => "Polyline",
            EntityType::Polyline2D(_) => "Polyline2D",
            EntityType::Arc(_) => "Arc",
            EntityType::Circle(_) => "Circle",
            EntityType::Spline(_) => "Spline",
            EntityType::Ellipse(_) => "Ellipse",
            EntityType::Insert(_) => "Insert",
            _ => "Other",
        };
        let lt = c.linetype.trim();
        let lt_norm = lt.to_ascii_lowercase();
        let is_bylayer = lt.is_empty() || lt_norm == "bylayer";
        let is_explicit_dashed = !is_bylayer
            && lt_norm != "byblock"
            && lt_norm != "continuous"
            && dashed_lts.iter().any(|d| d.eq_ignore_ascii_case(lt));
        if is_explicit_dashed {
            explicit_dashed += 1;
            *by_kind.entry(kind).or_default() += 1;
            if sample_explicit.len() < 6 {
                sample_explicit.push(format!(
                    "  [{:?}] {} layer=\"{}\" lt=\"{}\" lt_scale={:.4}",
                    c.handle, kind, c.layer, lt, c.linetype_scale
                ));
            }
        } else if is_bylayer && dashed_layers.contains_key(&c.layer) {
            by_layer_in_dashed_layer += 1;
            if sample_layer_inh.len() < 6 {
                sample_layer_inh.push(format!(
                    "  [{:?}] {} layer=\"{}\" (inherits \"{}\") lt_scale={:.4}",
                    c.handle,
                    kind,
                    c.layer,
                    dashed_layers.get(&c.layer).unwrap(),
                    c.linetype_scale
                ));
            }
        } else if is_bylayer {
            by_layer_in_continuous += 1;
        }
    }

    println!("─── entities w/ explicit dashed lt ──────");
    println!("  total: {}", explicit_dashed);
    for (k, v) in &by_kind {
        println!("    {}: {}", k, v);
    }
    for s in &sample_explicit {
        println!("{}", s);
    }
    println!();

    println!("─── entities inheriting dashed via layer:");
    println!("  total: {}", by_layer_in_dashed_layer);
    for s in &sample_layer_inh {
        println!("{}", s);
    }
    println!();
    println!(
        "─── entities bylayer/continuous: {}",
        by_layer_in_continuous
    );

    Ok(())
}
