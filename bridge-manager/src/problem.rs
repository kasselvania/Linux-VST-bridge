//! Plain-English explanations of retained failure records.
//!
//! Only enumerated values are mapped. Free text from a report (SDK reason
//! strings, vendor stdout/stderr, paths) is never copied into an explanation;
//! an unrecognised value becomes a generic sentence and a fixed code.
use crate::{hex, require, valid_hex, Artifact, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Read;

/// Retained Windows reports are bounded control records.
pub const REPORT_LIMIT: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Explanation {
    pub code: String,
    pub what_happened: String,
    pub likely_cause: String,
    pub what_to_try: String,
}

impl Explanation {
    fn new(code: impl Into<String>, what: impl Into<String>, cause: impl Into<String>,
        next: impl Into<String>) -> Self {
        Self { code: code.into(), what_happened: what.into(), likely_cause: cause.into(),
            what_to_try: next.into() }
    }
    /// One line for availability reasons and journey rows.
    pub fn summary(&self) -> String {
        format!("{} {}", self.what_happened, self.likely_cause)
    }
    pub fn value(&self) -> Value {
        json!(self)
    }
    /// Prefer a later check's own record; when that check kept no details, a
    /// discovery record for the same plug-in file is shown, labelled as such.
    pub fn or_first_found(self, discovered: Option<Explanation>) -> Explanation {
        match discovered {
            Some(mut found) if self.code == "details_not_retained" => {
                found.likely_cause.push_str(" (Recorded when this plug-in was first found.)");
                found
            }
            _ => self,
        }
    }
}

/// A failed supervised inspection that retained an exact report. Display is
/// the original error text, so recorded failure codes are unchanged.
#[derive(Debug)]
pub struct BoundFailure {
    pub message: String,
    pub session: String,
    pub report: Artifact,
}
impl std::fmt::Display for BoundFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for BoundFailure {}
impl BoundFailure {
    /// Attach the exact session report when one was written; otherwise the
    /// original error is returned unchanged.
    pub fn bind(error: Box<dyn std::error::Error + Send + Sync>, session: &str,
        report: &std::path::Path) -> Box<dyn std::error::Error + Send + Sync> {
        match crate::digest(report) {
            Ok(sha256) if valid_hex(session, 32) => Box::new(Self {
                message: error.to_string(), session: session.into(),
                report: Artifact { path: report.into(), sha256 },
            }),
            _ => error,
        }
    }
    /// Session and digest only; the reader rebuilds the managed report path.
    pub fn binding(&self) -> Value {
        json!({"session":self.session,"sha256":self.report.sha256})
    }
}

/// Read one exact retained report: bounded and byte-identical to its record.
pub fn read_report(report: &Artifact) -> Result<Value> {
    require(report.path.is_absolute() && valid_hex(&report.sha256, 64), "problem_report_identity")?;
    let mut bytes = Vec::new();
    std::fs::File::open(&report.path)?.take(REPORT_LIMIT as u64 + 1).read_to_end(&mut bytes)?;
    require(bytes.len() <= REPORT_LIMIT, "problem_report_bound")?;
    require(hex(&Sha256::digest(&bytes)) == report.sha256, "problem_report_changed")?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Explain one exact retained report. A missing or changed report is never
/// replaced by a different record: it is reported as not retained.
pub fn from_artifact(report: &Artifact, failure: &str) -> Explanation {
    match read_report(report) {
        Ok(value) => from_report(&value).or_else(|| from_code(failure))
            .unwrap_or_else(|| unrecognised(failure)),
        Err(_) => details_not_retained(failure),
    }
}

const CLEANUP_NOTE: &str = " The bridge also could not confirm the Windows side closed; \
    restart the manager before trying again.";

/// Explain a supervised Windows session report. None means the report records
/// no failure (the caller's own code then applies).
pub fn from_report(report: &Value) -> Option<Explanation> {
    let cleanup = report["cleanup_confirmed"] == true;
    let mut explanation = match report["error"].as_str() {
        Some(error) => classify_error(error, report),
        None if !cleanup => return Some(cleanup_unconfirmed()),
        None => return None,
    };
    if !cleanup {
        explanation.what_to_try.push_str(CLEANUP_NOTE);
    }
    Some(explanation)
}

fn leading_number(text: &str) -> Option<i64> {
    let digits: String = text.chars().take_while(char::is_ascii_digit).take(10).collect();
    digits.parse().ok()
}

fn records(report: &Value) -> std::slice::Iter<'_, Value> {
    report["records"].as_array().map(|rows| rows.as_slice()).unwrap_or(&[]).iter()
}

fn classify_error(error: &str, report: &Value) -> Explanation {
    if let Some(rest) = error.strip_prefix("Windows SDK host failed: ") {
        return match leading_number(rest) {
            Some(90) => sdk_failure(report),
            Some(91) => Explanation::new("sdk_shutdown",
                "The plug-in started, but it did not shut down cleanly when the bridge closed it.",
                "It returned an error while closing.",
                "Run the check again. If it keeps happening, the plug-in may need an update."),
            code => unrecognised_exit(code),
        };
    }
    if error.starts_with("Windows host exited without successful close") {
        let outer = report["exit_before_cleanup"].as_i64()
            .or_else(|| report["raw_exit"].as_i64()).filter(|code| *code != 0);
        let closed = closed_exit(report).filter(|code| *code != 0);
        return host_exit(outer.or(closed), report);
    }
    if error.starts_with("TimeoutError: Windows startup deadline") {
        return Explanation::new("timeout_startup",
            "Windows did not finish starting the plug-in host within 3 minutes.",
            "The Windows runtime may still have been setting itself up, or the computer was very busy.",
            "Close other heavy programs and run the check again.");
    }
    if let Some(operation) = error.strip_prefix("TimeoutError: Windows call deadline: ") {
        let operation = operation.split(';').next().unwrap_or_default().to_ascii_lowercase();
        let during = match operation.as_str() {
            "load_library" => "while Windows was loading the plug-in file",
            "init_dll" => "while its own start-up routine was running",
            "exit_dll" | "free_library" => "while it was being closed",
            other if other.contains("factory") || other.contains("class") =>
                "while it was listing the plug-ins it contains",
            other if other.contains("bus") || other.contains("sample") =>
                "while it was describing its audio inputs and outputs",
            other if other.contains("graphics") || other.contains("editor") =>
                "while its editor window was being checked",
            _ => "during a plug-in call",
        };
        return Explanation::new("timeout_call",
            format!("The plug-in stopped responding {during}."),
            "It did not answer within 30 seconds. Some plug-ins wait for a sign-in, activation or update window, or for a missing component.",
            "Open the vendor's app to finish any sign-in, activation or update, then run the check again.");
    }
    if error.starts_with("TimeoutError: inspection deadline") {
        return Explanation::new("timeout_inspection",
            "Checking the plug-in took longer than 3 minutes, so the bridge stopped it.",
            "The plug-in may have been waiting for something, such as a hidden window or a download, or it may be stuck.",
            "Open the vendor's app to finish any sign-in, activation or update, then run the check again.");
    }
    Explanation::new("check_failed_unrecognised",
        "The plug-in check failed for a reason the bridge cannot describe yet.",
        "The detailed record is kept on this computer.",
        "Run the check again. If it fails again, create a problem report.")
}

fn closed_exit(report: &Value) -> Option<i64> {
    records(report).rev().find(|r| r["state"] == "ap8_inspection_closed")
        .and_then(|r| r["exit_code"].as_i64())
}

fn has_state(report: &Value, state: &str) -> bool {
    records(report).any(|r| r["state"] == state)
}

/// The Win32 status the host recorded for one call, if it failed.
fn win32_error(report: &Value, operation: &str) -> Option<u32> {
    records(report).filter(|r| r["event"] == "call_completed"
        && r["operation"] == operation && r["return_kind"] == "win32_error")
        .filter_map(|r| r["win32_error_u32_hex"].as_str())
        .find(|text| valid_hex(text, 8))
        .and_then(|text| u32::from_str_radix(text, 16).ok())
}

/// Exit 70 is explained only by the LoadLibrary status the host recorded.
fn load_error(report: &Value) -> Option<Explanation> {
    Some(match win32_error(report, "load_library")? {
        0x7e => Explanation::new("module_dependency_missing",
            "Windows could not load the plug-in file.",
            "A Windows component it needs is missing (often a Visual C++ runtime).",
            "Run the plug-in's own installer again so it can add its required components, then run the check again."),
        0xc1 => Explanation::new("module_bad_format",
            "Windows could not load the plug-in file.",
            "It may be a 32-bit plug-in, which is not supported, or the file may be damaged or built for a different kind of computer.",
            "Install the 64-bit version of the plug-in if the vendor offers one, or reinstall it."),
        0x7f => Explanation::new("module_dependency_mismatch",
            "Windows could not load the plug-in file.",
            "A Windows component it needs is present but is the wrong version.",
            "Update or reinstall the plug-in so it brings matching components, then run the check again."),
        code => Explanation::new("module_load_failed",
            "Windows could not load the plug-in file.",
            format!("Windows reported error 0x{code:08X} while loading it."),
            "Reinstall the plug-in, then run the check again. If it fails again, create a problem report."),
    })
}

/// An exit status is explained only together with the host record that
/// produced it; otherwise the sentence stays generic.
fn host_exit(code: Option<i64>, report: &Value) -> Explanation {
    let has = |state| has_state(report, state);
    match code {
        // Returned before any record exists, for the bridge's own helper or
        // the plug-in file alike: neither is blamed.
        Some(64) => Explanation::new("start_verification_failed",
            "The bridge could not verify its Windows helper or the plug-in file before starting the check.",
            "One of them changed or could not be read.",
            "Find installed plug-ins again, then run the check again. If it keeps happening, repair or update the manager."),
        Some(65) if has("readiness_announced") => Explanation::new("plugin_file_changed",
            "The plug-in file changed while the bridge was starting the check.",
            "The plug-in may have been updated or reinstalled during the check.",
            "Find installed plug-ins again, then run the check again."),
        Some(70) => load_error(report).unwrap_or_else(|| unrecognised_exit(code)),
        Some(71) if has("module_entry_failed") => Explanation::new("module_entry_failed",
            "The plug-in file loaded, but its own start-up routine reported a failure.",
            "It may be missing files from its installer, an activation step, or a Windows component it expects.",
            "Use the vendor's installer or app to repair or activate the plug-in, then run the check again."),
        Some(72) if has("factory_export_missing") => Explanation::new("factory_export_missing",
            "This file is not a VST3 plug-in the bridge can use.",
            "It has no VST3 entry point; it may be a helper file or a different plug-in format.",
            "If another copy of this plug-in works, nothing needs to change. Otherwise reinstall it."),
        Some(73) if has("factory_get_failed") => Explanation::new("factory_null",
            "The plug-in file loaded but did not provide its list of plug-ins.",
            "It may not have finished starting, for example because installation or activation is incomplete.",
            "Use the vendor's installer or app to finish installation or activation, then run the check again."),
        Some(74..=78) if has("factory_obtained") => Explanation::new("factory_enumeration",
            "The plug-in file loaded, but its list of plug-ins could not be read.",
            "It returned incomplete or invalid information about what it contains.",
            "Update the plug-in to its latest version, then run the check again."),
        Some(79) if has("module_exit_failed") => Explanation::new("module_exit_failed",
            "The plug-in was checked, but its own shutdown routine reported a failure.",
            "It returned an error while the bridge was closing it.",
            "Run the check again. If it keeps happening, the plug-in may need an update."),
        Some(80) if win32_error(report, "free_library").is_some() => Explanation::new("module_unload_failed",
            "The plug-in was checked, but Windows could not unload it afterwards.",
            "Something inside the plug-in was still in use when the bridge closed it.",
            "Run the check again. If it keeps happening, create a problem report."),
        Some(92) => Explanation::new("forced_stop",
            "The bridge had to force the Windows side to stop after the check.",
            "The plug-in did not finish closing its windows or resources when asked.",
            "Run the check again. If it keeps happening, create a problem report."),
        _ => unrecognised_exit(code),
    }
}

fn unrecognised_exit(code: Option<i64>) -> Explanation {
    let what = match code.filter(|code| *code != 0) {
        Some(code) => format!("The Windows side of the bridge stopped before the check finished (exit code {code})."),
        None => "The Windows side of the bridge stopped before the check finished.".into(),
    };
    Explanation::new("host_exit_unrecognised", what,
        "The plug-in may have crashed, or the Windows runtime stopped it.",
        "Run the check again. If it keeps happening, create a problem report.")
}

/// Enumerated SDK call names written by the Windows host's ap8_failure row.
fn sdk_group(reason: &str) -> Option<&'static str> {
    const COMPONENT: &[&str] = &["setHostContext", "setHostContext failed", "createComponent",
        "null component", "initializeComponent", "queryAudioProcessor", "null audio processor"];
    const BUSES: &[&str] = &["getBusCount", "getBusInfo", "getBusArrangement", "bus count bound",
        "setStereoMainPair", "stereo main-pair negotiation/readback", "censusSetProcessing"];
    const CONTROLLER: &[&str] = &["queryEditController", "controller query tuple",
        "getControllerClassId", "createController", "null controller", "initializeController",
        "setComponentHandler", "connectComponent", "connectController", "parameter count bound",
        "enumerateParameters", "synchronizeController"];
    // The first settings hand-over from the audio engine to its controls.
    const SETTINGS: &[&str] = &["initial controller synchronization failed",
        "initial state stream lifetime", "initial state stream bounds/allocation",
        "initial state SDK failure"];
    const LIST: &[&str] = &["audio class absent", "class count bound", "getClassInfo",
        "getFactoryInfo", "getClassInfoUnicode", "getClassInfo2",
        "factory metadata class identity changed", "selected class syntax",
        "unterminated vendor class name", "unterminated SDK string", "invalid SDK string",
        "unterminated factory metadata"];
    let parameter_index = reason.strip_prefix("getParameterInfo index ")
        .is_some_and(|n| !n.is_empty() && n.len() <= 10 && n.bytes().all(|b| b.is_ascii_digit()));
    if COMPONENT.contains(&reason) { Some("component") }
    else if BUSES.contains(&reason) { Some("buses") }
    else if CONTROLLER.contains(&reason) || parameter_index { Some("controller") }
    else if SETTINGS.contains(&reason) { Some("settings") }
    else if LIST.contains(&reason) { Some("list") }
    else if reason == "multiple audio classes require explicit selection" { Some("multiple") }
    else { None }
}

