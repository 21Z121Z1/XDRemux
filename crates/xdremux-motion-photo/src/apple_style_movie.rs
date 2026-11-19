//! XDRemux's derived Apple Styles overlay on a canonical Live Photo MOV.
use xdremux_format::isobmff::{make_box, parse_boxes, BoxHeader};
use xdremux_format::{FormatError, FourCC};

type Result<T> = std::result::Result<T, FormatError>;
const CONTEXT: &str = "Live Photo Styles MOV";
const THUMBNAIL_NAME: &str = "com.apple.quicktime.video-map.smart-style-linear-thumbnail";
const STYLE_NAME: &str = "com.apple.quicktime.smartstyle-info";
const TEXTURE_NAME: &str = "com.apple.quicktime.texturestyle-info";

pub struct AppleLiveStyleFrames<'a> {
    pub track_id: u32,
    pub timescale: u32,
    /// The display-order timestamps, followed by the exclusive end time.
    pub boundaries: &'a [u64],
    pub width: u16,
    pub height: u16,
    pub hvcc: &'a [u8],
    pub thumbnail_samples: &'a [Vec<u8>],
    pub style_properties: &'a [Vec<u8>],
    pub texture_properties: &'a [u8],
    pub aac: &'a [AppleLiveStyleAac],
}

pub struct AppleLiveStyleAac {
    pub track_id: u32,
    pub frames_per_packet: u32,
    pub channel_count: u32,
}

fn invalid(detail: impl Into<String>) -> FormatError {
    FormatError::invalid(CONTEXT, detail)
}

fn four(name: &[u8; 4]) -> FourCC {
    FourCC::new(*name)
}
fn wrap(name: &[u8; 4], data: &[u8]) -> Result<Vec<u8>> {
    make_box(four(name), data)
}
fn u32_at(data: &[u8], at: usize) -> Result<u32> {
    let bytes = data
        .get(at..at + 4)
        .ok_or_else(|| invalid("truncated integer"))?;
    Ok(u32::from_be_bytes(bytes.try_into().unwrap()))
}
fn count_usize(size: usize) -> Result<u32> {
    u32::try_from(size).map_err(|_| FormatError::overflow(CONTEXT))
}
fn push32(data: &mut Vec<u8>, value: u32) {
    data.extend_from_slice(&value.to_be_bytes());
}
fn box_child<'a>(boxes: &'a [BoxHeader], kind: &[u8; 4]) -> Result<&'a BoxHeader> {
    let mut found = boxes.iter().filter(|header| header.kind == four(kind));
    let first = found
        .next()
        .ok_or_else(|| invalid(format!("missing {:?} box", four(kind))))?;
    if found.next().is_some() {
        return Err(invalid("duplicate movie box"));
    }
    Ok(first)
}

fn track_id(data: &[u8]) -> Result<u32> {
    match data.first() {
        Some(0) => u32_at(data, 12),
        Some(1) => u32_at(data, 20),
        _ => Err(invalid("unsupported track header version")),
    }
}

fn handler(component: &[u8; 4], subtype: &[u8; 4], label: &[u8]) -> Result<Vec<u8>> {
    let mut data = vec![0; 4];
    data.extend_from_slice(component);
    data.extend_from_slice(subtype);
    data.extend_from_slice(b"appl");
    data.extend_from_slice(&[0; 8]);
    data.push(label.len() as u8);
    data.extend_from_slice(label);
    wrap(b"hdlr", &data)
}

fn table(kind: &[u8; 4], words: impl IntoIterator<Item = u32>) -> Result<Vec<u8>> {
    let mut data = vec![0; 4];
    for value in words {
        push32(&mut data, value);
    }
    wrap(kind, &data)
}

fn sample_tables(entry: &[u8], deltas: &[u32], sizes: &[u32], offset: u64) -> Result<Vec<u8>> {
    let mut stsd = vec![0; 4];
    push32(&mut stsd, 1);
    stsd.extend_from_slice(entry);
    let mut result = wrap(b"stsd", &stsd)?;
    let groups = deltas.chunk_by(|a, b| a == b);
    let mut stts = vec![0; 4];
    push32(&mut stts, count_usize(groups.clone().count())?);
    for group in groups {
        push32(&mut stts, count_usize(group.len())?);
        push32(&mut stts, group[0]);
    }
    result.extend_from_slice(&wrap(b"stts", &stts)?);
    let size_count = count_usize(sizes.len())?;
    result.extend_from_slice(&table(b"stsc", [1, 1, size_count, 1])?);
    result.extend_from_slice(&table(
        b"stsz",
        [0, size_count].into_iter().chain(sizes.iter().copied()),
    )?);
    if let Ok(offset) = u32::try_from(offset) {
        result.extend_from_slice(&table(b"stco", [1, offset])?);
    } else {
        let mut co64 = vec![0; 4];
        push32(&mut co64, 1);
        co64.extend_from_slice(&offset.to_be_bytes());
        result.extend_from_slice(&wrap(b"co64", &co64)?);
    }
    wrap(b"stbl", &result)
}

