//! Nonblocking exporter supervision. No shell, Cargo invocation or detached build.
use super::BuildConfig;
use kiln_core::export::{self, ExportReceipt, RECEIPT_FILE};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver, TryRecvError},
    time::{Duration, Instant},
};

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Success(PathBuf),
    Failed(String),
    Cancelled,
}
#[derive(Debug)]
pub struct ExportJob {
    child: Child,
    logs: Receiver<String>,
    output: PathBuf,
    job: String,
    project: String,
    target: String,
    cancelled: Option<Instant>,
    exited: Option<(ExitStatus, Instant)>,
    verification: Option<Receiver<Result<(), String>>>,
}
impl ExportJob {
    pub fn start(config: &BuildConfig, manifest: &Path, project: &str) -> Result<Self, String> {
        let root = manifest.parent().ok_or("Project has no directory")?;
        let output = if config.output_dir.is_absolute() {
            config.output_dir.clone()
        } else {
            root.join(&config.output_dir)
        };
        if output.file_name().is_none() {
            return Err("Choose an output directory".into());
        }
        fs::create_dir_all(output.parent().ok_or("Output has no parent")?)
            .map_err(|e| e.to_string())?;
        let output = output
            .parent()
            .unwrap()
            .canonicalize()
            .map_err(|e| e.to_string())?
            .join(output.file_name().unwrap());
        let job = uuid::Uuid::new_v4().to_string();
        let mut child = Command::new(&config.cli_path)
            .arg("export")
            .arg("--platform")
            .arg(config.target.platform())
            .arg("--project")
            .arg(manifest)
            .arg("--output")
            .arg(&output)
            .arg("--player")
            .arg(&config.player_path)
            .arg("--web-runtime")
            .arg(&config.web_runtime)
            .arg("--job-id")
            .arg(&job)
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Cannot start exporter {}: {e}", config.cli_path.display()))?;
        let (sender, logs) = mpsc::sync_channel(256);
        for mut pipe in [
            Box::new(child.stdout.take().unwrap()) as Box<dyn Read + Send>,
            Box::new(child.stderr.take().unwrap()),
        ] {
            let sender = sender.clone();
            std::thread::spawn(move || {
                let mut buffer = [0u8; 4096];
                let mut pending = String::new();
                loop {
                    let count = match pipe.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    pending.push_str(&String::from_utf8_lossy(&buffer[..count]));
                    while let Some(end) = pending.find('\n') {
                        let line: String = pending.drain(..=end).collect();
                        if sender
                            .try_send(line.trim_end().chars().take(4096).collect())
                            .is_err()
                        { /* bounded log tail */ }
                    }
                    if pending.len() > 4096 {
                        let _ = sender.try_send(pending.chars().take(4096).collect());
                        pending.clear();
                    }
                }
                if !pending.is_empty() {
                    let _ = sender.try_send(pending);
                }
            });
        }
        Ok(Self {
            child,
            logs,
            output,
            job,
            project: project.into(),
            target: config.target.platform().into(),
            cancelled: None,
            exited: None,
            verification: None,
        })
    }
    pub fn is_verifying(&self) -> bool {
        self.verification.is_some() && self.cancelled.is_none()
    }
    pub fn cancel(&mut self) {
        if self.cancelled.is_none() {
            self.cancelled = Some(Instant::now());
            let _ = fs::write(export::cancel_path(&self.output, &self.job), b"cancel");
        }
    }
    fn cleanup(&self) {
        let _ = fs::remove_dir_all(export::staging_path(&self.output, &self.job));
        let _ = fs::remove_file(export::cancel_path(&self.output, &self.job));
    }
    pub fn poll(&mut self, log: &mut Vec<String>) -> Option<Outcome> {
        if let Some(verification) = &self.verification {
            if self.cancelled.is_some() {
                self.cleanup();
                return Some(Outcome::Cancelled);
            }
            return match verification.try_recv() {
                Ok(Ok(())) => Some(Outcome::Success(self.output.clone())),
                Ok(Err(error)) => Some(Outcome::Failed(format!(
                    "Exporter exited successfully but output validation failed: {error}"
                ))),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Outcome::Failed(
                    "Output verification stopped unexpectedly".into(),
                )),
            };
        }
        let mut closed = false;
        loop {
            match self.logs.try_recv() {
                Ok(line) => {
                    log.push(line);
                    if log.len() > 2000 {
                        log.remove(0);
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    closed = true;
                    break;
                }
            }
        }
        if self.exited.is_none() {
            if self
                .cancelled
                .is_some_and(|at| at.elapsed() > Duration::from_secs(3))
            {
                let _ = self.child.kill();
            }
            match self.child.try_wait() {
                Ok(Some(status)) => self.exited = Some((status, Instant::now())),
                Ok(None) => return None,
                Err(e) => {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    self.cleanup();
                    return Some(Outcome::Failed(format!("Cannot wait for exporter: {e}")));
                }
            }
        }
        let (status, at) = self.exited.as_ref()?;
        if !closed && at.elapsed() < Duration::from_millis(250) {
            return None;
        }
        self.cleanup();
        if self.cancelled.is_some() {
            return Some(Outcome::Cancelled);
        }
        if !status.success() {
            return Some(Outcome::Failed(format!(
                "Exporter failed: {status}. See diagnostics above."
            )));
        }
        let (sender, receiver) = mpsc::sync_channel(1);
        let output = self.output.clone();
        let job = self.job.clone();
        let project = self.project.clone();
        let target = self.target.clone();
        std::thread::spawn(move || {
            let _ = sender.send(verify_output(&output, &job, &project, &target));
        });
        self.verification = Some(receiver);
        None
    }
}
impl Drop for ExportJob {
    fn drop(&mut self) {
        if self.exited.is_none() {
            self.cancel();
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
        self.cleanup();
    }
}
fn verify_output(output: &Path, job: &str, project: &str, target: &str) -> Result<(), String> {
    let path = output.join(RECEIPT_FILE);
    if fs::metadata(&path).map_err(|e| e.to_string())?.len() > export::MAX_RECEIPT_BYTES {
        return Err("Receipt is oversized".into());
    }
    let receipt: ExportReceipt =
        serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    receipt.validate(output, job, project)?;
    if receipt.runtime.platform != target {
        return Err("Receipt target does not match this export".into());
    }
    let bytes = if target == "web" {
        let runtime = kiln_assets::web_bundle::verify(&output.join("runtime"))?;
        if runtime != receipt.runtime {
            return Err("Browser receipt/runtime mismatch".into());
        }
        fs::read(output.join("project/kiln.toml")).map_err(|e| e.to_string())?
    } else {
        let archive =
            kiln_assets::Archive::load(&output.join("assets.kar")).map_err(|e| e.to_string())?;
        archive.extract("kiln.toml").map_err(|e| e.to_string())?
    };
    let saved =
        kiln_core::Project::from_toml(std::str::from_utf8(&bytes).map_err(|e| e.to_string())?)?;
    if saved.id != project {
        return Err("Packed project identity does not match this export".into());
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new(script: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("kiln-job-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&dir).unwrap();
            fs::write(dir.join("cli"), format!("#!/bin/sh\n{script}\n")).unwrap();
            fs::set_permissions(dir.join("cli"), fs::Permissions::from_mode(0o755)).unwrap();
            fs::write(dir.join("kiln.toml"), "").unwrap();
            Self(dir)
        }
        fn job(&self) -> ExportJob {
            ExportJob::start(
                &BuildConfig {
                    target: Default::default(),
                    web_runtime: self.0.join("web-runtime"),
                    output_dir: self.0.join("output"),
                    cli_path: self.0.join("cli"),
                    player_path: self.0.join("player"),
                },
                &self.0.join("kiln.toml"),
                "project",
            )
            .unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn finish(job: &mut ExportJob) -> (Outcome, Vec<String>) {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut logs = Vec::new();
        loop {
            if let Some(result) = job.poll(&mut logs) {
                return (result, logs);
            }
            assert!(Instant::now() < deadline, "Job did not finish");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    #[test]
    fn actual_exit_code_and_both_streams_override_success_text() {
        let fixture = Fixture::new("echo 'Build completed'; echo 'real error' >&2; exit 9");
        let mut job = fixture.job();
        let (outcome, logs) = finish(&mut job);
        assert!(matches!(outcome,Outcome::Failed(ref e) if e.contains('9')));
        assert!(logs.iter().any(|l| l.contains("Build completed")));
        assert!(logs.iter().any(|l| l.contains("real error")));
    }
    #[test]
    fn zero_exit_without_this_jobs_validated_artifact_is_failure() {
        let fixture = Fixture::new("echo 'success'; exit 0");
        let mut job = fixture.job();
        assert!(
            matches!(finish(&mut job).0,Outcome::Failed(ref e) if e.contains("output validation failed"))
        );
    }
    #[test]
    fn pending_output_verification_keeps_polling_and_can_be_cancelled() {
        let fixture = Fixture::new("exit 0");
        let mut job = fixture.job();
        let status = job.child.wait().unwrap();
        job.exited = Some((status, Instant::now()));
        let (_sender, receiver) = mpsc::sync_channel(1);
        job.verification = Some(receiver);
        assert!(job.poll(&mut Vec::new()).is_none());
        job.cancel();
        assert_eq!(job.poll(&mut Vec::new()), Some(Outcome::Cancelled));
        assert!(!export::cancel_path(&job.output, &job.job).exists());
    }

    #[test]
    fn cancellation_fallback_and_drop_reap_the_owned_process() {
        let fixture = Fixture::new("exec sleep 20");
        let mut job = fixture.job();
        job.cancel();
        assert_eq!(finish(&mut job).0, Outcome::Cancelled);
        assert!(!export::cancel_path(&job.output, &job.job).exists());
        let job = fixture.job();
        let pid = job.child.id();
        drop(job);
        let status = Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
        assert!(!status.success(), "Dropped export process still exists");
    }
}
