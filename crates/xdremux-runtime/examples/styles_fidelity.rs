//! Research entry point; the product CLI and its source admission are unchanged.
#[cfg(target_os = "macos")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::{fs, path::Path};
    let path = std::env::args_os()
        .nth(1)
        .ok_or("request JSON path required")?;
    let request: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    let text = |key: &str| request[key].as_str().ok_or("missing request path");
    let number = |key: &str| request[key].as_f64().ok_or("missing calibration number");
    let input = fs::read(text("input")?)?;
    let before = xdremux_engine::apple_style_data_sha256(&input);
    let runtime = xdremux_runtime::PortableRuntime::default();
    runtime.rebuild_styles_for_research(
        Path::new(text("adapter")?),
        &input,
        Path::new(text("output")?),
        number("maximum_stops")? as f32,
        xdremux_engine::AppleStyleLinearCalibration {
            baseline_scale: number("baseline_scale")? as f32,
            gain_scale: number("gain_scale")? as f32,
            baseline_maximum: number("baseline_maximum")? as f32,
            histogram_light_maps: request["histogram_light_maps"].as_bool().unwrap_or(false),
        },
    )?;
    if xdremux_engine::apple_style_data_sha256(&fs::read(text("input")?)?) != before {
        return Err("research source changed".into());
    }
    println!(
        "{}",
        serde_json::json!({"source_preserved": true, "input_sha256": before})
    );
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Styles fidelity generation requires macOS and the Apple adapter");
    std::process::exit(1);
}
