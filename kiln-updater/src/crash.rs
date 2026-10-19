//! Crash reporting hook (stub, full implementation in M56).

use std::panic::PanicHookInfo;

/// Crash report metadata.
#[derive(Debug, Clone)]
pub struct CrashReport {
    pub version: String,
    pub message: String,
    pub backtrace: String,
}

/// Install a panic hook for crash reporting (opt-in).
///
/// This is a stub for now. Full implementation with symbolication
/// and upload to crash reporting service will be added in M56.
pub fn install_crash_hook(version: String) {
    std::panic::set_hook(Box::new(move |info: &PanicHookInfo| {
        let report = CrashReport {
            version: version.clone(),
            message: info.to_string(),
            backtrace: std::backtrace::Backtrace::force_capture().to_string(),
        };

        // Stub: log the crash report
        log::error!("Crash detected: {:?}", report);

        // Full implementation in M56:
        // - Serialize report
        // - Upload to crash reporting service (Sentry, Crashlytics, etc.)
        // - Write to local file for later upload if network unavailable
    }));
}

/// Manually report a crash (for non-panic errors).
pub fn report_crash(report: CrashReport) {
    // Stub: log the crash report
    log::error!("Manual crash report: {:?}", report);

    // Full implementation in M56:
    // - Upload to crash reporting service
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_report_creation() {
        let report = CrashReport {
            version: "1.0.0".to_string(),
            message: "Test crash".to_string(),
            backtrace: "backtrace here".to_string(),
        };
        assert_eq!(report.version, "1.0.0");
    }
}
