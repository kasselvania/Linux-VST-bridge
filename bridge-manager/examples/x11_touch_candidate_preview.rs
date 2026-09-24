//! Read-only exact candidate-C identity preview; no manager or vendor process.
use linux_vst_bridge::{preparation, Artifact, Environment, Runner, RunnerPolicy};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn sha(bytes: &[u8]) -> String {
    linux_vst_bridge::hex(&Sha256::digest(bytes))
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: x11_touch_candidate_preview B_CANDIDATE ENVIRONMENT RUNNER_MANIFEST".into());
    }
    let predecessor_path = PathBuf::from(&args[0]);
    let environment_path = PathBuf::from(&args[1]);
    let manifest_path = PathBuf::from(&args[2]);
    let predecessor: preparation::Candidate = serde_json::from_slice(&fs::read(&predecessor_path)?)?;
    let before: Environment = serde_json::from_slice(&fs::read(&environment_path)?)?;
    let manifest_bytes = fs::read(&manifest_path)?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)?;
    if predecessor.id()? != preparation::SERUM_TOUCH_PREDECESSOR
        || predecessor.selection.environment != before
        || manifest["kind"] != "x11_touch_release_reference_runner"
        || manifest["environment"] != before.id
    {
        return Err("touch_preview_predecessor_or_manifest".into());
    }
    let mut runner: Runner = serde_json::from_value(manifest["runner"].clone())?;
    runner.policy = Some(RunnerPolicy::X11TouchReleaseV1);
    runner.verify()?;
    let mut after = before.clone();
    after.revision = before.revision.checked_add(1).ok_or("touch_preview_revision")?;
    after.runner = runner;
    let managed_root = before.root.parent().and_then(|path| path.parent())
        .ok_or("touch_preview_manager_root")?;
    let transition_path = managed_root.join("private-rollback")
        .join(format!("experimental-runner-touch-{}", before.id))
        .join("result.json");
    let manifest_sha256 = sha(&manifest_bytes);
    let result = json!({
        "schema": 1,
        "state": "completed",
        "environment": after.id,
        "revision": after.revision,
        "runner_sha256": sha(&serde_json::to_vec(&after.runner)?),
        "candidate_manifest_sha256": manifest_sha256,
        "carried_predecessor": preparation::SERUM_TOUCH_PREDECESSOR
    });
    let mut result_bytes = serde_json::to_vec(&result)?;
    result_bytes.push(b'\n');
    let candidate = preparation::touch_successor(
        &predecessor,
        &after,
        preparation::TouchCarryForward {
            predecessor: preparation::SERUM_TOUCH_PREDECESSOR.into(),
            transition: Artifact { path: transition_path, sha256: sha(&result_bytes) },
            runner_manifest: Artifact { path: manifest_path, sha256: manifest_sha256 },
        },
    )?;
    println!("{}", json!({
        "candidate_c": candidate.id()?,
        "profile_sha256": candidate.profile.fingerprint()?,
        "transition_result_sha256": sha(&result_bytes),
        "predecessor": preparation::SERUM_TOUCH_PREDECESSOR,
        "current_inventory_relabelled": false
    }));
    Ok(())
}
