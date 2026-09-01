#!/usr/bin/env python3
from pathlib import Path

path = Path("crates/xdremux-cli/src/lib.rs")
text = path.read_text()

convert_anchor = '''#[derive(Debug, Args)]
struct ConvertArgs {
    /// Input ProXDR HEIC or supported Motion Photo.
    #[arg(long, value_name = "INPUT")]
    input: PathBuf,
    /// Output HEIC; ProXDR defaults in-place, Motion Photo chooses a new pair.
    #[arg(long, value_name = "OUTPUT")]
    output: Option<PathBuf>,
}
'''
convert_replacement = '''#[derive(Debug, Args, Default)]
struct ProductConversionArgs {
    /// Preserve compatibility with OPPO Gallery when converting ProXDR still images.
    #[arg(long)]
    oppo_compatible: bool,
}

impl ProductConversionArgs {
    fn request(&self) -> ConversionRequest {
        if self.oppo_compatible {
            ConversionRequest::oppo_gallery_compatible()
        } else {
            ConversionRequest::default()
        }
    }
}

#[derive(Debug, Args)]
struct ConvertArgs {
    /// Input ProXDR HEIC or supported Motion Photo.
    #[arg(long, value_name = "INPUT")]
    input: PathBuf,
    /// Output HEIC; ProXDR defaults in-place, Motion Photo chooses a new pair.
    #[arg(long, value_name = "OUTPUT")]
    output: Option<PathBuf>,
    #[command(flatten)]
    product: ProductConversionArgs,
}
'''
if "struct ProductConversionArgs" not in text:
    if convert_anchor not in text:
        raise SystemExit("ConvertArgs anchor not found")
    text = text.replace(convert_anchor, convert_replacement, 1)

batch_marker = '''    /// File converted assets by asset type and primary capture mode.
    #[arg(long)]
    categorize: bool,
'''
batch_replacement = batch_marker + '''    #[command(flatten)]
    product: ProductConversionArgs,
'''
if "product: ProductConversionArgs" not in text[text.find("struct BatchArgs"):text.find("fn write_clap_error")]:
    if batch_marker not in text:
        raise SystemExit("BatchArgs product anchor not found")
    text = text.replace(batch_marker, batch_replacement, 1)

old_run_sig = '''fn run_convert(
    input: PathBuf,
    output: Option<PathBuf>,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
'''
new_run_sig = '''fn run_convert(arguments: ConvertArgs, stdout: &mut impl Write, stderr: &mut impl Write) -> u8 {
    let request = arguments.product.request();
    let input = arguments.input;
    let output = arguments.output;
'''
if old_run_sig in text:
    text = text.replace(old_run_sig, new_run_sig, 1)

text = text.replace(
    "match runtime.convert_motion_photo_file(&source, &input, &output) {",
    "match runtime.convert_motion_photo_file_with_request(&source, &input, &output, request) {",
    1,
)
text = text.replace(
    "runtime.convert_proxdr_file(&source, &output, ConversionRequest::default(), |_| {})",
    "runtime.convert_proxdr_file(&source, &output, request, |_| {})",
    1,
)

batch_receipt = '''    let receipt =
        runtime.convert_batch_with_options(items, ConversionRequest::default(), &execution_options);
'''
if batch_receipt in text:
    text = text.replace(
        batch_receipt,
        '''    let receipt = runtime.convert_batch_with_options(
        items,
        arguments.product.request(),
        &execution_options,
    );
''',
        1,
    )

text = text.replace(
    "}) => run_convert(arguments.input, arguments.output, stdout, stderr),",
    "}) => run_convert(arguments, stdout, stderr),",
    1,
)

# Hand-constructed BatchArgs in tests need the flattened product defaults.
for marker in (
    "            categorize: false,\n            jobs: 1,",
    "            categorize: true,\n            jobs: 1,",
):
    if marker in text:
        text = text.replace(
            marker,
            marker.replace("            jobs: 1,", "            product: ProductConversionArgs::default(),\n            jobs: 1,"),
        )

# Extend existing convert parser test with the default intent.
text = text.replace(
    '''        assert_eq!(arguments.output, None);
    }

    #[test]
    fn convert_accepts_explicit_output()''',
    '''        assert_eq!(arguments.output, None);
        assert!(!arguments.product.oppo_compatible);
    }

    #[test]
    fn convert_accepts_oppo_gallery_product_intent() {
        let command = parse(&[
            "convert",
            "--input",
            "capture.heic",
            "--oppo-compatible",
        ]);
        let RootCommand::Convert(arguments) = command.command else {
            panic!("expected convert command");
        };
        assert!(arguments.product.oppo_compatible);
        let request = arguments.product.request();
        assert!(request.requests_oppo_gallery_compatibility());
    }

    #[test]
    fn convert_accepts_explicit_output()''',
    1,
)

# Batch parse contract should prove the same reusable Args surface is shared.
text = text.replace(
    '''        assert!(!arguments.categorize);
        assert!(arguments.jobs >= 1 && arguments.jobs <= 4);
    }
''',
    '''        assert!(!arguments.categorize);
        assert!(!arguments.product.oppo_compatible);
        assert!(arguments.jobs >= 1 && arguments.jobs <= 4);
    }

    #[test]
    fn batch_accepts_oppo_gallery_product_intent() {
        let command = parse(&[
            "batch",
            "--input",
            "a.heic",
            "--oppo-compatible",
        ]);
        let RootCommand::Batch(arguments) = command.command else {
            panic!("expected batch command");
        };
        assert!(arguments.product.oppo_compatible);
        assert!(arguments.product.request().requests_oppo_gallery_compatibility());
    }
''',
    1,
)

# A permanent product-surface contract: expose intents, not implementation knobs.
if "fn conversion_help_exposes_intent_not_internal_policy()" not in text:
    test_marker = '''    #[test]
    fn clap_command_definition_is_internally_consistent() {
'''
    product_help_test = '''    #[test]
    fn conversion_help_exposes_intent_not_internal_policy() {
        for command in ["convert", "batch"] {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            assert_eq!(run_from([command, "--help"], &mut stdout, &mut stderr), 0);
            assert!(stderr.is_empty());
            let help = String::from_utf8(stdout).unwrap();
            assert!(help.contains("--oppo-compatible"), "{help}");
            for internal in [
                "--family",
                "--oppo-compat ",
                "--oppo-camera-tail",
                "--input-processing",
                "--tmap-format",
            ] {
                assert!(!help.contains(internal), "internal option {internal} leaked into {command} help:\n{help}");
            }
        }
    }

'''
    if test_marker not in text:
        raise SystemExit("CLI final test anchor not found")
    text = text.replace(test_marker, product_help_test + test_marker, 1)

required = (
    "struct ProductConversionArgs",
    "oppo_compatible: bool",
    "ConversionRequest::oppo_gallery_compatible()",
    "convert_motion_photo_file_with_request",
    "arguments.product.request()",
    "fn conversion_help_exposes_intent_not_internal_policy()",
)
for marker in required:
    if marker not in text:
        raise SystemExit(f"missing CLI product marker: {marker}")

path.write_text(text)
