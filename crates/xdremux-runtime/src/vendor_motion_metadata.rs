//! Carry vendor XMP that is already present on the source into a public
//! Motion Photo. This module does not invent gallery fields from APK strings.

use xdremux_engine::ConversionTarget;

use crate::{Result, RuntimeError};

const OPLUS_CAMERA: &str = "http://ns.oplus.com/photos/1.0/camera/";

/// What a destination may copy. A namespace of `None` means no vendor attributes
/// are added, even when the source has them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VendorMetadataCarry {
    pub destination: &'static str,
    pub container: &'static str,
    pub preserve_namespace: Option<&'static str>,
    /// Leave out resources the public container cannot express, and name them.
    /// Still, primary video, and gain-map images are never in that set.
    pub leave_out_unmapped: bool,
    pub gaps: &'static [&'static str],
}

impl VendorMetadataCarry {
    pub const fn legacy() -> Self {
        Self {
            destination: "motion-target",
            container: "public-motion-photo-1.0",
            preserve_namespace: None,
            leave_out_unmapped: false,
            gaps: &["legacy --motion-target uses the public Motion Photo writer"],
        }
    }

    pub const fn from_target(target: ConversionTarget) -> Self {
        match target {
            ConversionTarget::Apple => Self {
                destination: "apple",
                container: "apple-live-photo-pair",
                preserve_namespace: None,
                leave_out_unmapped: false,
                gaps: &["Apple output is a HEIC+MOV pair, not an Android Motion Photo"],
            },
            ConversionTarget::AndroidStandard => Self {
                destination: "android-standard",
                container: "public-motion-photo-1.0",
                preserve_namespace: None,
                leave_out_unmapped: false,
                gaps: &[
                    "android-standard selects the public Motion Photo container, not a vendor profile",
                    "gallery recognition is not a device result",
                ],
            },
            ConversionTarget::Google => Self {
                destination: "google",
                container: "public-motion-photo-1.0",
                preserve_namespace: None,
                leave_out_unmapped: false,
                gaps: &[
                    "Google Motion Photo 1.0 writer; a previous device report is not re-proven by this build",
                    "gallery recognition is not a device result",
                ],
            },
            ConversionTarget::Samsung => Self {
                destination: "samsung",
                container: "public-motion-photo-1.0",
                preserve_namespace: None,
                leave_out_unmapped: true,
                gaps: &[
                    "Samsung gallery 15.1.03.4 checks SEF MotionPhoto_Data before XMP; this output does not rewrite SEF, so only the XMP fallback can apply",
                    "for a non-JPEG name, ex.a.d scans top-level boxes for mpvd and takes the payload through EOF; mpvd is last and uses a normal 32-bit size",
                    "that XMP fallback needs Item:Padding present and Item:Length greater than 1024; Padding is 0",
                    "a secondary MotionPhoto_AutoPlay stream is left out when it cannot be expressed",
                    "the HEIF box locator was not fully traced; this is not a Samsung-native or device claim",
                ],
            },
            ConversionTarget::Oppo => Self {
                destination: "oppo",
                container: "public-motion-photo-1.0",
                preserve_namespace: Some(OPLUS_CAMERA),
                leave_out_unmapped: true,
                gaps: &[
                    "OPPO gallery 25.17.35 JPEG olive recognizes GCamera:MotionPhoto=1; a non-empty Container is required to decode",
                    "OpCamera fields are read but are not the recognition gate; they are copied only when the source XMP already has them",
                    "GCamera:MicroVideoVersion makes that parser return null; it is not written",
                    "Item:Padding is parsed and is not part of the clear single-still offset formula; Padding=0 keeps the Samsung XMP fallback aligned",
                    "17.10.62 saq.w and 25.17.35 isOlivePhoto both require image/jpeg; neither package shows a HEIF olive path",
                    "gallery playback and EXIF display are not device-verified",
                ],
            },
            ConversionTarget::Vivo => Self {
                destination: "vivo",
                container: "vivo-preserved-source-layout",
                preserve_namespace: None,
                leave_out_unmapped: false,
                gaps: &[
                    "a JPEG whose MP4 already has vivoMediaEStream and vivoMediaExtInfo is published unchanged, including GCamera, VCamera, the optional EOI gap, and mp42/hvc1",
                    "application/livePhoto-meta is absent on sdr-01 and present on sdr-02, so it is kept only when already present",
                    "the repeated sample live-photo token and EStream digest are not synthesized",
                    "3.2.10.4 has no GCamera or VCamera strings in this package; that does not show the player ignores a file or that another source can never be written",
                    "hash, firstFrame, and version for a new file are not established; long-press playback consumption is still being traced",
                    "MediaExtra live_photo import is a device check, and gallery playback is not device-verified",
                ],
            },
            ConversionTarget::Xiaomi => Self {
                destination: "xiaomi",
                container: "public-motion-photo-1.0",
                preserve_namespace: None,
                leave_out_unmapped: true,
                gaps: &[
                    "Xiaomi gallery 4.3.1.15-global isGoogleMotionPhoto requires MotionPhoto>0, MotionPhotoVersion>0 and MotionPhotoPresentationTimestampUs>=0",
                    "MiCamera is registered and was not read by the checked recognition or offset function; it is not written",
                    "when MicroVideo is not 1, version is exactly 1, and Directory has at least two items, the offset is file size minus the MotionPhoto item Length; Padding is not read",
                    "MicroVideo is not written, so that priority branch is not taken; OPPO 25.17.35 also rejects MicroVideoVersion",
                    "xiaomi gallery playback is not device-verified",
                ],
            },
            ConversionTarget::Huawei => Self {
                destination: "huawei",
                container: "huawei-live-tail-40",
                preserve_namespace: None,
                leave_out_unmapped: false,
                gaps: &[
                    "LivePlayer.startPreview stores readPlayInfo in VideoTextureView.preview; play seeks to the first int and the first frame update pauses after the second int",
                    "the ints are milliseconds: start is the source presentation time truncated for seekTo, and time is the preserved mvhd duration truncated for the auto-stop delay",
                    "a blank field does not parse and a zero duration does not preview; DEFAULT_COVER_FRAME is not written",
                    "an optional 20-byte vN_fM tag is not written",
                    "gallery playback is not device-verified",
                ],
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorMetadataReport {
    pub destination: String,
    pub container: String,
    pub native_vendor_claim: bool,
    pub gallery_verified: bool,
    pub carried_attributes: Vec<String>,
    pub namespace_absent: bool,
    pub not_carried: Vec<String>,
    pub gaps: Vec<String>,
    pub exif_identifier_present: bool,
    pub video_byte_length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct VendorAttribute {
    prefix: String,
    local: String,
    value: String,
}

/// Copy already-present vendor attributes into `output`. Video bytes stay a
/// single preserved suffix or `mpvd` payload; this function checks that the
/// caller-supplied video length still describes one range after the edit.
pub(crate) fn carry_vendor_metadata(
    marker_source: &[u8],
    source_xmp: Option<&[u8]>,
    output: &mut Vec<u8>,
    video_len: usize,
    carry: VendorMetadataCarry,
) -> Result<VendorMetadataReport> {
    let mut not_carried = marker_gaps(marker_source, output);
    let mut carried = Vec::new();
    let mut namespace_absent = false;
    if let Some(namespace) = carry.preserve_namespace {
        let raw_packet = xmp_packet_containing(marker_source, namespace.as_bytes());
        let packet = raw_packet.or(source_xmp);
        match packet {
            Some(packet) => {
                let source_packet = std::str::from_utf8(packet)
                    .map_err(|_| RuntimeError::new("vendor metadata", "source XMP is not UTF-8"))?;
                let attributes = vendor_attributes(source_packet, namespace)?;
                if attributes.is_empty() {
                    namespace_absent = true;
                    not_carried.push(format!(
                        "source XMP has no attributes in {namespace}; none were invented"
                    ));
                } else {
                    let outcome =
                        splice_vendor_attributes(output, namespace, &attributes, video_len)?;
                    match outcome {
                        SpliceOutcome::Inserted(names) => carried = names,
                        SpliceOutcome::HeifItemCannotGrow => {
                            not_carried.push(
                                "HEIF XMP item cannot grow in place; source vendor attributes were not copied and were not invented"
                                    .to_owned(),
                            );
                        }
                    }
                }
            }
            None => {
                namespace_absent = true;
                not_carried.push(
                    "source has no XMP packet; vendor attributes were not invented".to_owned(),
                );
            }
        }
    }
    Ok(VendorMetadataReport {
        destination: carry.destination.to_owned(),
        container: carry.container.to_owned(),
        native_vendor_claim: false,
        gallery_verified: false,
        carried_attributes: carried,
        namespace_absent,
        not_carried,
        gaps: carry.gaps.iter().map(|gap| (*gap).to_owned()).collect(),
        exif_identifier_present: exif_identifier_present(output),
        video_byte_length: u64::try_from(video_len).unwrap_or(u64::MAX),
    })
}

fn xmp_packet_containing<'a>(bytes: &'a [u8], needle: &[u8]) -> Option<&'a [u8]> {
    let mut search = 0usize;
    while let Some(start) = bytes[search..]
        .windows(10)
        .position(|window| window == b"<x:xmpmeta")
        .map(|offset| search + offset)
    {
        let end_rel = bytes[start..]
            .windows(12)
            .position(|window| window == b"</x:xmpmeta>")?;
        let end = start + end_rel + 12;
        let packet = &bytes[start..end];
        if packet.windows(needle.len()).any(|window| window == needle) {
            return Some(packet);
        }
        search = end;
    }
    None
}

fn marker_gaps(source: &[u8], output: &[u8]) -> Vec<String> {
    const MARKERS: &[(&[u8], &str)] = &[
        (b"cameralbum!", "vivo cameralbum trailer"),
        (b"jxrs", "OPPO jxrs trailer"),
        (b"SEFT", "Samsung SEFT trailer"),
        (b"XIAOMI_CUSTOMIZE", "Xiaomi customize segment"),
        (b"lpex", "OPPO lpex extension"),
    ];
    MARKERS
        .iter()
        .filter(|(marker, _)| source.windows(marker.len()).any(|window| window == *marker))
        .filter(|(marker, _)| !output.windows(marker.len()).any(|window| window == *marker))
        .map(|(_, label)| {
            format!(
                "{label} is present on the source and absent from the output; it was not rewritten"
            )
        })
        .collect()
}

enum SpliceOutcome {
    Inserted(Vec<String>),
    HeifItemCannotGrow,
}

fn splice_vendor_attributes(
    output: &mut Vec<u8>,
    namespace: &str,
    attributes: &[VendorAttribute],
    video_len: usize,
) -> Result<SpliceOutcome> {
    let packet_range = unique_motion_packet(output)?;
    let old_packet = output[packet_range.clone()].to_vec();
    let old_text = std::str::from_utf8(&old_packet)
        .map_err(|_| RuntimeError::new("vendor metadata", "output XMP is not UTF-8"))?;
    let timestamp = output_timestamp_us(old_text);
    let mut current = old_text.to_owned();
    let mut insertion = String::new();
    let mut names = Vec::new();
    for attribute in attributes {
        let declaration = format!(" xmlns:{}=\"{namespace}\"", attribute.prefix);
        let mut value = attribute.value.clone();
        if attribute.local == "VideoLength" {
            value = video_len.to_string();
        } else if attribute.local.ends_with("PresentationTimestampUs") {
            if let Some(timestamp) = &timestamp {
                value = timestamp.clone();
            }
        }
        if !safe_token(&value) {
            return Err(RuntimeError::new(
                "vendor metadata",
                format!(
                    "refusing to copy {} because its value is not a short token",
                    attribute.local
                ),
            ));
        }
        let key = format!("{}:{}=\"", attribute.prefix, attribute.local);
        if let Some(start) = current.find(&key) {
            let value_at = start + key.len();
            let Some(end_rel) = current[value_at..].find('"') else {
                return Err(RuntimeError::new(
                    "vendor metadata",
                    "existing vendor attribute is missing its closing quote",
                ));
            };
            let value_end = value_at + end_rel;
            if &current[value_at..value_end] != value {
                current.replace_range(value_at..value_end, &value);
            }
        } else {
            if !current.contains(&declaration) && !insertion.contains(&declaration) {
                insertion.push_str(&declaration);
            }
            insertion.push_str(&format!(
                " {}:{}=\"{value}\"",
                attribute.prefix, attribute.local
            ));
        }
        names.push(format!("{}:{}", attribute.prefix, attribute.local));
    }
    if insertion.is_empty() && current == old_text {
        return Ok(SpliceOutcome::Inserted(names));
    }
    let new_packet = if insertion.is_empty() {
        current
    } else {
        insert_before_motion_tag(&current, &insertion)?
    };
    if new_packet.len() == old_packet.len() {
        output[packet_range].copy_from_slice(new_packet.as_bytes());
        return Ok(SpliceOutcome::Inserted(names));
    }
    if is_heif(output) {
        if new_packet.len() > old_packet.len() {
            return Ok(SpliceOutcome::HeifItemCannotGrow);
        }
        let padded = pad_xmp_packet(&new_packet, old_packet.len())?;
        output[packet_range].copy_from_slice(padded.as_bytes());
        return Ok(SpliceOutcome::Inserted(names));
    }
    replace_jpeg_xmp_segment(output, &packet_range, new_packet.as_bytes())?;
    Ok(SpliceOutcome::Inserted(names))
}

fn vendor_attributes(packet: &str, namespace: &str) -> Result<Vec<VendorAttribute>> {
    let mut found = Vec::new();
    let mut search = 0usize;
    let declared = format!("=\"{namespace}\"");
    while let Some(relative) = packet[search..].find(&declared) {
        let index = search + relative;
        let Some(xmlns) = packet[..index].rfind("xmlns:") else {
            search = index + declared.len();
            continue;
        };
        let prefix = &packet[xmlns + "xmlns:".len()..index];
        if prefix.is_empty()
            || !prefix
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            search = index + declared.len();
            continue;
        }
        let mut cursor = 0usize;
        let token = format!("{prefix}:");
        while let Some(hit) = packet[cursor..].find(&token) {
            let start = cursor + hit;
            if packet[..start].ends_with("xmlns:") {
                cursor = start + token.len();
                continue;
            }
            let after = &packet[start + token.len()..];
            let Some(name_end) = after.find("=\"") else {
                break;
            };
            let local = &after[..name_end];
            if local.is_empty()
                || !local
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            {
                cursor = start + token.len();
                continue;
            }
            let value_start = start + token.len() + name_end + 2;
            let Some(value_end) = packet[value_start..].find('"') else {
                break;
            };
            let value = &packet[value_start..value_start + value_end];
            if safe_token(value) {
                let name = format!("{prefix}:{local}");
                if !found
                    .iter()
                    .any(|item: &VendorAttribute| format!("{}:{}", item.prefix, item.local) == name)
                {
                    found.push(VendorAttribute {
                        prefix: prefix.to_owned(),
                        local: local.to_owned(),
                        value: value.to_owned(),
                    });
                }
            }
            cursor = value_start + value_end;
        }
        search = index + declared.len();
    }
    Ok(found)
}

fn safe_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'+' | b'-'))
}

