#!/usr/bin/env python3
from pathlib import Path


path = Path("crates/xdremux-cli/src/lib.rs")
text = path.read_text()

if "fn parse_batch_jobs(" not in text:
    anchor = """fn default_batch_jobs() -> usize {
    std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(1)
        .min(4)
}

"""
    helper = anchor + """fn parse_batch_jobs(raw: &str) -> Result<usize, String> {
    let jobs = raw
        .parse::<usize>()
        .map_err(|error| format!("invalid --jobs value {raw:?}: {error}"))?;
    if jobs == 0 {
        return Err("--jobs must be greater than zero".to_owned());
    }
    Ok(jobs)
}

"""
    if anchor not in text:
        raise SystemExit("default_batch_jobs anchor not found")
    text = text.replace(anchor, helper, 1)

old_field = """    /// Maximum number of concurrent conversions. Zero is treated as one.
    #[arg(long, default_value_t = default_batch_jobs(), value_name = "N")]
    jobs: usize,
"""
new_field = """    /// Maximum number of concurrent conversions; must be greater than zero.
    #[arg(
        long,
        default_value_t = default_batch_jobs(),
        value_name = "N",
        value_parser = parse_batch_jobs
    )]
    jobs: usize,
"""
if old_field in text:
    text = text.replace(old_field, new_field, 1)
elif "value_parser = parse_batch_jobs" not in text:
    raise SystemExit("BatchArgs jobs field anchor not found")

text = text.replace(
    "        jobs: arguments.jobs.max(1),",
    "        jobs: arguments.jobs,",
    1,
)

if "fn batch_rejects_zero_jobs_as_usage_error()" not in text:
    anchor = """    #[test]
    fn batch_requires_a_source() {
"""
    test = """    #[test]
    fn batch_rejects_zero_jobs_as_usage_error() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(
            run_from(
                ["batch", "--input", "capture.heic", "--jobs", "0"],
                &mut stdout,
                &mut stderr,
            ),
            2
        );
        assert!(stdout.is_empty());
        let error = String::from_utf8(stderr).unwrap();
        assert!(error.contains("--jobs must be greater than zero"), "{error}");
    }

"""
    if anchor not in text:
        raise SystemExit("batch unit-test anchor not found")
    text = text.replace(anchor, test + anchor, 1)

for marker in (
    "fn parse_batch_jobs(",
    "value_parser = parse_batch_jobs",
    "jobs: arguments.jobs,",
    "fn batch_rejects_zero_jobs_as_usage_error()",
):
    if marker not in text:
        raise SystemExit(f"jobs contract materialization missing marker: {marker}")
if "Zero is treated as one" in text or "jobs: arguments.jobs.max(1)," in text:
    raise SystemExit("legacy zero-clamping jobs behavior remains")

path.write_text(text)
