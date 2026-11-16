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
        plan.warning = heic_motion_warning(target);
        if matches!(
            target,
            ConversionTarget::Oppo | ConversionTarget::Vivo | ConversionTarget::Xiaomi
        ) {
            plan.kept = HEIC_SDR_JPEG_KEPT;
            plan.transcoded = HEIC_SDR_JPEG_TRANSCODED;
            plan.dropped = HEIC_SDR_JPEG_DROPPED;
        }
    } else if target == ConversionTarget::Huawei {
        plan.blocked = Some(HUAWEI_NOT_HARMONY);
        plan.warning = HUAWEI_NOT_HARMONY;
    }
    plan
}

const HARMONY_HEIC_WARNING: &str = "huawei keeps this file only when it is already a HEIC still plus an appended MP4 with com.openharmony.covertime (big-endian float32 milliseconds) and com.openharmony.videoId. Those source fields, starttime, deferredVideoEnhanceFlag, and a v6 trailer are copied, not created. OpenHarmony photoAccessHelper creates a moving photo from a separate image resource and a separate video resource through MediaAssetChangeRequest; that API does not define this combined container. This route does not call it. BeetMan/XDRemux-Flutter reads this container and writes an Apple Live Photo; that reader is not a Harmony gallery writer. Harmony gallery import and playback of this file are not verified. The Android gallery 11.0.19.478 40-byte tail is not this output";
const HEIC_SDR_JPEG_KEPT: &[&str] = &[
    "preserved video bytes",
    "EXIF Make, Model, DateTime, DateTimeOriginal, and DateTimeDigitized when the source still has them",
];
const HEIC_SDR_JPEG_TRANSCODED: &[&str] = &[
    "HEIC primary becomes an 8-bit sRGB JPEG; this program applies one admitted sRGB or Display P3 ICC transform",
];
const HEIC_SDR_JPEG_DROPPED: &[&str] = &[
    "gain map is not applied and is not copied",
    "HDR display",
    "HEIF auxiliary items other than the primary still",
    "source Orientation value; irot is applied to the pixels and the JPEG Orientation is 1",
    "other EXIF fields, including GPS and MakerNote",
];
const SDR_JPEG_HEIC_WARNING: &str = "HEIC Motion Photo is not supported by the reviewed OPPO, vivo, or Xiaomi gallery builds, so this still is decoded and encoded as an 8-bit sRGB JPEG. This program applies one admitted sRGB or Display P3 ICC transform; libheif's output tag is not that proof. Display P3 nclx without an ICC uses the Display P3 profile. PQ, HLG, and other HDR transfers are refused instead of clamping code values. irot is applied to the pixels and JPEG Orientation is 1. Make, Model, and the date tags are copied when present; other EXIF is not. The gain map is not applied and is not copied, so HDR display is lost. Other HEIF auxiliary items are not copied. The source video bytes stay unchanged. Accepting this warning starts that JPEG encode";
const HUAWEI_NOT_HARMONY: &str = "This file is not the OpenHarmony HEIC motion container. --target huawei does not publish the Android gallery 11.0.19.478 JPEG plus 40-byte tail. videoId, deferredVideoEnhanceFlag, starttime, and the v6 trailer are not invented. Accepting the warning does not enable that output";

const SAMSUNG_HEIC_WARNING: &str = "Samsung gallery 15.1.03.4 code can read a top-level mpvd that runs to EOF. Opening the HEIC still is not the same as playing it as motion. SEF is left out. This file was not played on a device";
const STANDARD_HEIC_WARNING: &str = "android-standard, also selected by standard and google, keeps this HEIC and the source video bytes in Motion Photo 1.0. The specification allows that container and is not a player. You confirmed Google Photos played some HEIC motion samples and that preserved video played. That is consumer evidence for this standard target, not another target, and it does not name a Photos version or cover this file. Samsung 15.1.03.4 is the traced mpvd reader. A kept gain map is not HDR display";
const APPLE_HEIC_WARNING: &str = "This Apple route writes a HEIC still plus MOV for Photos live pairing. That pairing rule is not a statement that iOS cannot open an Android combined file. The movie is remuxed. A kept gain map is not HDR display. This pair was not played on a device in this change";