fn output_timestamp_us(packet: &str) -> Option<String> {
    const KEY: &str = "MotionPhotoPresentationTimestampUs=\"";
    let mut rest = packet;
    while let Some(index) = rest.find(KEY) {
        let value = &rest[index + KEY.len()..];
        let end = value.find('"')?;
        let token = &value[..end];
        if token.bytes().all(|byte| byte.is_ascii_digit()) {
            let before = rest[..index].as_bytes();
            let primary = before.last().is_none_or(|byte| !byte.is_ascii_alphabetic());
            if primary {
                return Some(token.to_owned());
            }
        }
        rest = &rest[index + KEY.len()..];
    }
    None
}

fn insert_before_motion_tag(packet: &str, insertion: &str) -> Result<String> {
    const KEY: &str = "MotionPhoto=\"1\"";
    let index = packet.find(KEY).ok_or_else(|| {
        RuntimeError::new(
            "vendor metadata",
            "output XMP has no MotionPhoto marker to attach source attributes to",
        )
    })?;
    let start = packet[..index].rfind('<').ok_or_else(|| {
        RuntimeError::new(
            "vendor metadata",
            "MotionPhoto marker is not inside an element",
        )
    })?;
    let end = packet[index..]
        .find('>')
        .map(|offset| index + offset)
        .ok_or_else(|| RuntimeError::new("vendor metadata", "MotionPhoto element is unclosed"))?;
    let insert_at = if packet.as_bytes()[end - 1] == b'/' {
        end - 1
    } else {
        end
    };
    if start >= insert_at {
        return Err(RuntimeError::new(
            "vendor metadata",
            "MotionPhoto element is malformed",
        ));
    }
    let mut updated = String::with_capacity(packet.len() + insertion.len());
    updated.push_str(&packet[..insert_at]);
    updated.push_str(insertion);
    updated.push_str(&packet[insert_at..]);
    Ok(updated)
}