fn sdk_failure(report: &Value) -> Explanation {
    let reason = records(report).find(|r| r["state"] == "ap8_failure")
        .and_then(|r| r["reason"].as_str()).unwrap_or_default();
    match sdk_group(reason) {
        Some("component") => Explanation::new("sdk_component_start",
            "The plug-in loaded, but it refused to start its audio engine.",
            "It is usually missing something it needs at start-up: files from its installer, its activation, or a Windows component.",
            "Use the vendor's installer or app to repair or activate the plug-in, then run the check again."),
        Some("buses") => Explanation::new("sdk_audio_buses",
            "The plug-in started, but it would not describe its audio inputs and outputs.",
            "It may not support the channel layout the bridge asked for.",
            "If the plug-in has mono and stereo versions, try the other one. Otherwise update the plug-in and run the check again."),
        Some("controller") => Explanation::new("sdk_controls",
            "The plug-in started its audio engine, but its controls could not be set up.",
            "Its control interface reported an error.",
            "Update the plug-in and run the check again. If it keeps happening, create a problem report."),
        Some("settings") => Explanation::new("sdk_settings_handoff",
            "The plug-in started, but it could not pass its starting settings from its audio engine to its controls.",
            "It answered that step in a way the bridge does not accept yet.",
            "Update the plug-in to its latest version and run the check again. If it keeps happening, create a problem report."),
        Some("list") => Explanation::new("sdk_plugin_list",
            "The plug-in file loaded, but the list of plug-ins it contains could not be read correctly.",
            "It returned incomplete or invalid information about itself.",
            "Update the plug-in to its latest version, then run the check again."),
        Some(_) => Explanation::new("multiple_plugins_in_file",
            "This file contains several plug-ins.",
            "Each one is checked on its own.",
            "Choose the plug-in you want from the list and check it."),
        None => Explanation::new("sdk_unrecognised_step",
            "The plug-in reported an error while the bridge was checking it.",
            "The step that failed is not one the bridge can describe yet.",
            "Run the check again. If it keeps happening, create a problem report."),
    }
}

