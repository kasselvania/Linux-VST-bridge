//! Read-only exact candidate-C/D identity preview; no manager or vendor process.
use linux_vst_bridge::{preparation, Artifact, Environment, Runner, RunnerPolicy};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn sha(bytes: &[u8]) -> String {
    linux_vst_bridge::hex(&Sha256::digest(bytes))
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 && args.len() != 4 {
        return Err("usage: x11_touch_candidate_preview PREDECESSOR_CANDIDATE ENVIRONMENT RUNNER_MANIFEST [ORIGINAL_MANIFEST_PATH]".into());
    }
    let predecessor_path = PathBuf::from(&args[0]);
    let environment_path = PathBuf::from(&args[1]);
    let manifest_path = PathBuf::from(&args[2]);
    let original_manifest_path = args.get(3).map(PathBuf::from).unwrap_or_else(|| manifest_path.clone());
    if !original_manifest_path.is_absolute() {
        return Err("touch_preview_manifest_path".into());
    }
    let predecessor: preparation::Candidate = serde_json::from_slice(&fs::read(&predecessor_path)?)?;
    let before: Environment = serde_json::from_slice(&fs::read(&environment_path)?)?;
    let manifest_bytes = fs::read(&manifest_path)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    let routing = match manifest["kind"].as_str() {
        Some("x11_touch_release_reference_runner") => false,
        Some("x11_touch_routing_reference_runner") => true,
        _ => return Err("touch_preview_manifest_kind".into()),
    };
    let predecessor_id = if routing { preparation::SERUM_TOUCH_ROUTING_PREDECESSOR }
        else { preparation::SERUM_TOUCH_PREDECESSOR };
    if predecessor.id()? != predecessor_id
        || predecessor.selection.environment != before
        || manifest["environment"] != before.id
    {
        return Err("touch_preview_predecessor_or_manifest".into());
    }
    let mut runner: Runner = serde_json::from_value(manifest["runner"].clone())?;
    runner.policy = Some(if routing { RunnerPolicy::X11TouchRoutingV2 }
        else { RunnerPolicy::X11TouchReleaseV1 });
    if args.len() == 3 {
        runner.verify()?;
    }
    let mut after = before.clone();
    after.revision = before.revision.checked_add(1).ok_or("touch_preview_revision")?;
    after.runner = runner;
    let managed_root = before.root.parent().and_then(|path| path.parent())
        .ok_or("touch_preview_manager_root")?;
    let transition_path = managed_root.join("private-rollback")
        .join(format!("experimental-runner-{}-{}", if routing { "routing" } else { "touch" }, before.id))
        .join("result.json");
    let manifest_sha256 = sha(&manifest_bytes);
    let result = json!({
        "schema": 1,
        "state": "completed",
        "environment": after.id,
        "revision": after.revision,
        "runner_sha256": sha(&serde_json::to_vec(&after.runner)?),
        "candidate_manifest_sha256": manifest_sha256,
        "carried_predecessor": predecessor_id
    });
    let mut result_bytes = serde_json::to_vec(&result)?;
    result_bytes.push(b'\n');
    let provenance = preparation::TouchCarryForward {
        predecessor: predecessor_id.into(),
        transition: Artifact { path: transition_path, sha256: sha(&result_bytes) },
        runner_manifest: Artifact { path: original_manifest_path, sha256: manifest_sha256 },
    };
    let candidate = if routing { preparation::touch_routing_successor(&predecessor, &after, provenance)? }
        else { preparation::touch_successor(&predecessor, &after, provenance)? };
    let mut output = json!({
        "profile_sha256": candidate.profile.fingerprint()?,
        "transition_result_sha256": sha(&result_bytes),
        "predecessor": predecessor_id,
        "current_inventory_relabelled": false,
        "runner_files_verified_in_preview": args.len() == 3
    });
    output[if routing { "candidate_d" } else { "candidate_c" }] = json!(candidate.id()?);
    println!("{output}");
    Ok(())
}