fn unique_motion_packet(output: &[u8]) -> Result<std::ops::Range<usize>> {
    let mut found = None;
    let mut search = 0;
    while let Some(start) = output[search..]
        .windows(10)
        .position(|window| window == b"<x:xmpmeta")
        .map(|offset| search + offset)
    {
        let Some(end_rel) = output[start..]
            .windows(12)
            .position(|window| window == b"</x:xmpmeta>")
        else {
            return Err(RuntimeError::new(
                "vendor metadata",
                "output XMP packet is truncated",
            ));
        };
        let end = start + end_rel + 12;
        let packet = &output[start..end];
        if packet
            .windows(15)
            .any(|window| window == b"MotionPhoto=\"1\"")
        {
            if found.is_some() {
                return Err(RuntimeError::new(
                    "vendor metadata",
                    "output has more than one Motion Photo XMP packet",
                ));
            }
            found = Some(start..end);
        }
        search = end;
    }
    found.ok_or_else(|| {
        RuntimeError::new("vendor metadata", "output has no Motion Photo XMP packet")
    })
}

fn is_heif(output: &[u8]) -> bool {
    output.len() > 12 && &output[4..8] == b"ftyp"
}

fn pad_xmp_packet(packet: &str, len: usize) -> Result<String> {
    const CLOSE: &str = "</x:xmpmeta>";
    if packet.len() > len || !packet.ends_with(CLOSE) {
        return Err(RuntimeError::new(
            "vendor metadata",
            "HEIF XMP packet cannot be padded",
        ));
    }
    let pad = len - packet.len();
    let mut padded = String::with_capacity(len);
    padded.push_str(&packet[..packet.len() - CLOSE.len()]);
    padded.extend(std::iter::repeat(' ').take(pad));
    padded.push_str(CLOSE);
    Ok(padded)
}

