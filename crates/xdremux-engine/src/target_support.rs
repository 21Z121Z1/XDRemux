//! Playback support that has actually been traced. A container the product can
//! parse is not evidence that a gallery version will play it.

use crate::ConversionTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceLevel {
    /// A named device and gallery build played this output.
    DeviceConfirmed,
    /// A named gallery build's code reaches this reader or writer.
    CodeReachable,
    /// A public container or codec rule, with no named player.
    FormatStandard,
    /// Not established for this target.
    Unknown,
}

impl EvidenceLevel {
    pub const fn label(self) -> &'static str {
        match self {
            Self::DeviceConfirmed => "device-confirmed",
            Self::CodeReachable => "code-reachable",
            Self::FormatStandard => "format-standard",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupportClaim {
    pub subject: &'static str,
    pub level: EvidenceLevel,
    pub detail: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetPreflight {
    pub destination: &'static str,
    pub player: &'static str,
    pub claims: &'static [SupportClaim],
    pub kept: &'static [&'static str],
    pub transcoded: &'static [&'static str],
    pub dropped: &'static [&'static str],
    pub warning: &'static str,
    /// Missing conversion. Accepting the warning does not start this work.
    pub blocked: Option<&'static str>,
    pub requires_confirmation: bool,
}

pub fn target_preflight(target: ConversionTarget, still_is_heic: bool) -> TargetPreflight {
    let mut plan = match target {
        ConversionTarget::Apple => APPLE,
        ConversionTarget::AndroidStandard => ANDROID_STANDARD,
        ConversionTarget::Samsung => SAMSUNG,
        ConversionTarget::Oppo => OPPO,
        ConversionTarget::Vivo => VIVO,
        ConversionTarget::Xiaomi => XIAOMI,
        ConversionTarget::Huawei => HUAWEI,
    };
    if still_is_heic {
        if let Some(blocked) = heic_block(target) {
            plan.blocked = Some(blocked);
            plan.warning = blocked;
        } else if let Some(warning) = heic_motion_warning(target) {
            plan.warning = warning;
        }
    }
    plan
}

const SAMSUNG_HEIC_WARNING: &str = "Samsung gallery 15.1.03.4 code can read a top-level mpvd that runs to EOF. Opening the HEIC still is not the same as playing it as motion. SEF is left out. This file was not played on a device";
const STANDARD_HEIC_WARNING: &str = "android-standard, also selected by standard and google, keeps this HEIC and the source video bytes in Motion Photo 1.0. The specification allows that container and is not a player. You confirmed Google Photos played some HEIC motion samples and that preserved video played. That is consumer evidence for this standard target, not another target, and it does not name a Photos version or cover this file. Samsung 15.1.03.4 is the traced mpvd reader. A kept gain map is not HDR display";
const APPLE_HEIC_WARNING: &str = "This Apple route writes a HEIC still plus MOV for Photos live pairing. That pairing rule is not a statement that iOS cannot open an Android combined file. The movie is remuxed. A kept gain map is not HDR display. This pair was not played on a device in this change";

fn heic_motion_warning(target: ConversionTarget) -> Option<&'static str> {
    match target {
        ConversionTarget::Samsung => Some(SAMSUNG_HEIC_WARNING),
        ConversionTarget::AndroidStandard => Some(STANDARD_HEIC_WARNING),
        ConversionTarget::Apple => Some(APPLE_HEIC_WARNING),
        _ => None,
    }
}

fn heic_block(target: ConversionTarget) -> Option<&'static str> {
    match target {
        ConversionTarget::Oppo => Some(
            "17.10.62 saq.w and 25.17.35 isOlivePhoto have no reached HEIF olive path. That limit is these two builds, not every future ColorOS version. Publication stops before still re-encoding. TranscodeRequired: a JPEG Motion Photo would drop or rewrite HDR and auxiliary resources. Accepting the warning does not enable that conversion",
        ),
        ConversionTarget::Vivo => Some(
            "10.0.10.42 combined playback that was traced is JPEG plus an appended MP4. HEIC live playback is not established for that Y04s/W20 package. Publication stops before still re-encoding. TranscodeRequired: a JPEG still would drop or rewrite HDR and auxiliary resources. Accepting the warning does not enable that conversion",
        ),
        ConversionTarget::Xiaomi => Some(
            "4.3.1.15-global XMP parsing requires a JPEG SOI, so this HEIC misses that path. The camera note that HEIF mode cannot capture a live photo is not a claim that every Xiaomi version refuses imported HEIC live playback. This package has no second proven HEIC live path. Publication stops before still re-encoding. TranscodeRequired: a JPEG still would drop or rewrite HDR and auxiliary resources. Accepting the warning does not enable that conversion",
        ),
        ConversionTarget::Huawei => Some(
            "11.0.19.478 live location is a trailing video plus a 40-byte tail. HEIC live playback was not traced. Publication stops before still re-encoding. TranscodeRequired: a JPEG still would drop or rewrite HDR and auxiliary resources. Accepting the warning does not enable that conversion",
        ),
        _ => None,
    }
}