fn log_sample_entry(frames: &AppleLiveStyleFrames<'_>) -> Result<Vec<u8>> {
    let mut entry = vec![0; 78];
    entry[7] = 1;
    entry[24..26].copy_from_slice(&frames.width.to_be_bytes());
    entry[26..28].copy_from_slice(&frames.height.to_be_bytes());
    entry[28..36].copy_from_slice(&[0, 72, 0, 0, 0, 72, 0, 0]);
    entry[41] = 1;
    entry[74..78].copy_from_slice(&[0, 24, 255, 255]);
    entry.extend_from_slice(&wrap(b"hvcC", frames.hvcc)?);
    // Apple Log's Rec.2020 sample entry includes its own transfer identifier.
    entry.extend_from_slice(&wrap(b"colr", b"nclc\0\x09\0\x01\0\x09")?);
    entry.extend_from_slice(&wrap(b"logs", b"com.apple.rec2020.apple-log")?);
    wrap(b"hvc1", &entry)
}

fn metadata_sample_entry(name: &str) -> Result<Vec<u8>> {
    let mut key_value = b"mdta".to_vec();
    key_value.extend_from_slice(name.as_bytes());
    let mut format = wrap(b"keyd", &key_value)?;
    format.extend_from_slice(&wrap(b"dtyp", &[0; 8])?);
    format.extend_from_slice(&wrap(b"sdpd", &wrap(b"sdpi", &[0; 4])?)?);
    let local_key = make_box(FourCC::new(1_u32.to_be_bytes()), &format)?;
    let mut entry = vec![0; 8];
    entry[7] = 1;
    entry.extend_from_slice(&wrap(b"keys", &local_key)?);
    wrap(b"mebx", &entry)
}

struct StyleTrack<'a> {
    header: &'a [u8],
    id: u32,
    main_id: u32,
    timing: &'a AppleLiveStyleFrames<'a>,
    deltas: &'a [u32],
}

impl StyleTrack<'_> {
    fn pack(&self, entry: &[u8], sizes: &[u32], offset: u64, auxiliary: bool) -> Result<Vec<u8>> {
        let mut tkhd = self.header.to_vec();
        let (id_at, dimensions_at) = match tkhd.first() {
            Some(0) => (12, 76),
            Some(1) => (20, 88),
            _ => return Err(invalid("unsupported track header")),
        };
        if tkhd.len() != dimensions_at + 8 {
            return Err(invalid("invalid track header length"));
        }
        tkhd[1..4].copy_from_slice(&[0, 0, if auxiliary { 14 } else { 15 }]);
        tkhd[id_at..id_at + 4].copy_from_slice(&self.id.to_be_bytes());
        let dimensions = if auxiliary {
            [self.timing.width, self.timing.height]
        } else {
            [0, 0]
        };
        let (stored_dimensions, _) = tkhd[dimensions_at..].as_chunks_mut::<4>();
        for (target, dimension) in stored_dimensions.iter_mut().zip(dimensions) {
            *target = (u32::from(dimension) << 16).to_be_bytes();
        }
        let mut data = wrap(b"tkhd", &tkhd)?;
        let media = self.main_id.to_be_bytes();
        if auxiliary {
            let mut aperture = Vec::new();
            for k in [b"clef", b"prof", b"enof"] {
                aperture.extend_from_slice(&table(k, dimensions.map(|d| u32::from(d) << 16))?);
            }
            data.extend_from_slice(&wrap(b"tapt", &aperture)?);
            data.extend_from_slice(&wrap(b"tref", &wrap(b"vmap", &media)?)?);
        } else {
            let mut references = wrap(b"cdsc", &media)?;
            references.extend_from_slice(&wrap(b"cdep", &media)?);
            data.extend_from_slice(&wrap(b"tref", &references)?);
        }
        let duration = *self
            .timing
            .boundaries
            .last()
            .ok_or_else(|| invalid("missing duration"))?;
        let duration = u32::try_from(duration)
            .map_err(|_| invalid("video duration exceeds the media header"))?;
        let mut mdia = table(b"mdhd", [0, 0, self.timing.timescale, duration, 0x55c40000])?;
        let mut minf = if auxiliary {
            mdia.extend_from_slice(&handler(b"mhlr", b"auxv", b"Core Media Auxiliary Picture")?);
            wrap(b"vmhd", &[0, 0, 0, 1, 0, 64, 128, 0, 128, 0, 128, 0])?
        } else {
            mdia.extend_from_slice(&handler(b"mhlr", b"meta", b"Core Media Metadata")?);
            wrap(
                b"gmhd",
                &wrap(
                    b"gmin",
                    &[0, 0, 0, 0, 0, 64, 128, 0, 128, 0, 128, 0, 0, 0, 0, 0],
                )?,
            )?
        };
        minf.extend_from_slice(&handler(b"dhlr", b"alis", b"Core Media Data Handler")?);
        let mut dref = vec![0; 4];
        push32(&mut dref, 1);
        dref.extend_from_slice(&wrap(b"alis", &[0, 0, 0, 1])?);
        minf.extend_from_slice(&wrap(b"dinf", &wrap(b"dref", &dref)?)?);
        minf.extend_from_slice(&sample_tables(entry, self.deltas, sizes, offset)?);
        mdia.extend_from_slice(&wrap(b"minf", &minf)?);
        data.extend_from_slice(&wrap(b"mdia", &mdia)?);
        if auxiliary {
            data.extend_from_slice(&wrap(b"udta", &wrap(b"tagc", THUMBNAIL_NAME.as_bytes())?)?);
        }
        wrap(b"trak", &data)
    }
}

