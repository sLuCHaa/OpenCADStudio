// List entities inside a named block, showing their layer + resolved linetype.
//
// cargo run --release --example inspect_block -- <file> <block_name>

use acadrust::entities::EntityType;
use acadrust::io::dwg::DwgReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).expect("usage: inspect_block <file> [block]");
    let block_name = std::env::args().nth(2);
    let doc = DwgReader::from_file(&path)?.read()?;

    if block_name.is_none() {
        let mut blocks: Vec<_> = doc
            .block_records
            .iter()
            .map(|br| {
                let n = doc
                    .entities()
                    .filter(|e| e.common().owner_handle == br.handle)
                    .count();
                (br.name.clone(), n)
            })
            .collect();
        blocks.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        println!("─── top 30 blocks by entity count ───────");
        for (name, n) in blocks.iter().take(30) {
            println!("  {:>5}  {}", n, name);
        }
        return Ok(());
    }
    let block_name = block_name.unwrap();

    // Find the block record by name.
    let br = doc
        .block_records
        .iter()
        .find(|br| br.name.eq_ignore_ascii_case(&block_name))
        .ok_or_else(|| format!("block '{}' not found", block_name))?;

    println!("block: \"{}\"  handle={:?}", br.name, br.handle);
    println!();

    let mut counts: std::collections::BTreeMap<&'static str, u32> = Default::default();
    let mut sample_by_kind: std::collections::BTreeMap<&'static str, Vec<String>> =
        Default::default();

    for e in doc.entities() {
        if e.common().owner_handle != br.handle {
            continue;
        }
        let kind: &'static str = match e {
            EntityType::Line(_) => "Line",
            EntityType::LwPolyline(_) => "LwPolyline",
            EntityType::Polyline(_) => "Polyline",
            EntityType::Polyline2D(_) => "Polyline2D",
            EntityType::Arc(_) => "Arc",
            EntityType::Circle(_) => "Circle",
            EntityType::Spline(_) => "Spline",
            EntityType::Ellipse(_) => "Ellipse",
            EntityType::Text(_) => "Text",
            EntityType::MText(_) => "MText",
            EntityType::Insert(_) => "Insert",
            EntityType::Hatch(_) => "Hatch",
            EntityType::Solid(_) => "Solid",
            _ => "Other",
        };
        *counts.entry(kind).or_default() += 1;

        let c = e.common();
        let extra = match e {
            EntityType::Insert(i) => format!(" → block=\"{}\"", i.block_name),
            _ => String::new(),
        };
        let line = format!(
            "  [{:?}] layer=\"{}\"  lt=\"{}\"  lt_scale={:.4}{}",
            e.common().handle, c.layer, c.linetype, c.linetype_scale, extra
        );
        let bucket = sample_by_kind.entry(kind).or_default();
        if bucket.len() < 4 {
            bucket.push(line);
        }
    }

    println!("─── entity counts in block ──────────────");
    for (k, v) in &counts {
        println!("  {:<12} {}", k, v);
    }
    println!();
    for (k, samples) in &sample_by_kind {
        println!("─── sample {} (first {}) ────────", k, samples.len());
        for s in samples {
            println!("{}", s);
        }
        println!();
    }

    println!("─── relevant layers ─────────────────────");
    let mut seen: std::collections::BTreeSet<String> = Default::default();
    for e in doc.entities() {
        if e.common().owner_handle == br.handle {
            seen.insert(e.common().layer.clone());
        }
    }
    for layer_name in &seen {
        if let Some(l) = doc.layers.get(layer_name) {
            println!(
                "  \"{}\"  lt=\"{}\"  off={} frozen={}",
                l.name, l.line_type, l.flags.off, l.flags.frozen
            );
        }
    }

    println!();
    println!("─── linetype table (dashed only) ────────");
    for lt in doc.line_types.iter() {
        if lt.elements.is_empty() || lt.is_continuous() {
            continue;
        }
        let elems: Vec<f64> = lt.elements.iter().map(|el| el.length).collect();
        println!(
            "  {:<24} pattern_len={:.4}  elements={:?}",
            lt.name, lt.pattern_length, elems
        );
    }

    Ok(())
}
