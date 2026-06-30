use anyhow::{anyhow, Context};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

/// Maximum time to wait for a single AI research call before giving up.
const TIMEOUT: Duration = Duration::from_secs(300);

/// Result of an AI research call.
pub struct AiResult {
    /// The short, extracted answer (from the `ANSWER:` line, if present).
    pub answer: String,
    /// The full model response, kept for the "details" view.
    pub full: String,
}

/// Run the AI research command (Claude Code CLI by default) in headless mode.
///
/// Invokes: `<program> -p "<prompt>" --output-format json --permission-mode
/// bypassPermissions`, which lets the model use its web tools without an
/// interactive permission prompt. The JSON envelope's `result` field is the
/// model's text; we extract a one-line answer from it.
pub async fn research(program: &str, prompt: &str) -> anyhow::Result<AiResult> {
    let output = tokio::time::timeout(
        TIMEOUT,
        Command::new(program)
            .arg("-p")
            .arg(prompt)
            .arg("--output-format")
            .arg("json")
            .arg("--permission-mode")
            .arg("bypassPermissions")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output(),
    )
    .await
    .map_err(|_| anyhow!("AI research timed out after {}s", TIMEOUT.as_secs()))?
    .with_context(|| format!("failed to launch AI command '{}'", program))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "AI command exited with {}: {}",
            output.status,
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    // The CLI's JSON envelope puts the model text in `result`; fall back to raw
    // stdout if the shape is unexpected (e.g. a different CLI was configured).
    let full = serde_json::from_str::<serde_json::Value>(&stdout)
        .ok()
        .and_then(|v| {
            v.get("result")
                .and_then(|r| r.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or(stdout);

    let answer = extract_answer(&full);
    Ok(AiResult { answer, full })
}

/// Pull a concise answer out of the model's response: prefer the last line that
/// starts with `ANSWER:`, otherwise fall back to the last non-empty line.
fn extract_answer(full: &str) -> String {
    for line in full.lines().rev() {
        if let Some(rest) = line.trim().strip_prefix("ANSWER:") {
            return rest.trim().to_string();
        }
    }
    full.lines()
        .rev()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("(no answer)")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_answer_line() {
        let text = "I looked it up.\nReviews are positive.\nANSWER: Yes, in-unit laundry";
        assert_eq!(extract_answer(text), "Yes, in-unit laundry");
    }

    #[test]
    fn falls_back_to_last_line() {
        let text = "first\n\nlast line\n\n";
        assert_eq!(extract_answer(text), "last line");
    }
}