fn append_styles_metadata(data: &[u8], meta: &BoxHeader) -> Result<Vec<u8>> {
    let children = parse_boxes(data, meta.payload_range())?;
    let keys_box = box_child(&children, b"keys")?;
    let items_box = box_child(&children, b"ilst")?;
    let original_keys = &data[keys_box.payload_range()];
    let mut keys = original_keys.to_vec();
    let original_count = u32_at(&keys, 4)?;
    let parsed = parse_boxes(data, keys_box.data_start + 8..keys_box.data_end)?;
    if usize::try_from(original_count).ok() != Some(parsed.len()) {
        return Err(invalid("invalid keys count"));
    }
    let mut items = data[items_box.payload_range()].to_vec();
    let neutral = 0f32.to_be_bytes();
    let full = 1f32.to_be_bytes();
    let version = 1u32.to_be_bytes();
    let standard = 1u64.to_be_bytes();
    let fields: [(&str, u32, &[u8]); 10] = [
        ("smartstyle.rendering-version", 21, &version),
        ("smartstyle.tone", 23, &neutral),
        ("smartstyle.color", 23, &neutral),
        ("smartstyle.intensity", 23, &full),
        ("smartstyle.bypassed", 21, &[1]),
        ("smartstyle.cast", 21, &standard),
        ("texturestyle.rendering-version", 21, &version),
        ("texturestyle.preset", 21, &standard),
        ("texturestyle.intensity", 23, &full),
        ("texturestyle.grain", 23, &neutral),
    ];
    let mut index = original_count;
    for (key, format, value) in fields {
        let name = format!("com.apple.quicktime.{key}");
        if parsed
            .iter()
            .any(|header| data[header.payload_range()] == *name.as_bytes())
        {
            return Err(invalid("input already has style rendering metadata"));
        }
        keys.extend_from_slice(&wrap(b"mdta", name.as_bytes())?);
        index = index
            .checked_add(1)
            .ok_or_else(|| FormatError::overflow(CONTEXT))?;
        let mut item = format.to_be_bytes().to_vec();
        item.extend_from_slice(&[0; 4]);
        item.extend_from_slice(value);
        items.extend_from_slice(&make_box(
            FourCC::new(index.to_be_bytes()),
            &wrap(b"data", &item)?,
        )?);
    }
    keys[4..8].copy_from_slice(&index.to_be_bytes());
    let mut rebuilt = Vec::new();
    for item in children {
        match item.kind.as_bytes() {
            b"keys" => rebuilt.extend_from_slice(&wrap(b"keys", &keys)?),
            b"ilst" => rebuilt.extend_from_slice(&wrap(b"ilst", &items)?),
            _ => rebuilt.extend_from_slice(&data[item.box_range()]),
        }
    }
    wrap(b"meta", &rebuilt)
}

