use std::{env, fs, process};

use xdremux_format::isobmff::{
    parse_boxes, parse_ispe_dimensions, parse_meta_box, scan_top_level_boxes, BoxHeader, IPRP,
    ISPE, META,
};
use xdremux_format::{FormatError, FourCC, Result};

fn invalid(context: &'static str, message: impl Into<String>) -> FormatError {
    FormatError::InvalidData {
        context,
        message: message.into(),
    }
}

fn fourcc_hex(kind: FourCC) -> String {
    kind.as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn required_box<'a>(
    boxes: &'a [BoxHeader],
    kind: FourCC,
    context: &'static str,
) -> Result<&'a BoxHeader> {
    boxes
        .iter()
        .find(|header| header.kind == kind)
        .ok_or_else(|| invalid(context, format!("required box {kind} is missing")))
}

fn canonical_summary(data: &[u8]) -> Result<String> {
    let scan = scan_top_level_boxes(data)?;
    let mut lines = Vec::new();
    lines.push(format!("file\t{}", data.len()));
    for header in &scan.boxes {
        lines.push(format!(
            "box\t{}\t{}\t{}\t{}\t{}",
            fourcc_hex(header.kind),
            header.box_start,
            header.data_start,
            header.data_end,
            header.size
        ));
    }
    lines.push(format!(
        "trailer\t{}\t{}",
        scan.trailing_range.start,
        scan.trailing_range.end - scan.trailing_range.start
    ));

    let meta_header = required_box(&scan.boxes, META, "top-level HEIF")?;
    let meta = parse_meta_box(data, meta_header)?;
    lines.push(format!("primary\t{}", meta.primary_item_id));

    let mut iloc_entries = meta.iloc.entries.clone();
    iloc_entries.sort_by_key(|entry| entry.item_id);
    for entry in iloc_entries {
        let mut extents = Vec::new();
        for extent in &entry.extents {
            extents.push(format!(
                "{}:{}",
                entry.resolved_extent_offset(extent)?,
                extent.length
            ));
        }
        lines.push(format!(
            "iloc\t{}\t{}\t{}\t{}",
            entry.item_id,
            entry.construction_method,
            entry.data_reference_index,
            extents.join(",")
        ));
    }

    let mut items = meta.iinf.entries.clone();
    items.sort_by_key(|item| item.item_id);
    for item in items {
        if let Some(kind) = item.item_type {
            lines.push(format!(
                "iinf\t{}\t{}\t{}",
                item.item_id,
                fourcc_hex(kind),
                item.flags
            ));
        }
    }

    let mut ipma_entries = meta.ipma.entries.clone();
    ipma_entries.sort_by_key(|entry| entry.item_id);
    let wide_associations = meta.ipma.flags & 1 != 0;
    for entry in ipma_entries {
        let associations = entry
            .associations
            .iter()
            .map(|association| {
                let raw = if wide_associations {
                    association.property_index | if association.essential { 0x8000 } else { 0 }
                } else {
                    association.property_index | if association.essential { 0x0080 } else { 0 }
                };
                raw.to_string()
            })
            .collect::<Vec<_>>()
            .join(",");
        lines.push(format!("ipma\t{}\t{}", entry.item_id, associations));
    }

    if let Some(iref) = meta.iref {
        lines.push(format!("iref-version\t{}", iref.version));
        let mut entries = iref.entries;
        entries.sort_by(|left, right| {
            fourcc_hex(left.kind)
                .cmp(&fourcc_hex(right.kind))
                .then(left.from_item_id.cmp(&right.from_item_id))
                .then(left.to_item_ids.cmp(&right.to_item_ids))
        });
        for entry in entries {
            let targets = entry
                .to_item_ids
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(",");
            lines.push(format!(
                "iref\t{}\t{}\t{}",
                fourcc_hex(entry.kind),
                entry.from_item_id,
                targets
            ));
        }
    } else {
        lines.push("iref-version\t0".to_string());
    }

    let meta_children = parse_boxes(data, (meta_header.data_start + 4)..meta_header.data_end)?;
    let _iprp = required_box(&meta_children, IPRP, "meta")?;
    let mut properties = meta.properties;
    properties.sort_by_key(|property| property.index);
    for property in properties {
        let geometry = if property.kind == ISPE {
            let parsed = parse_boxes(data, property.box_range.clone())?;
            if parsed.len() != 1 {
                return Err(invalid(
                    "ipco property",
                    format!(
                        "property {} range contains {} boxes",
                        property.index,
                        parsed.len()
                    ),
                ));
            }
            let (width, height) = parse_ispe_dimensions(data, &parsed[0])?;
            format!("\t{width}\t{height}")
        } else {
            String::new()
        };
        lines.push(format!(
            "property\t{}\t{}{}",
            property.index,
            fourcc_hex(property.kind),
            geometry
        ));
    }

    Ok(lines.join("\n") + "\n")
}

fn run() -> Result<()> {
    let mut arguments = env::args_os();
    let _program = arguments.next();
    let Some(path) = arguments.next() else {
        return Err(invalid(
            "xdremux-format-inspect",
            "usage: xdremux-format-inspect <heif-file>",
        ));
    };
    if arguments.next().is_some() {
        return Err(invalid(
            "xdremux-format-inspect",
            "expected exactly one input path",
        ));
    }
    let data = fs::read(&path).map_err(|error| {
        invalid(
            "xdremux-format-inspect",
            format!("cannot read {}: {error}", path.to_string_lossy()),
        )
    })?;
    print!("{}", canonical_summary(&data)?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