fn replace_jpeg_xmp_segment(
    output: &mut Vec<u8>,
    packet: &std::ops::Range<usize>,
    new_packet: &[u8],
) -> Result<()> {
    if output.first() != Some(&0xff) || output.get(1) != Some(&0xd8) {
        return Err(RuntimeError::new(
            "vendor metadata",
            "vendor XMP growth is implemented for JPEG APP1 and in-place HEIF packets",
        ));
    }
    let mut cursor = 2usize;
    while cursor + 4 <= output.len() && output[cursor] == 0xff {
        let marker = output[cursor + 1];
        if marker == 0xd8 || marker == 0xd9 || marker == 0x00 {
            cursor += 2;
            continue;
        }
        if marker == 0xda {
            break;
        }
        let segment_len = u16::from_be_bytes([output[cursor + 2], output[cursor + 3]]) as usize;
        if segment_len < 2 || cursor + 2 + segment_len > output.len() {
            return Err(RuntimeError::new(
                "vendor metadata",
                "JPEG segment length is outside the file",
            ));
        }
        let segment_end = cursor + 2 + segment_len;
        if packet.start >= cursor && packet.end <= segment_end {
            let payload_start = cursor + 4;
            let old_payload_len = segment_end - payload_start;
            let packet_offset = packet.start - payload_start;
            let mut payload = output[payload_start..segment_end].to_vec();
            payload.splice(
                packet_offset..packet_offset + (packet.end - packet.start),
                new_packet.iter().copied(),
            );
            if payload.len() > 65533 || payload.len() < old_payload_len.saturating_sub(packet.len())
            {
                return Err(RuntimeError::new(
                    "vendor metadata",
                    "rewritten JPEG XMP segment does not fit in one APP segment",
                ));
            }
            let mut rebuilt = Vec::with_capacity(output.len() - old_payload_len + payload.len());
            rebuilt.extend_from_slice(&output[..cursor]);
            rebuilt.push(0xff);
            rebuilt.push(marker);
            let length = (payload.len() + 2) as u16;
            rebuilt.extend_from_slice(&length.to_be_bytes());
            rebuilt.extend_from_slice(&payload);
            rebuilt.extend_from_slice(&output[segment_end..]);
            *output = rebuilt;
            return Ok(());
        }
        cursor = segment_end;
    }
    Err(RuntimeError::new(
        "vendor metadata",
        "Motion Photo XMP is not inside a JPEG segment",
    ))
}