// The Photos saver needs a version-1 QuickTime sound description to pass AAC
// through with its magic cookie. Only the sample description changes.
fn quicktime_aac_track(
    data: &[u8],
    track: &BoxHeader,
    format: &AppleLiveStyleAac,
) -> Result<Option<Vec<u8>>> {
    let children = parse_boxes(data, track.payload_range())?;
    let mdia = box_child(&children, b"mdia")?;
    let media = parse_boxes(data, mdia.payload_range())?;
    let handler = &data[box_child(&media, b"hdlr")?.payload_range()];
    if handler.get(8..12) != Some(b"soun") {
        return Ok(None);
    }
    let minf = box_child(&media, b"minf")?;
    let minf_children = parse_boxes(data, minf.payload_range())?;
    let stbl = box_child(&minf_children, b"stbl")?;
    let tables = parse_boxes(data, stbl.payload_range())?;
    let stsd = box_child(&tables, b"stsd")?;
    let descriptions = &data[stsd.payload_range()];
    if descriptions.len() < 8 {
        return Err(invalid("truncated audio sample description"));
    }
    let entries = parse_boxes(data, stsd.data_start + 8..stsd.data_end)?;
    let mut replacement = descriptions[..8].to_vec();
    let mut changed = false;
    for entry in entries {
        let body = &data[entry.payload_range()];
        if entry.kind != four(b"mp4a")
            || body.get(8..10) != Some(&[0, 0])
            || body.get(18..20) != Some(&[0, 16])
        {
            replacement.extend_from_slice(&data[entry.box_range()]);
            continue;
        }
        let extensions = parse_boxes(data, entry.data_start + 28..entry.data_end)?;
        let esds = extensions
            .iter()
            .find(|extension| extension.kind == four(b"esds"));
        let has_wave = extensions
            .iter()
            .any(|extension| extension.kind == four(b"wave"));
        match (has_wave, esds) {
            (false, Some(esds)) => {
                let mut wave = wrap(b"frma", b"mp4a")?;
                wave.extend_from_slice(&wrap(b"mp4a", &[0; 4])?);
                wave.extend_from_slice(&data[esds.box_range()]);
                wave.extend_from_slice(&wrap(b"\0\0\0\0", &[])?);
                let mut version_one = body[..28].to_vec();
                let channels = u16::try_from(format.channel_count)
                    .map_err(|_| invalid("AAC channel count exceeds the sample description"))?;
                version_one[8..10].copy_from_slice(&1u16.to_be_bytes());
                version_one[16..18].copy_from_slice(&channels.to_be_bytes());
                version_one[20..22].copy_from_slice(&(-2i16).to_be_bytes());
                let bytes_per_sample = 2;
                let bytes_per_frame = format
                    .channel_count
                    .checked_mul(bytes_per_sample)
                    .ok_or_else(|| FormatError::overflow(CONTEXT))?;
                for word in [
                    format.frames_per_packet,
                    0,
                    bytes_per_frame,
                    bytes_per_sample,
                ] {
                    push32(&mut version_one, word);
                }
                version_one.extend_from_slice(&data[entry.data_start + 28..esds.box_start]);
                version_one.extend_from_slice(&wrap(b"wave", &wave)?);
                version_one.extend_from_slice(&data[esds.data_end..entry.data_end]);
                replacement.extend_from_slice(&wrap(b"mp4a", &version_one)?);
                changed = true;
            }
            _ => replacement.extend_from_slice(&data[entry.box_range()]),
        }
    }
    if !changed {
        return Ok(None);
    }
    let mut rebuilt = wrap(b"stsd", &replacement)?;
    for (parent, child) in [(stbl, stsd), (minf, stbl), (mdia, minf), (track, mdia)] {
        rebuilt = replace_one_child(data, parent, child, &rebuilt)?;
    }
    Ok(Some(rebuilt))
}

fn replace_one_child(
    data: &[u8],
    parent: &BoxHeader,
    child: &BoxHeader,
    bytes: &[u8],
) -> Result<Vec<u8>> {
    let payload = [
        &data[parent.data_start..child.box_start],
        bytes,
        &data[child.data_end..parent.data_end],
    ]
    .concat();
    make_box(parent.kind, &payload)
}

