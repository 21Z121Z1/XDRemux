use xdremux_format::isobmff::{
    make_box, make_full_box, make_iloc_box, parse_boxes, parse_meta_box, scan_top_level_boxes,
    BoxHeader, IlocEntry, ParsedMeta, ILOC, MDAT, META,
};
use xdremux_format::FourCC;

use crate::error::{HeifError, Result};

const HDLR: FourCC = FourCC::new(*b"hdlr");
const DINF: FourCC = FourCC::new(*b"dinf");
const DREF: FourCC = FourCC::new(*b"dref");
const URL_: FourCC = FourCC::new(*b"url ");

fn invalid(message: impl Into<String>) -> HeifError {
    HeifError::invalid(message)
}

fn raw_box<'a>(source: &'a [u8], header: &BoxHeader, context: &str) -> Result<&'a [u8]> {
    source
        .get(header.box_range())
        .ok_or_else(|| invalid(format!("{context} box is outside source")))
}

fn make_dinf_box() -> Result<Vec<u8>> {
    // ISO-BMFF's self-contained data reference: one `url ` entry with flag 1.
    // This is the same minimal DataInformation graph used by the proven
    // ImageIO-compatible resource writer; it introduces no Apple product policy.
    let url = make_full_box(URL_, 0, 1, &[])?;
    let mut dref_payload = Vec::with_capacity(4 + url.len());
    dref_payload.extend_from_slice(&1_u32.to_be_bytes());
    dref_payload.extend_from_slice(&url);
    let dref = make_full_box(DREF, 0, 0, &dref_payload)?;
    Ok(make_box(DINF, &dref)?)
}

fn rebuild_meta_with_dinf(
    source: &[u8],
    meta_header: &BoxHeader,
    meta_children: &[BoxHeader],
    iloc: &[u8],
) -> Result<Vec<u8>> {
    let full_header_end = meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("meta full-box header offset overflows"))?;
    let full_header = source
        .get(meta_header.data_start..full_header_end)
        .ok_or_else(|| invalid("meta full-box header is truncated"))?;

    let mut payload = full_header.to_vec();
    let mut saw_hdlr = false;
    let mut saw_iloc = false;

    for header in meta_children {
        if header.kind == ILOC {
            if saw_iloc {
                return Err(invalid("canonical HEIF meta contains more than one iloc"));
            }
            saw_iloc = true;
            payload.extend_from_slice(iloc);
        } else {
            payload.extend_from_slice(raw_box(source, header, "meta child")?);
        }

        if header.kind == HDLR {
            if saw_hdlr {
                return Err(invalid("canonical HEIF meta contains more than one hdlr"));
            }
            saw_hdlr = true;
            payload.extend_from_slice(&make_dinf_box()?);
        }
    }

    if !saw_hdlr {
        return Err(invalid(
            "canonical HEIF meta has no hdlr for DataInformation insertion",
        ));
    }
    if !saw_iloc {
        return Err(invalid("canonical HEIF meta has no iloc"));
    }

    make_box(META, &payload).map_err(|error| invalid(format!("canonical meta with dinf: {error}")))
}

fn shifted(value: u64, delta: i128, context: &str) -> Result<u64> {
    let shifted = i128::from(value)
        .checked_add(delta)
        .ok_or_else(|| invalid(format!("{context} overflows")))?;
    u64::try_from(shifted).map_err(|_| invalid(format!("{context} becomes negative or too large")))
}

fn relocated_iloc_entries(
    meta: &ParsedMeta,
    mdat: &BoxHeader,
    delta: i128,
) -> Result<Vec<IlocEntry>> {
    let mdat_start =
        u64::try_from(mdat.data_start).map_err(|_| invalid("mdat data offset exceeds u64"))?;
    let mdat_end =
        u64::try_from(mdat.data_end).map_err(|_| invalid("mdat end offset exceeds u64"))?;
    let mut entries = meta.iloc.entries.clone();

    for entry in &mut entries {
        if entry.construction_method != 0 {
            continue;
        }
        for extent in &mut entry.extents {
            let absolute = entry
                .base_offset
                .checked_add(extent.offset)
                .ok_or_else(|| {
                    invalid(format!("item {} extent offset overflows", entry.item_id))
                })?;
            let end = absolute
                .checked_add(extent.length)
                .ok_or_else(|| invalid(format!("item {} extent end overflows", entry.item_id)))?;
            if absolute < mdat_start || end > mdat_end {
                return Err(invalid(format!(
                    "item {} has file-backed data outside the primary mdat; DataInformation canonicalization refuses to guess",
                    entry.item_id
                )));
            }
            extent.offset = shifted(extent.offset, delta, "iloc file-backed extent offset")?;
        }
    }

    Ok(entries)
}