pub(crate) fn exif_identifier_present(bytes: &[u8]) -> bool {
    let mut search = 0;
    while let Some(offset) = bytes[search..]
        .windows(6)
        .position(|window| window == b"Exif\0\0")
    {
        let at = search + offset + 6;
        if bytes
            .get(at..at + 2)
            .is_some_and(|mark| mark == b"II" || mark == b"MM")
        {
            return true;
        }
        search += offset + 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg_with_xmp(packet: &str, tail: &[u8]) -> Vec<u8> {
        let ident = b"http://ns.adobe.com/xap/1.0/\0";
        let mut payload = ident.to_vec();
        payload.extend_from_slice(packet.as_bytes());
        let mut bytes = vec![0xff, 0xd8, 0xff, 0xe1];
        bytes.extend_from_slice(&(payload.len() as u16 + 2).to_be_bytes());
        bytes.extend_from_slice(&payload);
        bytes.extend_from_slice(&[0xff, 0xd9]);
        bytes.extend_from_slice(tail);
        bytes
    }

    #[test]
    fn vivo_carry_does_not_copy_vcamera_and_keeps_one_video_suffix() {
        let source = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:Description xmlns:VCamera="http://ns.vivo.com/photos/1.0/camera/" VCamera:VMotionPhotoVersion="1" VCamera:VMotionPhotoSource="1" VCamera:VMediaKitVersion="1.0.0.7"/></x:xmpmeta>"#;
        let output_packet = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:Description xmlns:Camera="http://ns.google.com/photos/1.0/camera/" Camera:MotionPhoto="1" Camera:MotionPhotoPresentationTimestampUs="1333944"></rdf:Description></x:xmpmeta>"#;
        let tail = b"ONE-VIDEO";
        let mut output = jpeg_with_xmp(output_packet, tail);
        let report = carry_vendor_metadata(
            b"cameralbum! in source only",
            Some(source.as_bytes()),
            &mut output,
            tail.len(),
            VendorMetadataCarry::from_target(ConversionTarget::Vivo),
        )
        .unwrap();
        assert!(output.ends_with(tail));
        let text = String::from_utf8_lossy(&output);
        assert!(!text.contains("VCamera:"));
        assert!(report.carried_attributes.is_empty());
        assert!(report
            .not_carried
            .iter()
            .any(|item| item.contains("cameralbum")));
        assert!(!report.native_vendor_claim);
        assert_eq!(
            output
                .windows(tail.len())
                .filter(|window| *window == tail)
                .count(),
            1
        );
    }

    #[test]
    fn oppo_video_length_follows_the_preserved_video_and_is_not_invented() {
        let source = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:Description xmlns:OpCamera="http://ns.oplus.com/photos/1.0/camera/" OpCamera:MotionPhotoOwner="oplus" OpCamera:OLivePhotoVersion="2" OpCamera:VideoLength="10" OpCamera:MotionPhotoPrimaryPresentationTimestampUs="1"/></x:xmpmeta>"#;
        let output_packet = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:Description Camera:MotionPhoto="1" Camera:MotionPhotoPresentationTimestampUs="1469600"/></x:xmpmeta>"#;
        let mut output = jpeg_with_xmp(output_packet, b"0123456789ABCDEF");
        carry_vendor_metadata(
            source.as_bytes(),
            Some(source.as_bytes()),
            &mut output,
            16,
            VendorMetadataCarry::from_target(ConversionTarget::Oppo),
        )
        .unwrap();
        let text = String::from_utf8_lossy(&output);
        assert!(text.contains("OpCamera:VideoLength=\"16\""));
        assert!(text.contains("OpCamera:MotionPhotoPrimaryPresentationTimestampUs=\"1469600\""));
        assert!(text.contains("OpCamera:MotionPhotoOwner=\"oplus\""));
    }

    #[test]
    fn missing_vendor_namespace_does_not_invent_fields() {
        let output_packet = r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:Description Camera:MotionPhoto="1" Camera:MotionPhotoPresentationTimestampUs="1"/></x:xmpmeta>"#;
        let original = jpeg_with_xmp(output_packet, b"VIDEO");
        let mut output = original.clone();
        let report = carry_vendor_metadata(
            b"plain",
            Some(br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:Description Camera:MotionPhoto="1"/></x:xmpmeta>"#),
            &mut output,
            5,
            VendorMetadataCarry::from_target(ConversionTarget::Oppo),
        )
        .unwrap();
        assert!(report.namespace_absent);
        assert!(report.carried_attributes.is_empty());
        assert_eq!(output, original);
        assert!(report
            .not_carried
            .iter()
            .any(|item| item.contains("not invented") || item.contains("none were invented")));
    }
}