const APPLE: TargetPreflight = TargetPreflight {
    destination: "apple",
    player: "Apple Photos; no device pass is recorded for this build",
    claims: &[
        SupportClaim { subject: "still", level: EvidenceLevel::CodeReachable, detail: "HEIC still for a Live Photo pair" },
        SupportClaim { subject: "motion", level: EvidenceLevel::CodeReachable, detail: "paired MOV; the product remuxes the movie" },
        SupportClaim { subject: "video", level: EvidenceLevel::CodeReachable, detail: "MOV pair member, not the original MP4 bytes" },
        SupportClaim { subject: "photos-live", level: EvidenceLevel::CodeReachable, detail: "Photos live recognition pairs a still with a MOV using a content identifier and still-image time. That does not mean the system cannot open an Android combined file as a still or a movie" },
        SupportClaim { subject: "hdr-display", level: EvidenceLevel::Unknown, detail: "a kept gain map is not proof Photos shows HDR live" },
    ],
    kept: &["source still resources the existing pair writer retains", "presentation time"],
    transcoded: &["movie container becomes MOV, so the original MP4 bytes are not the pair video"],
    dropped: &["resources the existing Apple pair writer does not carry"],
    warning: "This Apple route writes a still plus MOV. Photos live recognition is that pair, not a statement that iOS rejects every Android combined file. The movie is remuxed. A kept gain map is not HDR display. This pair was not played on a device in this change",
    blocked: None,
    requires_confirmation: true,
};

const ANDROID_STANDARD: TargetPreflight = TargetPreflight {
    destination: "android-standard",
    player: "Motion Photo 1.0. standard and google select this same target. The specification is not a player",
    claims: &[
        SupportClaim { subject: "still", level: EvidenceLevel::FormatStandard, detail: "JPEG or HEIC primary, matching the source still. The specification also names AVIF; this writer does not publish AVIF" },
        SupportClaim { subject: "motion", level: EvidenceLevel::FormatStandard, detail: "Motion Photo 1.0, Padding 0, no MicroVideoVersion" },
        SupportClaim { subject: "video", level: EvidenceLevel::CodeReachable, detail: "original video bytes preserved in one copy" },
        SupportClaim { subject: "google-photos", level: EvidenceLevel::DeviceConfirmed, detail: "you confirmed Google Photos played some HEIC motion samples and that preserved video played. No Photos version was named. This checkout does not contain the historical gphotos-re tree, so that confirmation is not re-derived as APK control flow. It is consumer evidence for this standard target, not a second target, and it does not cover every file this writer emits" },
        SupportClaim { subject: "video-codec", level: EvidenceLevel::Unknown, detail: "hvc1 in checked samples is the file codec, not proof every device plays HEVC, H.264, AV1, or HDR video" },
        SupportClaim { subject: "hdr-display", level: EvidenceLevel::Unknown, detail: "a kept gain map is not HDR display" },
    ],
    kept: &["source still container", "preserved video bytes", "gain-map XMP the public writer keeps"],
    transcoded: &[],
    dropped: &["vendor fields that are not part of the public container"],
    warning: "android-standard, also selected by standard and google, writes Motion Photo 1.0 and keeps the source video bytes. The specification allows the container and is not a player. Your Google Photos confirmation covers some HEIC samples and preserved video, not this file and not a named Photos version. Sample hvc1 is file encoding only. A kept gain map is not HDR display",
    blocked: None,
    requires_confirmation: true,
};

