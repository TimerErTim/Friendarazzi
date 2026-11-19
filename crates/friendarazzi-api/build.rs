use std::path::Path;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() {
    println!("cargo:rerun-if-env-changed=FRIENDARAZZI_BUILD_COMMIT");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");

    emit_git_rerun_hints();

    if let Some(commit) = resolve_commit() {
        println!("cargo:rustc-env=FRIENDARAZZI_BUILD_COMMIT={commit}");
    }

    let build_time = format_utc_rfc3339(resolve_build_timestamp());
    println!("cargo:rustc-env=FRIENDARAZZI_BUILD_TIME={build_time}");
}

fn emit_git_rerun_hints() {
    let git_head = Path::new("../../.git/HEAD");
    if !git_head.exists() {
        return;
    }
    println!("cargo:rerun-if-changed={}", git_head.display());
    if let Ok(head) = std::fs::read_to_string(git_head)
        && let Some(reference) = head.trim().strip_prefix("ref: ")
    {
        let ref_path = Path::new("../../.git").join(reference);
        if ref_path.exists() {
            println!("cargo:rerun-if-changed={}", ref_path.display());
        }
    }
}

fn resolve_commit() -> Option<String> {
    for var in ["FRIENDARAZZI_BUILD_COMMIT", "GITHUB_SHA"] {
        if let Ok(val) = std::env::var(var) {
            let trimmed = val.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_owned());
            }
        }
    }

    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8(output.stdout).ok()?;
    let trimmed = sha.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn resolve_build_timestamp() -> u64 {
    if let Ok(raw) = std::env::var("SOURCE_DATE_EPOCH")
        && let Ok(secs) = raw.trim().parse::<u64>()
    {
        return secs;
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

/// Format Unix seconds as `YYYY-MM-DDTHH:MM:SSZ` without adding a build dependency.
fn format_utc_rfc3339(epoch_secs: u64) -> String {
    let days = (epoch_secs / 86_400) as i64;
    let rem = epoch_secs % 86_400;
    let hour = rem / 3_600;
    let minute = (rem % 3_600) / 60;
    let second = rem % 60;

    // Howard Hinnant's civil-from-days algorithm (valid across proleptic Gregorian calendar).
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = y + i64::from(month <= 2);

    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}
