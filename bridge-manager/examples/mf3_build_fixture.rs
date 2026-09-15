//! Development-only exercise of the production preparation/publication owner.
//! Source-owned Windows CI report and module only. Never install this example.
use linux_vst_bridge::{catalogue::Software, inventory, observation, preparation as prep, *};
use std::{
    env, fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
fn artifact(path: PathBuf) -> Result<Artifact> {
    Ok(Artifact {
        sha256: digest(&path)?,
        path,
    })
}
fn copy(from: &Path, to: &Path) -> Result<Artifact> {
    fs::copy(from, to)?;
    fs::set_permissions(to, fs::Permissions::from_mode(0o400))?;
    artifact(to.into())
}
fn main() -> Result<()> {
    unsafe {
        libc::umask(0o077);
    }
    let args: Vec<_> = env::args().collect();
    require(
        args.len() == 6,
        "fixture: NEW_ROOT KIT WINDOWS_PACKAGE SOFTWARE_JSON RUNNER_JSON",
    )?;
    let root = PathBuf::from(&args[1]);
    require(!root.exists(), "fixture root already exists")?;
    private_dir(&root)?;
    let root = root.canonicalize()?;
    let m = Manager {
        publications: root.join("private-publications"),
        root,
    };
    let mut sw: Software = read_json(Path::new(&args[4]))?;
    let runner: Runner = read_json(Path::new(&args[5]))?;
    runner.verify()?;
    let software = m.root.join("software/fixture");
    private_dir(&software)?;
    sw.preparation_kit = Some(copy(
        Path::new(&args[2]),
        &software.join("preparation-kit.zip"),
    )?);
    atomic_json(&m.root.join("software.json"), &sw)?;
    let runtime = prep::build::stage_runtime(&m)?;
    let environment = Environment {
        id: random_id()?,
        root: PathBuf::new(),
        runner,
        revision: 1,
    };
    let environment = Environment {
        root: m.root.join("environments").join(&environment.id),
        ..environment
    };
    let drive = environment.root.join("compatdata/pfx/drive_c");
    private_dir(&drive)?;
    atomic_json(&environment.root.join("environment.json"), &environment)?;
    let package = Path::new(&args[3]);
    let module = copy(
        &package.join("ap10-return-fixture.vst3"),
        &drive.join("fixture.vst3"),
    )?;
    let report = copy(
        &package.join("mf3-source-inspection.json"),
        &m.root.join("source-inspection.json"),
    )?;
    let raw: serde_json::Value = read_json(&report.path)?;
    let classes = inventory::classes(&raw)?;
    let class = classes
        .iter()
        .find(|c| c.category == "Audio Module Class")
        .ok_or("audio class absent")?
        .clone();
    let scan = inventory::Scan {
        schema: 1,
        id: random_id()?,
        environment: environment.clone(),
        host: runtime.host.clone(),
        host_source_sha256: runtime.source_manifest.sha256.clone(),
        completed_at: observation::now()?,
        modules: vec![inventory::Module {
            artifact: module.clone(),
            classes,
            report: report.clone(),
            inspection_error: None,
            quarantine_reason: None,
        }],
        changes: Default::default(),
    };
    private_dir(&m.root.join("inventory"))?;
    atomic_json(
        &m.root
            .join("inventory")
            .join(format!("{}.json", environment.id)),
        &scan,
    )?;
    let selection = prep::Selection {
        schema: 1,
        environment,
        module,
        class,
        scanner: runtime.host.clone(),
        scanner_source: runtime.source_manifest.sha256.clone(),
        factory_report: report.clone(),
    };
    let selected = prep::select(
        &m,
        &selection.id()?,
        &runtime.host,
        &runtime.source_manifest.sha256,
    )?;
    require(selected == selection, "selection differs")?;
    let inspection = prep::inspect_record_with(
        selection.clone(),
        report,
        prep::Origin::ManagedPreparation,
        runtime.host.clone(),
        runtime.source_manifest.clone(),
    )?;
    prep::retain_inspection(&m, &inspection)?;
    let operation = random_id()?;
    let candidate = prep::build::construct(
        &m,
        selection,
        inspection,
        runtime.host.clone(),
        runtime.source_manifest.clone(),
        &operation,
    )?;
    prep::verify_candidate(
        &m,
        &candidate,
        &runtime.host,
        &runtime.source_manifest.sha256,
    )?;
    let id = prep::record_candidate(&m, &candidate)?;
    require(
        prep::record_candidate(&m, &candidate)? == id,
        "duplicate candidate",
    )?;
    require(m.registry()?.classes.is_empty(), "preparation published")?;
    let revision = prep::enable(&m, &candidate, false)?;
    require(
        prep::publication_state(&m, &candidate)? == "experimental",
        "experimental missing",
    )?;
    require(
        prep::accepted(&m, &candidate).is_err(),
        "unqualified ordinary allowed",
    )?;
    prep::disable(&m, &candidate)?;
    require(
        prep::publication_state(&m, &candidate)? == "removed",
        "experimental removal failed",
    )?;
    prep::build::cleanup_work(&m, &operation)?;
    let result = serde_json::json!({"schema":1,"source_owned":true,"native_sha256":candidate.native.artifact.sha256,"descriptor_sha256":candidate.native.descriptor_sha256,"candidate":id,"revision":revision.id,"exact_controller":candidate.inspection.controller,"prepare_did_not_publish":true,"experimental_enable_remove":true,"ordinary_refused":true,"cleanup":true,"commercial_product_run":false});
    atomic_json(&m.root.join("fixture-result.json"), &result)?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