const SAMSUNG: TargetPreflight = TargetPreflight {
    destination: "samsung",
    player: "com.sec.android.gallery3d 15.1.03.4 code; playback is not device-confirmed",
    claims: &[
        SupportClaim { subject: "still", level: EvidenceLevel::CodeReachable, detail: "JPEG XMP fallback, and HEIC with a normal top-level mpvd last" },
        SupportClaim { subject: "motion", level: EvidenceLevel::CodeReachable, detail: "XMP Motion Photo after SEF; Padding must be present and is 0" },
        SupportClaim { subject: "video", level: EvidenceLevel::CodeReachable, detail: "primary video bytes preserved; length must be greater than 1024" },
        SupportClaim { subject: "sef", level: EvidenceLevel::Unknown, detail: "SEF MotionPhoto_Data is preferred by that build and is not written" },
    ],
    kept: &["primary still", "primary video", "gain map the public writer keeps"],
    transcoded: &[],
    dropped: &["SEF records", "a secondary autoplay stream when it cannot be expressed"],
    warning: "Samsung gallery 15.1.03.4 code reads SEF MotionPhoto_Data first, then JPEG XMP when Item:Length is greater than 1024 and Padding is present. This writer leaves SEF out and writes Padding 0. Opening a HEIC still is not the same as playing motion. Sample hvc1 is file encoding only. A kept gain map is not HDR display. This file was not played on a device",
    blocked: None,
    requires_confirmation: true,
};

const OPPO: TargetPreflight = TargetPreflight {
    destination: "oppo",
    player: "com.coloros.gallery3d 25.17.35 and 17.10.62 code; playback is not device-confirmed",
    claims: &[
        SupportClaim { subject: "still", level: EvidenceLevel::CodeReachable, detail: "image/jpeg only; isOlivePhoto and saq.w both require it" },
        SupportClaim { subject: "motion", level: EvidenceLevel::CodeReachable, detail: "MotionPhoto=1 plus a non-empty container; OpCamera is copied only when the source has it" },
        SupportClaim { subject: "video", level: EvidenceLevel::CodeReachable, detail: "original video bytes; offset is file size minus Item:Length when Padding is 0" },
        SupportClaim { subject: "heif", level: EvidenceLevel::Unknown, detail: "neither package shows an HEIF olive path" },
    ],
    kept: &["JPEG still", "preserved video", "existing OpCamera fields", "gain map the JPEG writer keeps"],
    transcoded: &[],
    dropped: &["unmapped vendor tails the public JPEG container cannot express"],
    warning: "OPPO gallery 25.17.35 isOlivePhoto and 17.10.62 saq.w require image/jpeg before the olive reader. Existing OpCamera fields are copied. Sample hvc1 is file encoding only. A kept gain map is not HDR display. This file was not played on a device",
    blocked: None,
    requires_confirmation: true,
};

const VIVO: TargetPreflight = TargetPreflight {
    destination: "vivo",
    player: "com.vivo.gallery 10.0.10.42 Y04s/W20 code, and 3.2.10.4 sidecar code; neither playback is device-confirmed",
    claims: &[
        SupportClaim { subject: "still", level: EvidenceLevel::CodeReachable, detail: "JPEG combined file; samples with both UUID boxes stay byte for byte" },
        SupportClaim { subject: "motion", level: EvidenceLevel::CodeReachable, detail: "10.0.10.42 Path.w combined playback uses file size minus Item:Length when Padding is 0" },
        SupportClaim { subject: "video", level: EvidenceLevel::CodeReachable, detail: "preserved MP4; ExtInfo type and imageTime sit before the video" },
        SupportClaim { subject: "sidecar", level: EvidenceLevel::CodeReachable, detail: "3.2.10.4 and Path.x use a separate MP4; this target does not emit that scheme" },
    ],
    kept: &["JPEG still", "preserved video", "source VCamera fields", "gain-map XMP"],
    transcoded: &[],
    dropped: &["watermark EStream is not copied", "per-shot hash and firstFrame are not invented"],
    warning: "vivo 10.0.10.42 on the Y04s/W20 package locates combined JPEG motion with file size minus Item:Length when Padding is 0. 3.2.10.4 sidecar playback is a separate scheme and is not written. This is not an S60/W10 device pass. Sample hvc1 is file encoding only",
    blocked: None,
    requires_confirmation: true,
};

