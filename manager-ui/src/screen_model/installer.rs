//! Words for retained installer records: what an installer run left behind,
//! its scripting policy, and why environment creation did not start. Every
//! line restates a manager record; none claims more than the record shows.

/// How completely an installer run is known to have installed.
pub fn installer_installation_label(t: &serde_json::Value) -> String {
    if t["durable_installation"] == "partial_installation"
        && t["installation_completeness"] == "unproved"
    {
        "Installation changes observed; completion not verified".into()
    } else {
        format!("Installation observation: {}",
            t["durable_installation"].as_str().unwrap_or("unavailable").replace('_', " "))
    }
}

/// The retained result of one installer run, most important first.
pub fn installer_lines(v: &serde_json::Value) -> Vec<String> {
    let t = &v["transaction"];
    if t["schema"] != 1 || t["operation"] != v["operation"] {
        return if v["error"] == "installer_launcher_failed" {
            vec!["Earlier installer result: outer launch route exited nonzero; the failing later stage and installation completeness were not captured.".into()]
        } else {
            vec![]
        };
    }
    let installation = installer_installation_label(t);
    let outcome=match t["outcome"].as_str() {
        Some("in_progress")=>"Installer supervision is ongoing. Exact Focus and Stop controls remain available; cleanup will be checked after retirement.",
        Some("outer_nonzero_stage_unknown")=>"The outer installer route exited nonzero. The failing child or stage is not established.",
        Some("installed_dependency_failed")=>"Application files are installed, but a process performing service/dependency work exited nonzero. Review that stage before reinstalling.",
        Some("child_failed")=>"An owned child exited nonzero. Its role and underlying cause may still be unknown.",
        Some("cancelled")=>"This attempt was cancelled. Earlier failure observations remain retained.",
        Some("cleanup_unconfirmed")=>"Installer cleanup is unconfirmed. Further work is blocked.",
        Some("installed")=>"Application files and installation registration were observed. First launch and dependency health remain unproved.",
        Some("partial_installation") if t["durable_installation"] == "partial_installation"
            && t["installation_completeness"] == "unproved"=>installation.as_str(),
        Some("partial_installation")=>"Partial installation: durable changes exist, but a complete application installation is not established.",
        Some("not_installed")=>"No durable installation was found in the inspected surfaces.",
        _=>"Review the installer stage record; completion does not qualify or publish a plug-in.",
    };
    let mut lines = vec![outcome.into()];
    if let Some(code) = v["startup"]["first_problem"]["code"].as_str() {
        let observation = match code {
            "native_steamclient_load_failed" => "native runtime dependency load failed",
            "native_steamclient_export_unavailable" => "native runtime export unavailable",
            "runtime_assertion_observed" => "runtime assertion observed",
            "prefix_initialization_failed" | "prefix_initialization_timeout" => {
                "environment initialization did not complete"
            }
            _ => "startup problem retained; inspect bounded details",
        };
        lines.push(format!(
            "Earlier startup observation: {observation}. Cancellation and cleanup do not erase it."
        ));
    }
    if let Some(n) = t["outer_launcher_exit"].as_i64() {
        lines.push(format!(
            "Outer launcher exit: {n} (separate from payload and service exits)"
        ));
    }
    if outcome != installation.as_str() {
        lines.push(installation);
    }
    if !t["first_failure"].is_null() {
        let f = &t["first_failure"];
        lines.push(format!("First retained process result: phase {} · role {} · relationship {} · domain {} · status {} · cause unestablished",
            f["phase"].as_str().unwrap_or("unknown"), f["role"].as_str().unwrap_or("unknown"),
            f["relationship"].as_str().unwrap_or("unknown"), f["domain"].as_str().unwrap_or("unknown"), f["status"]));
    }
    let binding = &t["launch_binding"];
    if binding["schema"] == 1 && binding["operation"] == v["operation"] {
        lines.push(if binding["status"] == "bound" {
            "Installer Windows root: exact operation and launch generation observed.".into()
        } else {
            format!(
                "Installer Windows root unavailable: {}. Child attribution may be incomplete.",
                binding["reason"].as_str().unwrap_or("missing observation")
            )
        });
    }
    if t["presence_close"]["schema"] == 1 {
        lines.push(format!("Application presence/close requests observed: {}. Helper success does not prove a match or successful closure; exact match and recheck results remain unavailable.",t["presence_close"]["observation_count"]));
        if t["presence_close"]["operation_classes"]
            .as_array()
            .is_some_and(|rows| {
                rows.iter()
                    .any(|r| r == "presence_query" || r == "close_request")
            })
        {
            lines.push("Observed request mechanism: PowerShell/CIM process query or script process close. The matched object and actual close outcome are unavailable.".into());
        }
    }
    lines.push(
        if v["cleanup_confirmed"] == true {
            "Owned process cleanup confirmed."
        } else {
            "Owned process cleanup not yet confirmed."
        }
        .into(),
    );
    lines
}

/// The requested and applied scripting policy of one installer run.
pub fn installer_policy_lines(v: &serde_json::Value) -> Vec<&'static str> {
    let Some(policy) = v.get("installer_capability") else {
        return vec![];
    };
    let requested = policy["requested"]["powershell"].as_str();
    let applied = policy["effective"]["effective"]["windows_scripting"]["powershell"].as_str();
    vec![match requested {
        Some("intentionally_unavailable")=>"Requested: PowerShell intentionally unavailable for this installer only.",
        Some("inherited")=>"Requested: inherited scripting behavior.",
        _=>"Scripting policy request could not be verified.",
    }, match applied {
        Some("intentionally_unavailable")=>"Applied at target launch: PowerShell intentionally unavailable. Installer fallback success is not yet established.",
        Some("inherited")=>"Applied at target launch: inherited scripting behavior.",
        _=>"Target-launch policy has not been confirmed applied.",
    }]
}

/// Why environment creation did not start, and whether retrying is safe.
pub fn failure_lines(f: &crate::model::OperationFailure) -> Vec<String> {
    let mut lines = vec![match f.stage {
        crate::model::FailureStage::OperatorValidationReadback => {
            "Manager could not finish validating state."
        }
        crate::model::FailureStage::EnvironmentCreationAdmission => {
            "Manager was busy before environment creation could start."
        }
    }
    .into()];
    if !f.mutation_started && !f.environment_created {
        lines.push("Environment creation did not start.".into());
    }
    if !f.installer_launched {
        lines.push("The installer was not launched; this is not an installer failure.".into());
    }
    lines.push(
        if f.retryable {
            "Retry is safe once the manager refreshes and canonical state is healthy."
        } else {
            "Manager access needs attention before retrying."
        }
        .into(),
    );
    lines.push(format!(
        "Validation lock: {:?}; {:?}; {} attempts, {:.1} ms elapsed; holder unknown.",
        f.lock.name,
        f.lock.outcome,
        f.lock.attempts,
        f.lock.elapsed_wait_us as f64 / 1000.0
    ));
    lines
}