pub(crate) fn ensure_canonical_data_information_box(source: &[u8]) -> Result<Vec<u8>> {
    let top = scan_top_level_boxes(source)?;
    let meta_header = top
        .boxes
        .iter()
        .find(|header| header.kind == META)
        .ok_or_else(|| invalid("canonical HEIF output has no meta box"))?;
    let mdat = top
        .boxes
        .iter()
        .find(|header| header.kind == MDAT)
        .ok_or_else(|| invalid("canonical HEIF output has no mdat box"))?;
    let meta = parse_meta_box(source, meta_header)?;
    let meta_children_start = meta_header
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("meta child offset overflows"))?;
    let meta_children = parse_boxes(source, meta_children_start..meta_header.data_end)?;

    if meta_children.iter().any(|header| header.kind == DINF) {
        return Ok(source.to_vec());
    }

    let iloc_header = meta_children
        .iter()
        .find(|header| header.kind == ILOC)
        .ok_or_else(|| invalid("canonical HEIF output has no iloc box"))?;
    let original_iloc = raw_box(source, iloc_header, "iloc")?;
    let preliminary_meta =
        rebuild_meta_with_dinf(source, meta_header, &meta_children, original_iloc)?;

    let new_mdat_box_start = top
        .boxes
        .iter()
        .take_while(|header| header.box_start != mdat.box_start)
        .try_fold(0_usize, |offset, header| {
            let replacement_len = if header.kind == META {
                preliminary_meta.len()
            } else {
                header.size
            };
            offset
                .checked_add(replacement_len)
                .ok_or_else(|| invalid("canonical mdat file offset overflows"))
        })?;
    let mdat_header_size = mdat
        .data_start
        .checked_sub(mdat.box_start)
        .ok_or_else(|| invalid("mdat header geometry is invalid"))?;
    let new_mdat_data_start = new_mdat_box_start
        .checked_add(mdat_header_size)
        .ok_or_else(|| invalid("canonical mdat data offset overflows"))?;
    let delta = i128::try_from(new_mdat_data_start)
        .map_err(|_| invalid("new mdat data offset exceeds i128"))?
        - i128::try_from(mdat.data_start)
            .map_err(|_| invalid("old mdat data offset exceeds i128"))?;

    let relocated = relocated_iloc_entries(&meta, mdat, delta)?;
    let final_iloc = make_iloc_box(
        meta.iloc.version,
        meta.iloc.offset_size,
        meta.iloc.length_size,
        meta.iloc.base_offset_size,
        meta.iloc.index_size,
        &relocated,
    )
    .map_err(|error| invalid(format!("canonical relocated iloc: {error}")))?;
    let final_meta = rebuild_meta_with_dinf(source, meta_header, &meta_children, &final_iloc)?;
    if final_meta.len() != preliminary_meta.len() {
        return Err(invalid(
            "DataInformation iloc rewrite changed meta size unexpectedly",
        ));
    }

    let trailing = source
        .get(top.trailing_range.clone())
        .ok_or_else(|| invalid("top-level trailing bytes are outside source"))?;
    let output_capacity = top.boxes.iter().try_fold(trailing.len(), |total, header| {
        total
            .checked_add(if header.kind == META {
                final_meta.len()
            } else {
                header.size
            })
            .ok_or_else(|| invalid("canonical HEIF output size overflows"))
    })?;
    let mut output = Vec::with_capacity(output_capacity);
    for header in &top.boxes {
        if header.kind == META {
            output.extend_from_slice(&final_meta);
        } else {
            output.extend_from_slice(raw_box(source, header, "top-level")?);
        }
    }
    output.extend_from_slice(trailing);

    let reparsed_top = scan_top_level_boxes(&output)?;
    let reparsed_meta = reparsed_top
        .boxes
        .iter()
        .find(|header| header.kind == META)
        .ok_or_else(|| invalid("DataInformation rewrite lost meta box"))?;
    let children_start = reparsed_meta
        .data_start
        .checked_add(4)
        .ok_or_else(|| invalid("reparsed meta child offset overflows"))?;
    let children = parse_boxes(&output, children_start..reparsed_meta.data_end)?;
    if !children.iter().any(|header| header.kind == DINF) {
        return Err(invalid("DataInformation rewrite did not produce dinf"));
    }
    crate::validation::validate_gain_map_structure(&output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dinf_contains_one_self_contained_url_reference() {
        let dinf = make_dinf_box().unwrap();
        let dinf_header = parse_boxes(&dinf, 0..dinf.len()).unwrap().remove(0);
        assert_eq!(dinf_header.kind, DINF);

        let dref_header = parse_boxes(&dinf, dinf_header.payload_range())
            .unwrap()
            .remove(0);
        assert_eq!(dref_header.kind, DREF);
        assert_eq!(
            &dinf[dref_header.data_start..dref_header.data_start + 4],
            &[0, 0, 0, 0]
        );
        assert_eq!(
            &dinf[dref_header.data_start + 4..dref_header.data_start + 8],
            &1_u32.to_be_bytes()
        );

        let url_start = dref_header.data_start + 8;
        let url_header = parse_boxes(&dinf, url_start..dref_header.data_end)
            .unwrap()
            .remove(0);
        assert_eq!(url_header.kind, URL_);
        assert_eq!(
            &dinf[url_header.data_start..url_header.data_end],
            &[0, 0, 0, 1]
        );
    }
}