fn cleanup_unconfirmed() -> Explanation {
    Explanation::new("cleanup_unconfirmed",
        "The bridge could not confirm that the Windows side closed after the check.",
        "A Windows process may still be running.",
        "Restart the manager, or your computer, before trying again.")
}

/// A recorded failure whose exact report is not retained. Never borrowed from
/// another session.
pub fn details_not_retained(code: &str) -> Explanation {
    Explanation::new("details_not_retained",
        "The plug-in check failed.",
        "The detailed record of this attempt was not kept, so the exact reason is unknown.",
        format!("Run the check again; the new attempt records why it fails.{}", code_suffix(code)))
}

/// Generic text plus the internal code when the code is a plain identifier.
pub fn unrecognised(code: &str) -> Explanation {
    Explanation::new("unrecognised",
        "Something went wrong that the bridge cannot describe yet.",
        "The detailed record is kept on this computer.",
        format!("Try again. If it fails again, create a problem report.{}", code_suffix(code)))
}

fn code_suffix(code: &str) -> String {
    let plain = !code.is_empty() && code.len() <= 64
        && code.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    if plain { format!(" (code: {code})") } else { String::new() }
}

/// The last DAW load of a plug-in was refused at this recorded startup phase
/// (`runtime/native-startup`). Only phases with a known meaning explain.
pub fn from_load_refusal(phase: Option<&str>) -> Option<Explanation> {
    Some(match phase? {
        "keeper_failed" => Explanation::new("windows_side_stopped",
            "The last time your DAW loaded this plug-in, its Windows side had stopped, so the plug-in stayed empty and silent.",
            "The Windows programs behind this plug-in's setup ended unexpectedly (for example when the desktop's display server restarted) or failed while starting.",
            "Load the plug-in again: reopen the project without saving it, or remove and add the plug-in. Saving while it is not loaded drops its settings from the project. The bridge starts its Windows side again on that load. If it failed to start more than once, wait a minute before loading it again. If it keeps failing, create a problem report."),
        _ => return None,
    })
}