/// Preserve the canonical movie's original media bytes and track offsets.
/// These new tracks describe an unstyled motion complement. Each displayed
/// frame has its own valid reference even when it cannot use matte interpolation.
pub fn add_apple_live_style_tracks(
    movie: &[u8],
    frames: &AppleLiveStyleFrames<'_>,
) -> Result<Vec<u8>> {
    let count = frames.thumbnail_samples.len();
    if count == 0
        || frames.timescale == 0
        || frames.boundaries.len() != count + 1
        || frames.boundaries.first() != Some(&0)
        || frames.style_properties.len() != count
        || frames.width == 0
        || frames.height == 0
        || frames.hvcc.is_empty()
    {
        return Err(invalid(
            "inconsistent style samples or unsupported nonzero start",
        ));
    }
    let deltas = frames
        .boundaries
        .windows(2)
        .map(|range| {
            range[1]
                .checked_sub(range[0])
                .filter(|value| *value > 0)
                .and_then(|value| u32::try_from(value).ok())
                .ok_or_else(|| invalid("invalid presentation sample duration"))
        })
        .collect::<Result<Vec<_>>>()?;
    let top = parse_boxes(movie, 0..movie.len())?;
    let moov = box_child(&top, b"moov")?;
    if moov.data_end != movie.len() {
        return Err(invalid("expected the canonical movie header at the end"));
    }
    let children = parse_boxes(movie, moov.payload_range())?;
    let meta = box_child(&children, b"meta")?;
    let header = box_child(&children, b"mvhd")?;
    let mut mvhd = movie[header.payload_range()].to_vec();
    let next_id_at = mvhd
        .len()
        .checked_sub(4)
        .ok_or_else(|| invalid("truncated movie header"))?;
    let mut max_id = 0;
    let mut main = None;
    let mut quicktime_audio = Vec::new();
    for track in children
        .iter()
        .filter(|header| header.kind == four(b"trak"))
    {
        let children = parse_boxes(movie, track.payload_range())?;
        let tkhd = &movie[box_child(&children, b"tkhd")?.payload_range()];
        let id = track_id(tkhd)?;
        max_id = max_id.max(id);
        if let Some(format) = frames.aac.iter().find(|format| format.track_id == id) {
            if let Some(rebuilt) = quicktime_aac_track(movie, track, format)? {
                quicktime_audio.push((track.box_start, rebuilt));
            }
        }
        if id == frames.track_id {
            let mdia = box_child(&children, b"mdia")?;
            let media = parse_boxes(movie, mdia.payload_range())?;
            let handler = &movie[box_child(&media, b"hdlr")?.payload_range()];
            if handler.get(8..12) != Some(b"vide") {
                return Err(invalid("decoded track is not a primary video track"));
            }
            main = Some(tkhd);
        }
    }
    let main = main.ok_or_else(|| invalid("decoded source track is not in the movie"))?;
    let mut id = max_id
        .checked_add(1)
        .ok_or_else(|| FormatError::overflow(CONTEXT))?;
    let mut payload = Vec::new();
    let mut added = Vec::new();
    #[derive(Clone, Copy)]
    enum Resource {
        Thumbnail,
        SemanticStyle,
        TextureStyle,
    }
    for (kind, entry) in [
        (Resource::Thumbnail, log_sample_entry(frames)?),
        (Resource::SemanticStyle, metadata_sample_entry(STYLE_NAME)?),
        (Resource::TextureStyle, metadata_sample_entry(TEXTURE_NAME)?),
    ] {
        let offset = u64::try_from(moov.box_start).map_err(|_| FormatError::overflow(CONTEXT))?
            + 8
            + payload.len() as u64;
        let mut sizes = Vec::with_capacity(count);
        for index in 0..count {
            let sample = match kind {
                Resource::Thumbnail => &frames.thumbnail_samples[index],
                Resource::SemanticStyle => &frames.style_properties[index],
                Resource::TextureStyle => frames.texture_properties,
            };
            if sample.is_empty() {
                return Err(invalid("empty style sample"));
            }
            let mut size = count_usize(sample.len())?;
            if !matches!(kind, Resource::Thumbnail) {
                if !sample.starts_with(b"bplist00") {
                    return Err(invalid("style property list is not binary"));
                }
                size = size
                    .checked_add(8)
                    .ok_or_else(|| FormatError::overflow(CONTEXT))?;
                push32(&mut payload, size);
                push32(&mut payload, 1);
            }
            payload.extend_from_slice(sample);
            sizes.push(size);
        }
        added.extend_from_slice(
            &StyleTrack {
                header: main,
                id,
                main_id: frames.track_id,
                timing: frames,
                deltas: &deltas,
            }
            .pack(&entry, &sizes, offset, matches!(kind, Resource::Thumbnail))?,
        );
        id = id
            .checked_add(1)
            .ok_or_else(|| FormatError::overflow(CONTEXT))?;
    }
    mvhd[next_id_at..].copy_from_slice(&id.to_be_bytes());
    let mut new_header = Vec::new();
    for atom in &children {
        if atom.kind == four(b"meta") {
            new_header.extend_from_slice(&append_styles_metadata(movie, meta)?);
        } else if atom.kind == four(b"mvhd") {
            new_header.extend_from_slice(&wrap(b"mvhd", &mvhd)?);
        } else if let Some((_, rebuilt)) = quicktime_audio
            .iter()
            .find(|(start, _)| *start == atom.box_start)
        {
            new_header.extend_from_slice(rebuilt);
        } else {
            new_header.extend_from_slice(&movie[atom.box_range()]);
        }
    }
    // Keep the original track order. Some consumers select the first video
    // track without interpreting the auxiliary handler.
    new_header.extend_from_slice(&added);
    let overlay = wrap(b"mdat", &payload)?;
    if overlay.len() != payload.len() + 8 {
        return Err(invalid("unexpected extended overlay header"));
    }
    let result = [
        &movie[..moov.box_start],
        overlay.as_slice(),
        &wrap(b"moov", &new_header)?,
    ]
    .concat();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn child(data: &[u8], parent: &BoxHeader, kind: &[u8; 4]) -> BoxHeader {
        box_child(&parse_boxes(data, parent.payload_range()).unwrap(), kind)
            .unwrap()
            .clone()
    }

    #[test]
    fn quicktime_aac_preserves_decoder_config_and_sample_tables() {
        let esds = wrap(b"esds", b"decoder configuration").unwrap();
        let mut sound = vec![0; 28];
        sound[7] = 1;
        sound[16..20].copy_from_slice(&[0, 1, 0, 16]);
        sound.extend_from_slice(&esds);
        let entry = wrap(b"mp4a", &sound).unwrap();
        let tables = sample_tables(&entry, &[1024, 1024, 960], &[60, 50, 70], 4096).unwrap();
        let minf = wrap(b"minf", &tables).unwrap();
        let mut media = handler(b"mhlr", b"soun", b"SoundHandler").unwrap();
        media.extend(minf);
        let original = wrap(b"trak", &wrap(b"mdia", &media).unwrap()).unwrap();
        let track = |data: &[u8]| parse_boxes(data, 0..data.len()).unwrap().remove(0);
        let format = AppleLiveStyleAac {
            track_id: 1,
            frames_per_packet: 1024,
            channel_count: 1,
        };
        let rebuilt = quicktime_aac_track(&original, &track(&original), &format)
            .unwrap()
            .unwrap();
        let table_box = |data: &[u8]| {
            let media = child(data, &track(data), b"mdia");
            let minf = child(data, &media, b"minf");
            child(data, &minf, b"stbl")
        };
        let before = table_box(&original);
        let after = table_box(&rebuilt);
        for kind in [b"stts", b"stsc", b"stsz", b"stco"] {
            assert_eq!(
                original[child(&original, &before, kind).box_range()],
                rebuilt[child(&rebuilt, &after, kind).box_range()]
            );
        }
        let stsd = child(&rebuilt, &after, b"stsd");
        let [entry] = &parse_boxes(&rebuilt, stsd.data_start + 8..stsd.data_end).unwrap()[..]
        else {
            panic!("expected one audio sample description");
        };
        let body = &rebuilt[entry.payload_range()];
        assert_eq!(&body[8..10], &1_u16.to_be_bytes());
        assert_eq!(&body[20..22], &(-2_i16).to_be_bytes());
        let packet_fields = [1024_u32, 0, 2, 2].map(u32::to_be_bytes).concat();
        assert_eq!(&body[28..44], packet_fields);
        let [wave] = &parse_boxes(&rebuilt, entry.data_start + 44..entry.data_end).unwrap()[..]
        else {
            panic!("expected one wave box");
        };
        assert_eq!(wave.kind, four(b"wave"));
        let config = child(&rebuilt, wave, b"esds");
        assert_eq!(rebuilt[config.box_range()], esds);
        assert!(quicktime_aac_track(&rebuilt, &track(&rebuilt), &format)
            .unwrap()
            .is_none());
    }
}