fn heic_motion_warning(target: ConversionTarget) -> &'static str {
    match target {
        ConversionTarget::Samsung => SAMSUNG_HEIC_WARNING,
        ConversionTarget::AndroidStandard => STANDARD_HEIC_WARNING,
        ConversionTarget::Apple => APPLE_HEIC_WARNING,
        ConversionTarget::Huawei => HARMONY_HEIC_WARNING,
        ConversionTarget::Oppo | ConversionTarget::Vivo | ConversionTarget::Xiaomi => {
            SDR_JPEG_HEIC_WARNING
        }
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
        SupportClaim { subject: "heic-motion", level: EvidenceLevel::CodeReachable, detail: "not supported in 17.10.62 and 25.17.35: JPEG MIME gate plus JpegOLiveReader SOI. ExoPlayer mpvd is not the olive player. Not every ColorOS version" },
    ],
    kept: &["JPEG still", "preserved video", "existing OpCamera fields", "gain map the JPEG writer keeps"],
    transcoded: &[],
    dropped: &["unmapped vendor tails the public JPEG container cannot express"],
    warning: "OPPO Photos 25.17.35 isOlivePhoto and 17.10.62 saq.w require image/jpeg, and JpegOLiveReader requires JPEG SOI. HEIC Motion Photo is not supported in these two builds. Existing OpCamera fields are copied. Sample hvc1 is file encoding only. A kept gain map is not HDR display. This file was not played on a device",
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
        SupportClaim { subject: "heic-motion", level: EvidenceLevel::CodeReachable, detail: "not supported in 10.0.10.42 (XmpUtil JPEG SOI, no mpvd; Y04s/W20 overseas proxy, not S60) or 3.2.10.4 (sidecar only, no combined fallback). Not every vivo version" },
    ],
    kept: &["JPEG still", "preserved video", "source VCamera fields", "gain-map XMP"],
    transcoded: &[],
    dropped: &["watermark EStream is not copied", "per-shot hash and firstFrame are not invented"],
    warning: "vivo 10.0.10.42 on the Y04s/W20 overseas proxy, not S60, locates combined JPEG motion with file size minus Item:Length when Padding is 0. HEIC Motion Photo is not supported in that build or in 3.2.10.4, which plays a sidecar and does not fall back to a combined file. Sample hvc1 is file encoding only",
    blocked: None,
    requires_confirmation: true,
};

const XIAOMI: TargetPreflight = TargetPreflight {
    destination: "xiaomi",
    player: "com.miui.gallery 4.3.1.15-global code; playback is not device-confirmed",
    claims: &[
        SupportClaim { subject: "still", level: EvidenceLevel::CodeReachable, detail: "JPEG Motion Photo. HEIC Motion Photo is not supported in 4.3.1.15-global: 1073 needs JPEG SOI, 1006/1071 are cleared without a positive readMicroMotionVideoOffset, and 1026 is a still HEIF mark. Not every Xiaomi version" },
        SupportClaim { subject: "motion", level: EvidenceLevel::CodeReachable, detail: "MotionPhoto>0, version>0, timestamp>=0; offset code requires version exactly 1" },
        SupportClaim { subject: "video", level: EvidenceLevel::CodeReachable, detail: "original video bytes; MicroVideo is not written" },
        SupportClaim { subject: "micamera", level: EvidenceLevel::Unknown, detail: "MiCamera is registered and was not read by the checked functions" },
    ],
    kept: &["JPEG still", "preserved video", "gain map the public writer keeps"],
    transcoded: &[],
    dropped: &["MiCamera and MicroVideo are not written"],
    warning: "Xiaomi Gallery 4.3.1.15-global does not support HEIC Motion Photo. XmpHelper for flag 1073 requires JPEG SOI, and 1006/1071/1073 are cleared when readMicroMotionVideoOffset is not positive. Flag 1026 is a still HEIF mark. This is not every Xiaomi version. Sample hvc1 is file encoding only. A kept gain map is not HDR display. This file was not played on a device",
    blocked: None,
    requires_confirmation: true,
};