const XIAOMI: TargetPreflight = TargetPreflight {
    destination: "xiaomi",
    player: "com.miui.gallery 4.3.1.15-global code; playback is not device-confirmed",
    claims: &[
        SupportClaim { subject: "still", level: EvidenceLevel::CodeReachable, detail: "JPEG Motion Photo; HEIC playback is not established for this build" },
        SupportClaim { subject: "motion", level: EvidenceLevel::CodeReachable, detail: "MotionPhoto>0, version>0, timestamp>=0; offset code requires version exactly 1" },
        SupportClaim { subject: "video", level: EvidenceLevel::CodeReachable, detail: "original video bytes; MicroVideo is not written" },
        SupportClaim { subject: "micamera", level: EvidenceLevel::Unknown, detail: "MiCamera is registered and was not read by the checked functions" },
    ],
    kept: &["JPEG still", "preserved video", "gain map the public writer keeps"],
    transcoded: &[],
    dropped: &["MiCamera and MicroVideo are not written"],
    warning: "Xiaomi gallery 4.3.1.15-global XMP parsing requires a JPEG SOI for the checked recognition path. The camera HEIF capture limit is not a claim that every Xiaomi version refuses imported HEIC live playback. Sample hvc1 is file encoding only. A kept gain map is not HDR display. This file was not played on a device",
    blocked: None,
    requires_confirmation: true,
};

const HUAWEI: TargetPreflight = TargetPreflight {
    destination: "huawei",
    player: "com.android.gallery3d 11.0.19.478 code; playback is not device-confirmed",
    claims: &[
        SupportClaim { subject: "still", level: EvidenceLevel::CodeReachable, detail: "one file whose video is a trailing range" },
        SupportClaim { subject: "motion", level: EvidenceLevel::CodeReachable, detail: "40-byte tail; play info is seek and duration in truncated milliseconds" },
        SupportClaim { subject: "video", level: EvidenceLevel::CodeReachable, detail: "original video bytes located by LIVE_<length>" },
        SupportClaim { subject: "heif", level: EvidenceLevel::Unknown, detail: "the traced tail is not an HEIC motion reader" },
    ],
    kept: &["bytes before the video", "preserved video"],
    transcoded: &[],
    dropped: &["sub-millisecond remainder of the presentation time and mvhd duration", "optional vN_fM tag"],
    warning: "Huawei gallery 11.0.19.478 locates a trailing video with a 40-byte tail and seeks using truncated milliseconds. No Huawei motion fixture records a video codec. This file was not played on a device",
    blocked: None,
    requires_confirmation: true,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_target_is_device_confirmed() {
        for target in [
            ConversionTarget::Apple,
            ConversionTarget::AndroidStandard,
            ConversionTarget::Samsung,
            ConversionTarget::Oppo,
            ConversionTarget::Vivo,
            ConversionTarget::Xiaomi,
            ConversionTarget::Huawei,
        ] {
            let plan = target_preflight(target, false);
            assert!(plan.requires_confirmation);
            assert!(!plan.warning.contains("本项目无用户真机确认"));
            if target != ConversionTarget::AndroidStandard {
                assert!(plan
                    .claims
                    .iter()
                    .all(|claim| claim.level != EvidenceLevel::DeviceConfirmed));
            }
        }
        let standard = target_preflight(ConversionTarget::AndroidStandard, true);
        assert!(standard.warning.contains("Google Photos"));
        assert!(standard.warning.contains("preserved video"));
        assert!(standard.warning.contains("not a player"));
        assert!(standard.blocked.is_none());
        let apple = target_preflight(ConversionTarget::Apple, false);
        assert!(apple.warning.contains("Android combined file"));
        assert!(!apple.warning.contains("whole system"));
        assert!(apple
            .claims
            .iter()
            .any(|claim| claim.detail.contains("cannot open")));
    }

    #[test]
    fn heic_motion_is_not_forced_onto_untraced_galleries() {
        for target in [
            ConversionTarget::Oppo,
            ConversionTarget::Vivo,
            ConversionTarget::Xiaomi,
            ConversionTarget::Huawei,
        ] {
            let plan = target_preflight(target, true);
            let blocked = plan.blocked.unwrap();
            assert!(blocked.contains("TranscodeRequired"));
            assert!(blocked.contains("does not enable"));
        }
        assert!(target_preflight(ConversionTarget::AndroidStandard, true)
            .blocked
            .is_none());
        assert!(target_preflight(ConversionTarget::Samsung, true)
            .blocked
            .is_none());
        let standard = target_preflight(ConversionTarget::AndroidStandard, true);
        assert!(standard.blocked.is_none());
        assert!(standard.player.contains("not a player"));
        assert!(standard.warning.contains("Google Photos"));
    }
}
