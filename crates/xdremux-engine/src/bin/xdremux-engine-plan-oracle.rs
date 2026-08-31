use std::env;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Write};

use serde::{Deserialize, Serialize};
use xdremux_engine::{
    resolve_effective_input_processing_branch, InputProcessingBranch, OppoCameraTail, TmapFormat,
};

#[derive(Debug, Deserialize)]
struct PlanCase {
    name: String,
    family: Option<String>,
    oppo_compatibility: Option<String>,
    input_processing_branch: Option<String>,
    oppo_camera_tail: Option<String>,
    tmap_format: Option<String>,
    apple_photographic_styles: Option<bool>,
    apple_portrait: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NormalizedPlan {
    name: String,
    family: String,
    oppo_compatibility: String,
    requested_input_processing_branch: String,
    effective_input_processing_branch: String,
    oppo_camera_tail: String,
    tmap_format: String,
    apple_feature_route: String,
}

fn invalid_value(case_name: &str, field: &str, value: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid {field} '{value}' in case {case_name}"),
    )
}

fn parse_family(value: Option<&str>, case_name: &str) -> io::Result<String> {
    let value = value.unwrap_or("auto");
    match value {
        "auto" | "x6" | "x7" => Ok(value.to_owned()),
        _ => Err(invalid_value(case_name, "family", value)),
    }
}

fn parse_oppo_compatibility(value: Option<&str>, case_name: &str) -> io::Result<String> {
    let value = value.unwrap_or("off");
    match value {
        "auto" | "iso" | "iso-no-local" | "iso-graph" | "on" | "tail" | "off" => {
            Ok(value.to_owned())
        }
        _ => Err(invalid_value(case_name, "oppo_compatibility", value)),
    }
}

fn parse_input_processing_branch(
    value: Option<&str>,
    case_name: &str,
) -> io::Result<InputProcessingBranch> {
    let value = value.unwrap_or("hybrid");
    match value {
        "system" => Ok(InputProcessingBranch::System),
        "system-decoded" => Ok(InputProcessingBranch::SystemDecoded),
        "hybrid" => Ok(InputProcessingBranch::Hybrid),
        "passthrough" => Ok(InputProcessingBranch::Passthrough),
        _ => Err(invalid_value(case_name, "input_processing_branch", value)),
    }
}

fn input_processing_branch_name(value: InputProcessingBranch) -> &'static str {
    match value {
        InputProcessingBranch::System => "system",
        InputProcessingBranch::SystemDecoded => "system-decoded",
        InputProcessingBranch::Hybrid => "hybrid",
        InputProcessingBranch::Passthrough => "passthrough",
    }
}

fn parse_oppo_camera_tail(value: Option<&str>, case_name: &str) -> io::Result<OppoCameraTail> {
    let value = value.unwrap_or("preserve-without-private-hdr");
    match value {
        "off" => Ok(OppoCameraTail::Off),
        "watermark" => Ok(OppoCameraTail::Watermark),
        "compact" => Ok(OppoCameraTail::Compact),
        "preserve" => Ok(OppoCameraTail::Preserve),
        "preserve-without-portrait" => Ok(OppoCameraTail::PreserveWithoutPortrait),
        "preserve-without-portrait-or-private-hdr" => {
            Ok(OppoCameraTail::PreserveWithoutPortraitOrPrivateHdr)
        }
        "preserve-without-private-uhdr" => Ok(OppoCameraTail::PreserveWithoutPrivateUhdr),
        "preserve-without-private-hdr" => Ok(OppoCameraTail::PreserveWithoutPrivateHdr),
        "preserve-no-uhdr" => Ok(OppoCameraTail::PreserveNoUhdr),
        "preserve-no-hdr" => Ok(OppoCameraTail::PreserveNoHdr),
        _ => Err(invalid_value(case_name, "oppo_camera_tail", value)),
    }
}

fn oppo_camera_tail_name(value: OppoCameraTail) -> &'static str {
    match value {
        OppoCameraTail::Off => "off",
        OppoCameraTail::Watermark => "watermark",
        OppoCameraTail::Compact => "compact",
        OppoCameraTail::Preserve => "preserve",
        OppoCameraTail::PreserveWithoutPortrait => "preserve-without-portrait",
        OppoCameraTail::PreserveWithoutPortraitOrPrivateHdr => {
            "preserve-without-portrait-or-private-hdr"
        }
        OppoCameraTail::PreserveWithoutPrivateUhdr => "preserve-without-private-uhdr",
        OppoCameraTail::PreserveWithoutPrivateHdr => "preserve-without-private-hdr",
        OppoCameraTail::PreserveNoUhdr => "preserve-no-uhdr",
        OppoCameraTail::PreserveNoHdr => "preserve-no-hdr",
    }
}

fn parse_tmap_format(value: Option<&str>, case_name: &str) -> io::Result<TmapFormat> {
    let value = value.unwrap_or("imageio");
    match value {
        "strict" => Ok(TmapFormat::Strict),
        "imageio" => Ok(TmapFormat::ImageIo),
        _ => Err(invalid_value(case_name, "tmap_format", value)),
    }
}

fn tmap_format_name(value: TmapFormat) -> &'static str {
    match value {
        TmapFormat::Strict => "strict",
        TmapFormat::ImageIo => "imageio",
    }
}

fn normalize(test_case: &PlanCase) -> io::Result<NormalizedPlan> {
    let family = parse_family(test_case.family.as_deref(), &test_case.name)?;
    let oppo_compatibility =
        parse_oppo_compatibility(test_case.oppo_compatibility.as_deref(), &test_case.name)?;
    let requested_input_processing_branch = parse_input_processing_branch(
        test_case.input_processing_branch.as_deref(),
        &test_case.name,
    )?;
    let oppo_camera_tail =
        parse_oppo_camera_tail(test_case.oppo_camera_tail.as_deref(), &test_case.name)?;
    let tmap_format = parse_tmap_format(test_case.tmap_format.as_deref(), &test_case.name)?;
    let effective_input_processing_branch = resolve_effective_input_processing_branch(
        requested_input_processing_branch,
        oppo_camera_tail,
        tmap_format,
    );
    let apple_feature_route = if test_case.apple_photographic_styles.unwrap_or(false)
        || test_case.apple_portrait.unwrap_or(false)
    {
        "apple-features"
    } else {
        "core"
    };

    Ok(NormalizedPlan {
        name: test_case.name.clone(),
        family,
        oppo_compatibility,
        requested_input_processing_branch: input_processing_branch_name(
            requested_input_processing_branch,
        )
        .to_owned(),
        effective_input_processing_branch: input_processing_branch_name(
            effective_input_processing_branch,
        )
        .to_owned(),
        oppo_camera_tail: oppo_camera_tail_name(oppo_camera_tail).to_owned(),
        tmap_format: tmap_format_name(tmap_format).to_owned(),
        apple_feature_route: apple_feature_route.to_owned(),
    })
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = env::args_os().collect();
    if arguments.len() != 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: xdremux-engine-plan-oracle <conversion_plan_cases.json>",
        )
        .into());
    }

    let reader = BufReader::new(File::open(&arguments[1])?);
    let cases: Vec<PlanCase> = serde_json::from_reader(reader)?;
    let plans = cases
        .iter()
        .map(normalize)
        .collect::<io::Result<Vec<_>>>()?;

    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    serde_json::to_writer(&mut writer, &plans)?;
    writer.write_all(b"\n")?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