/// Enumerated manager and installer outcome codes.
pub fn from_code(code: &str) -> Option<Explanation> {
    Some(match code {
        "outer_nonzero_stage_unknown" => Explanation::new(code,
            "The installer finished with an error.",
            "It did not say which step failed; some files may have been installed.",
            "Look for the plug-in in the list. If it is missing, run the installer again."),
        "partial_installation" => Explanation::new(code,
            "The installer stopped part of the way through.",
            "Some files were installed, but not everything.",
            "Run the installer again to finish the installation."),
        "cancelled" | "installation_cancelled" | "vendor_cancelled" => Explanation::new("cancelled",
            "The installation was stopped before it finished.",
            "It was cancelled.",
            "Run the installer again when you are ready."),
        "not_installed" => Explanation::new(code,
            "The installer ran, but nothing was installed.",
            "The installer may have closed early, or it installed somewhere the bridge does not look.",
            "Run the installer again and complete all of its steps."),
        "installer_launch_refused" => Explanation::new(code,
            "The installer could not be started.",
            "The bridge was not ready to start it safely.",
            "Refresh the manager and try again."),
        "no_supported_instruments_or_effects_discovered" => Explanation::new(code,
            "The installation finished, but no instruments or effects were found.",
            "The installer may have installed only an app, or the plug-ins use a format the bridge does not support yet (it uses 64-bit VST3 plug-ins).",
            "If the vendor uses its own app to download plug-ins, open that app and install them there."),
        "cleanup_unconfirmed" => cleanup_unconfirmed(),
        "inspection_incomplete" | "inspection_binding_mismatch" | "class_mismatch"
        | "metadata_mismatch" | "class_absent" | "inspection_module_changed" => Explanation::new(code,
            "The plug-in check finished, but its results were incomplete or did not match the installed plug-in.",
            "The plug-in may have changed during the check, or it reported different details each time.",
            "Find installed plug-ins again, then run the check again."),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(error: &str, records: Value) -> Value {
        json!({"error":error,"cleanup_confirmed":true,"gated":true,"raw_exit":0,
            "records":records,"vendor_stdout":"SECRET","stderr":"SECRET C:\\users\\alice"})
    }
    fn sdk(reason: &str) -> Explanation {
        from_report(&report("Windows SDK host failed: 90",
            json!([{"state":"ap8_failure","reason":reason},
                {"state":"ap8_inspection_closed","exit_code":90}]))).unwrap()
    }
    fn exited(code: i64, win32: Option<&str>) -> Explanation {
        exited_with(code, win32, &[])
    }
    fn exited_with(code: i64, win32: Option<&str>, states: &[&str]) -> Explanation {
        let mut rows: Vec<Value> = states.iter().map(|state| json!({"event":"lifecycle","state":state}))
            .collect();
        if let Some(hex) = win32 {
            let operation = if code == 80 { "free_library" } else { "load_library" };
            rows.push(json!({"event":"call_completed","operation":operation,
                "return_kind":"win32_error","win32_error_u32_hex":hex}));
        }
        let mut r = report("Windows host exited without successful close", json!(rows));
        r["exit_before_cleanup"] = json!(code);
        r["raw_exit"] = json!(code);
        from_report(&r).unwrap()
    }
    fn private(e: &Explanation) {
        let text = serde_json::to_string(e).unwrap();
        for secret in ["alice", "SECRET", "C:\\", "users"] {
            assert!(!text.contains(secret), "{secret} leaked: {text}");
        }
    }

    #[test]
    fn sdk_call_reasons_are_grouped() {
        for (reason, code) in [
            ("createComponent", "sdk_component_start"),
            ("initializeComponent", "sdk_component_start"),
            ("null audio processor", "sdk_component_start"),
            ("getBusInfo", "sdk_audio_buses"),
            ("getBusArrangement", "sdk_audio_buses"),
            ("stereo main-pair negotiation/readback", "sdk_audio_buses"),
            ("createController", "sdk_controls"),
            ("initializeController", "sdk_controls"),
            ("connectComponent", "sdk_controls"),
            ("connectController", "sdk_controls"),
            ("getParameterInfo index 17", "sdk_controls"),
            ("parameter count bound", "sdk_controls"),
            ("initial controller synchronization failed", "sdk_settings_handoff"),
            ("initial state SDK failure", "sdk_settings_handoff"),
            ("getClassInfo", "sdk_plugin_list"),
            ("audio class absent", "sdk_plugin_list"),
            ("multiple audio classes require explicit selection", "multiple_plugins_in_file"),
            ("std::bad_alloc", "sdk_unrecognised_step"),
        ] {
            let e = sdk(reason);
            assert_eq!(e.code, code, "{reason}");
            assert!(!e.what_happened.is_empty() && !e.likely_cause.is_empty()
                && !e.what_to_try.is_empty());
            private(&e);
        }
        let mut missing = report("Windows SDK host failed: 90", json!([]));
        missing["records"] = Value::Null;
        assert_eq!(from_report(&missing).unwrap().code, "sdk_unrecognised_step");
        assert_eq!(from_report(&report("Windows SDK host failed: 91", json!([]))).unwrap().code,
            "sdk_shutdown");
    }

    /// Every reason the Windows host's ordinary check can record has its own
    /// sentence, so a musician never reads "not one the bridge can describe".
    #[test]
    fn every_check_step_the_windows_host_records_is_explained() {
        let inspect = include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
            "/../windows-factory-probe/source/inspect_module.cpp"));
        let state = include_str!(concat!(env!("CARGO_MANIFEST_DIR"),
            "/../windows-factory-probe/source/ap8_state.h"));
        let quoted_after = |text: &str, marker: &str| -> Vec<String> {
            text.match_indices(marker).filter_map(|(at, _)| {
                let rest = &text[at + marker.len()..];
                let end = rest.find('"')?;
                Some(rest[..end].to_owned())
            }).collect()
        };
        let mut reasons = quoted_after(inspect, "runtime_error(\"");
        reasons.extend(quoted_after(inspect, ",\"").into_iter()
            .filter(|name| !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric())));
        // Written with the failing parameter's index appended.
        for reason in &mut reasons {
            if reason == "getParameterInfo index " { reason.push('7'); }
        }
        let initial = state.split("inline bool synchronize_initial").nth(1).unwrap()
            .split("\n}").next().unwrap();
        reasons.extend(quoted_after(initial, ",\""));
        // Not part of the ordinary check: the vendor-access window, the
        // separate graphics assessment, and audio in a running DAW session.
        let elsewhere = ["vendor access editor open failed", "vendor access title encoding",
            "vendor access title conversion", "assessGraphicsEditor", "commercial processing failed"];
        reasons.retain(|reason| !elsewhere.contains(&reason.as_str()));
        assert!(reasons.len() >= 40, "{reasons:?}");
        let unexplained: Vec<_> = reasons.iter().filter(|reason| sdk_group(reason).is_none()).collect();
        assert!(unexplained.is_empty(), "no plain explanation for host reasons {unexplained:?}");
    }

    #[test]
    fn host_exit_codes_and_load_errors_are_explained() {
        // 64 is also the bridge's own helper failing its hash check: neutral.
        let start = exited(64, None);
        assert_eq!(start.code, "start_verification_failed");
        assert!(start.what_happened.contains("Windows helper or the plug-in file"));
        assert_eq!(exited_with(65, None, &["readiness_announced"]).code, "plugin_file_changed");
        assert_eq!(exited(65, None).code, "host_exit_unrecognised");
        let missing = exited(70, Some("0000007e"));
        assert_eq!(missing.code, "module_dependency_missing");
        assert!(missing.likely_cause.contains("Visual C++ runtime"));
        let format = exited(70, Some("000000c1"));
        assert_eq!(format.code, "module_bad_format");
        assert!(format.likely_cause.starts_with("It may be a 32-bit plug-in"));
        assert!(format.likely_cause.contains("damaged"));
        assert_eq!(exited(70, Some("0000007f")).code, "module_dependency_mismatch");
        let other = exited(70, Some("00000005"));
        assert_eq!(other.code, "module_load_failed");
        assert!(other.likely_cause.contains("0x00000005"));
        // Without the host's own LoadLibrary record, 70 is not explained.
        assert_eq!(exited(70, None).code, "host_exit_unrecognised");
        assert_eq!(exited(70, Some("C:\\users\\alice")).code, "host_exit_unrecognised");
        // Exit codes alone never select a specific sentence.
        for (code, state, expected) in [
            (71, "module_entry_failed", "module_entry_failed"),
            (72, "factory_export_missing", "factory_export_missing"),
            (73, "factory_get_failed", "factory_null"),
            (74, "factory_obtained", "factory_enumeration"),
            (78, "factory_obtained", "factory_enumeration"),
            (79, "module_exit_failed", "module_exit_failed"),
        ] {
            assert_eq!(exited_with(code, None, &[state]).code, expected, "{code}");
            let bare = exited(code, None);
            assert_eq!(bare.code, "host_exit_unrecognised", "{code}");
            assert!(bare.what_happened.contains(&format!("exit code {code}")));
        }
        assert_eq!(exited(80, Some("00000005")).code, "module_unload_failed");
        assert_eq!(exited(80, None).code, "host_exit_unrecognised");
        let forced = exited(92, None);
        assert_eq!(forced.code, "forced_stop");
        for e in [&exited(79, None), &exited(80, None), &forced] {
            assert!(!e.likely_cause.contains("while starting"), "{e:?}");
        }
        let unknown = exited(139, None);
        assert_eq!(unknown.code, "host_exit_unrecognised");
        assert!(unknown.what_happened.contains("139"));
        for e in [missing, format, other, unknown] { private(&e); }
        // An exit status from the SDK host itself is used when no outer code exists.
        let mut closed = report("Windows host exited without successful close",
            json!([{"state":"factory_get_failed"},{"state":"ap8_inspection_closed","exit_code":73}]));
        closed["raw_exit"] = Value::Null;
        assert_eq!(from_report(&closed).unwrap().code, "factory_null");
        // A clean SDK close and no outer status never reads "exit code 0".
        let mut zero = report("Windows host exited without successful close",
            json!([{"state":"ap8_inspection_closed","exit_code":0}]));
        zero["raw_exit"] = json!(0);
        let zero = from_report(&zero).unwrap();
        assert_eq!(zero.code, "host_exit_unrecognised");
        assert!(!zero.what_happened.contains("exit code"), "{}", zero.what_happened);
    }

    #[test]
    fn timeouts_cleanup_and_unknown_errors() {
        let e = from_report(&report("TimeoutError: Windows startup deadline", json!([]))).unwrap();
        assert_eq!(e.code, "timeout_startup");
        let e = from_report(&report("TimeoutError: Windows call deadline: load_library", json!([]))).unwrap();
        assert_eq!(e.code, "timeout_call");
        assert!(e.what_happened.contains("loading the plug-in file"));
        for (operation, phrase) in [("get_bus_info", "audio inputs"), ("GetBusArrangement", "audio inputs"),
            ("countClasses", "listing the plug-ins"), ("get_plugin_factory", "listing the plug-ins"),
            ("closeGraphicsEditor", "editor window")] {
            let e = from_report(&report(&format!("TimeoutError: Windows call deadline: {operation}"),
                json!([]))).unwrap();
            assert!(e.what_happened.contains(phrase), "{operation}: {}", e.what_happened);
        }
        let e = from_report(&report("TimeoutError: Windows call deadline: C:\\users\\alice\\SECRET", json!([]))).unwrap();
        assert_eq!(e.what_happened, "The plug-in stopped responding during a plug-in call.");
        private(&e);
        let e = from_report(&report("TimeoutError: inspection deadline; cleanup: x", json!([]))).unwrap();
        assert_eq!(e.code, "timeout_inspection");
        let mut unclean = report("TimeoutError: inspection deadline", json!([]));
        unclean["cleanup_confirmed"] = json!(false);
        assert!(from_report(&unclean).unwrap().what_to_try.contains("could not confirm"));
        let mut only = json!({"error":null,"cleanup_confirmed":false});
        assert_eq!(from_report(&only).unwrap().code, "cleanup_unconfirmed");
        only["cleanup_confirmed"] = json!(true);
        assert!(from_report(&only).is_none());
        let e = from_report(&report("RuntimeError: C:\\users\\alice SECRET", json!([]))).unwrap();
        assert_eq!(e.code, "check_failed_unrecognised");
        private(&e);
        let e = from_report(&report("Windows SDK host failed: 90",
            json!([{"state":"ap8_failure","reason":"createComponent C:\\users\\alice"}]))).unwrap();
        assert_eq!(e.code, "sdk_unrecognised_step");
        private(&e);
        let e = from_report(&report("Windows SDK host failed: 90",
            json!([{"state":"ap8_failure","reason":"getParameterInfo index C:\\users\\alice"}]))).unwrap();
        assert_eq!(e.code, "sdk_unrecognised_step");
        private(&e);
    }

    #[test]
    fn installer_outcomes_and_codes() {
        for code in ["outer_nonzero_stage_unknown", "partial_installation", "cancelled",
            "not_installed", "installer_launch_refused",
            "no_supported_instruments_or_effects_discovered"] {
            let e = from_code(code).unwrap();
            assert!(!e.what_happened.is_empty() && !e.what_to_try.is_empty(), "{code}");
        }
        assert!(from_code("C:\\users\\alice").is_none());
        let e = details_not_retained("inspection_failed");
        assert_eq!(e.code, "details_not_retained");
        assert!(e.what_to_try.ends_with("(code: inspection_failed)"));
        let e = unrecognised("Error at C:\\users\\alice: SECRET");
        private(&e);
        assert!(unrecognised("lock_timeout").what_to_try.contains("lock_timeout"));
        let found = sdk("createComponent");
        let shown = details_not_retained("inspection_failed").or_first_found(Some(found.clone()));
        assert_eq!(shown.code, "sdk_component_start");
        assert!(shown.summary().ends_with("(Recorded when this plug-in was first found.)"));
        // A check's own record always wins over the discovery record.
        assert_eq!(sdk("getBusInfo").or_first_found(Some(found)).code, "sdk_audio_buses");
        assert_eq!(details_not_retained("x").or_first_found(None).code, "details_not_retained");
    }

    #[test]
    fn exact_report_binding_is_required() {
        let dir = std::env::temp_dir().join(format!("lvb-problem-{}", crate::random_id().unwrap()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("windows-report.json");
        let value = report("Windows SDK host failed: 90",
            json!([{"state":"ap8_failure","reason":"createComponent"}]));
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        let exact = Artifact { path: path.clone(), sha256: crate::digest(&path).unwrap() };
        assert_eq!(from_artifact(&exact, "inspection_failed").code, "sdk_component_start");
        let other = Artifact { path: path.clone(), sha256: "00".repeat(32) };
        assert_eq!(from_artifact(&other, "inspection_failed").code, "details_not_retained");
        let gone = Artifact { path: dir.join("missing.json"), sha256: exact.sha256.clone() };
        assert_eq!(from_artifact(&gone, "inspection_failed").code, "details_not_retained");
        let bound = BoundFailure::bind("inspection_failed".into(), &"ab".repeat(16), &path);
        assert_eq!(bound.to_string(), "inspection_failed");
        let bound = bound.downcast_ref::<BoundFailure>().unwrap();
        assert_eq!(bound.report, exact);
        let unbound = BoundFailure::bind("inspection_failed".into(), &"ab".repeat(16),
            &dir.join("missing.json"));
        assert!(unbound.downcast_ref::<BoundFailure>().is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
