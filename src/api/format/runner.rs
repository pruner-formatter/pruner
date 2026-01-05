use anyhow::{Context, Result};
use std::{
  fs,
  io::Write,
  path::PathBuf,
  process::{Command, Stdio},
  time::{Instant, SystemTime, UNIX_EPOCH},
};

use crate::config::FormatterSpec;

#[derive(Debug)]
pub struct FormatOpts<'a> {
  pub printwidth: u32,
  pub language: &'a str,
}

fn unique_temp_file() -> std::io::Result<PathBuf> {
  let mut path = std::env::temp_dir();
  let nanos = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_nanos();
  path.push(format!("prune-format-{}-{nanos}", std::process::id()));
  Ok(path)
}

pub fn format(formatter: &FormatterSpec, source: &[u8], opts: &FormatOpts) -> Result<Vec<u8>> {
  log::trace!("Calling formatter [{}] with opts {:?}", formatter.cmd, opts);

  let use_stdin = formatter.stdin.unwrap_or(true);
  let mut temp_file: Option<PathBuf> = None;

  if !use_stdin {
    let path = unique_temp_file().context("Failed to create temp file for fomatting")?;
    fs::write(&path, source).context("Failed to write to temp file")?;
    temp_file = Some(path);
  }

  let file_var = temp_file
    .as_ref()
    .map(|path| path.to_string_lossy().to_string())
    .unwrap_or_default();

  let args = formatter.args.iter().map(|arg| {
    arg
      .replace("$textwidth", &format!("{}", opts.printwidth))
      .replace("$language", opts.language)
      .replace("$file", &file_var)
  });

  let mut command = Command::new(&formatter.cmd);
  command
    .args(args)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .stdin(Stdio::piped());

  let start = Instant::now();

  let result = || -> Result<Vec<u8>> {
    let mut proc = command.spawn()?;

    if use_stdin {
      let stdin = proc
        .stdin
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("Failed to open stdin"))?;
      stdin.write_all(source)?;
    }

    let output = proc.wait_with_output()?;

    if !output.status.success() {
      anyhow::bail!(
        "Failed to run formatter {}: {}",
        formatter.cmd,
        String::from_utf8_lossy(&output.stderr)
      );
    }

    if formatter.fail_on_stderr.unwrap_or(false) && !output.stderr.is_empty() {
      anyhow::bail!(
        "Failed to run formatter {}: {}",
        formatter.cmd,
        String::from_utf8_lossy(&output.stderr)
      );
    }

    let mut result = output.stdout;

    if !use_stdin {
      if let Some(path) = temp_file.as_ref() {
        result = fs::read(path).context("Failed to read temp file after formatting")?;
      }
    }

    Ok(result)
  }();

  log::debug!(
    "Formatted using [{}] in: {:?}",
    formatter.cmd,
    Instant::now().duration_since(start)
  );

  if let Some(ref path) = temp_file {
    if let Err(err) = fs::remove_file(path) {
      log::warn!("Failed to remove temp file {path:?}: {err}");
    }
  }

  result
}