const HUAWEI: TargetPreflight = TargetPreflight {
    destination: "huawei",
    player: "OpenHarmony HEIC plus appended MP4. Harmony gallery import is not verified. Android gallery 11.0.19.478 is not this output",
    claims: &[
        SupportClaim { subject: "still", level: EvidenceLevel::FormatStandard, detail: "HEIC prefix kept byte for byte, including HDR and auxiliary items already in the still" },
        SupportClaim { subject: "motion", level: EvidenceLevel::CodeReachable, detail: "appended MP4 whose moov carries com.openharmony.covertime and videoId. The Flutter fork reads this and writes an Apple pair; it does not prove gallery playback" },
        SupportClaim { subject: "covertime", level: EvidenceLevel::CodeReachable, detail: "big-endian float32 milliseconds copied from the source. It is not converted into the Android seekTo pair" },
        SupportClaim { subject: "trailer", level: EvidenceLevel::Unknown, detail: "a v6 trailer that contains the ASCII LIVE_ is kept with the video when present and is not parsed as the Android LIVE_ length" },
        SupportClaim { subject: "library", level: EvidenceLevel::CodeReachable, detail: "downstream photoAccessHelper selects the original HEIC. No moving-photo asset creation was found" },
    ],
    kept: &["HEIC still bytes", "appended MP4 bytes", "source openharmony fields", "source trailer bytes"],
    transcoded: &[],
    dropped: &["nothing is dropped from a file that already has this container"],
    warning: "huawei preserves an existing OpenHarmony HEIC motion file. videoId and the other capture fields are not invented for a different still. Harmony gallery import and playback are not verified",
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
        ] {
            let plan = target_preflight(target, true);
            assert!(plan.blocked.is_none());
            assert!(plan.requires_confirmation);
            assert!(plan.warning.contains("sRGB JPEG"));
            assert!(plan.warning.contains("gain map"));
            assert!(plan.warning.contains("not supported"));
            assert!(!plan.warning.contains("not established"));
            assert!(!plan.kept.iter().any(|item| item.contains("gain map")));
            assert!(plan
                .transcoded
                .iter()
                .any(|item| item.contains("sRGB JPEG")));
            assert!(plan.dropped.iter().any(|item| item.contains("gain map")));
            assert!(plan.dropped.iter().any(|item| item.contains("Orientation")));
            assert!(plan.kept.iter().any(|item| item.contains("DateTime")));
            let jpeg = target_preflight(target, false);
            assert!(jpeg.kept.iter().any(|item| item.contains("gain")));
            assert!(jpeg.transcoded.is_empty());
        }
        assert!(target_preflight(ConversionTarget::Oppo, false)
            .claims
            .iter()
            .any(|claim| {
                claim.level == EvidenceLevel::CodeReachable
                    && claim.detail.contains("not supported")
            }));
        assert!(target_preflight(ConversionTarget::Vivo, false)
            .claims
            .iter()
            .any(|claim| claim.subject == "heic-motion" && claim.detail.contains("not S60")));
        assert!(target_preflight(ConversionTarget::Xiaomi, false)
            .claims
            .iter()
            .any(|claim| claim.detail.contains("readMicroMotionVideoOffset")));
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
        let harmony = target_preflight(ConversionTarget::Huawei, true);
        assert!(harmony.blocked.is_none());
        assert!(harmony.warning.contains("com.openharmony.covertime"));
        assert!(harmony.warning.contains("not a Harmony gallery writer"));
        let jpeg = target_preflight(ConversionTarget::Huawei, false);
        let blocked = jpeg.blocked.unwrap();
        assert!(blocked.contains("11.0.19.478"));
        assert!(blocked.contains("not invented"));
        assert!(blocked.contains("does not enable"));
    }
}
