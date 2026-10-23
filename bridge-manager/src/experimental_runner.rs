//! Read-only interpretation of retained experimental-runner records: the BG1
//! transition history behind its installed runner.
use super::*;
use serde::Deserialize;
use serde_json::Value;
use std::fs::{File, OpenOptions};
use std::os::unix::fs::MetadataExt;

// The selected BG1 V4 environment predates public ownership of its native
// command session. These are the three completed, retained transitions that
// led to that exact installed runner. This is a read-only interpretation of
// history, not permission to create another BG1 runner or execute a rollback.
const BG1_ENVIRONMENT: &str = "7d8fce354e68595cdc20485f754a892d";
const BG1_CLASS: &str = "ABCDEF019182FAEB4C756E6170726F43";
pub(super) const BG1_V4_RUNNER: &str = "proton-11.0-2c-dcomp-bg1-v4";
const BG1_V4_COMPONENT: &str =
    "1fca37746647a975d80666f75f84c050770820b2c0f7de58352bbf7ed4f40c30";

struct Bg1Step<'a> {
    before_revision: u64,
    before_runner: &'a str,
    before_key: &'a str,
    after_runner: &'a str,
    after_key: &'a str,
    manifest: &'a str,
    tree: &'a str,
    predecessor: &'a str,
    removed_id: &'a str,
    removed_sha256: &'a str,
}

const BG1_STEPS: [Bg1Step<'static>; 3] = [
    Bg1Step {
        before_revision: 1,
        before_runner: "proton-11.0-2c-25118279-slr4-4.0.20260805.254769",
        before_key: "72d6da6c18cd30cb9027952cbbcc7ca2e7753c86933e310a7c8db32d8261be63",
        after_runner: "proton-11.0-2c-dcomp-bg1-v1",
        after_key: "12e13d6e12a0e40c01c6a6d6efc6a5eeb1c289d7f319a51ae25e391762839bae",
        manifest: "e7fc21b89f0646c60ed2fa6704aa9c1ef17777788381ed872ce13990d38e4fd4",
        tree: "ff499951bdb395fa9824642dd3ea92357e63ab059eea11d8ec211d4a9afbd420",
        predecessor: "efa58d7bcf2c790125a4a17b6727226f537f56c4b09b9031a4616f4753b909c2",
        removed_id: "5dd30d9ba5a89686573a3014acec5b7a",
        removed_sha256: "af4c06a4fcec49d57438223a4f63b45f2773e9e4b684b861a4aa692d938591f8",
    },
    Bg1Step {
        before_revision: 2,
        before_runner: "proton-11.0-2c-dcomp-bg1-v1",
        before_key: "12e13d6e12a0e40c01c6a6d6efc6a5eeb1c289d7f319a51ae25e391762839bae",
        after_runner: "proton-11.0-2c-dcomp-bg1-v3",
        after_key: "fd44190349394d8d63825b5b3a02b4061e15f30f9fdc00024911a64592e2018a",
        manifest: "a876a7df03c2abf0f23a8f71d658137f6a0c2d63d7ef6898d01023fb8926b047",
        tree: "9a85a60e33690019f062481caa51e5703dcf6fc8f1213ae883fb7c3e800d065b",
        predecessor: "b75b10fe131cd9da79c3e6e0ba093dea608022a713cfc29842c6c0dc3f379cc5",
        removed_id: "841cba2c67ef5d9c8538e0b52a2b550c",
        removed_sha256: "3541227f76370d6679d5e7eeb6d0feed1595930a6083070028483df3e93ebb47",
    },
    Bg1Step {
        before_revision: 3,
        before_runner: "proton-11.0-2c-dcomp-bg1-v3",
        before_key: "fd44190349394d8d63825b5b3a02b4061e15f30f9fdc00024911a64592e2018a",
        after_runner: BG1_V4_RUNNER,
        after_key: "b3b6fdba60f0c41d780a80e129e0ebdb523a9a440249d64d476e71dec1baa310",
        manifest: "324b41cfeba3f1dd43dc0225657313ebb801d4f245cef3fe9e46d8a64f6e0494",
        tree: "5306487660c41c2c7f560147e87bc35501bc956151d14cac3844602d9e9db3c1",
        predecessor: "331f0fa94c3145809b0d6547c58342ac541964063f04c56c6ae81b60473e0419",
        removed_id: "61590014efdc6b0cce7edf0c6890cf66",
        removed_sha256: "8d7681f2cfbbbd1664f47101c4ac5ccbed289b0de01c0d9695d23ae39ea2c796",
    },
];

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Bg1Transition {
    schema: u32,
    state: String,
    environment: String,
    before_revision: u64,
    before_runner_sha256: String,
    after_revision: u64,
    after_runner_sha256: String,
    candidate_manifest_sha256: String,
    candidate_tree_sha256: String,
    retired_class: String,
    predecessor_candidate: String,
    removed_publication: publication::RevisionRef,
    prefix_recreated: bool,
    installation_changed: bool,
    historical_results_rewritten: bool,
    publication_carried_forward: bool,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Bg1TransitionResult {
    schema: u32,
    state: String,
    environment: String,
    revision: u64,
    runner_sha256: String,
    candidate_manifest_sha256: String,
}

fn verify_bg1_step(
    transition: &Bg1Transition,
    result: &Bg1TransitionResult,
    step: &Bg1Step<'_>,
    environment: &str,
    class: &str,
) -> Result<()> {
    require(
        transition.schema == 1
            && transition.state == "prepared"
            && transition.environment == environment
            && transition.before_revision == step.before_revision
            && transition.before_runner_sha256 == step.before_key
            && transition.after_revision == step.before_revision + 1
            && transition.after_runner_sha256 == step.after_key
            && transition.candidate_manifest_sha256 == step.manifest
            && transition.candidate_tree_sha256 == step.tree
            && transition.retired_class == class
            && transition.predecessor_candidate == step.predecessor
            && transition.removed_publication.id == step.removed_id
            && transition.removed_publication.sha256 == step.removed_sha256
            && !transition.prefix_recreated
            && !transition.installation_changed
            && !transition.historical_results_rewritten
            && !transition.publication_carried_forward
            && result.schema == 1
            && result.state == "completed"
            && result.environment == environment
            && result.revision == transition.after_revision
            && result.runner_sha256 == step.after_key
            && result.candidate_manifest_sha256 == step.manifest,
        "bg1_retained_transition_binding",
    )
}

// Rollback history is input, not a directory we are authorized to create.
// Keep the directory descriptor open so a replacement cannot pass the final
// identity check while the original inode is still in use.
struct ExistingPrivateDirectory {
    path: PathBuf,
    descriptor: File,
}

impl ExistingPrivateDirectory {
    fn open(path: &Path) -> Result<Self> {
        let before = fs::symlink_metadata(path)?;
        require(path.is_absolute() && before.file_type().is_dir()
            && before.uid() == unsafe { libc::getuid() }
            && before.mode() & 0o077 == 0
            && path.canonicalize()? == path,
            "bg1_retained_directory_identity")?;
        let descriptor = OpenOptions::new().read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY).open(path)?;
        let selected = Self { path: path.to_owned(), descriptor };
        selected.verify()?;
        Ok(selected)
    }

    fn verify(&self) -> Result<()> {
        let path = fs::symlink_metadata(&self.path)?;
        let opened = self.descriptor.metadata()?;
        require(self.path.canonicalize()? == self.path
            && path.file_type().is_dir() && opened.is_dir()
            && path.uid() == unsafe { libc::getuid() }
            && opened.uid() == unsafe { libc::getuid() }
            && path.mode() & 0o077 == 0 && opened.mode() & 0o077 == 0
            && path.dev() == opened.dev() && path.ino() == opened.ino(),
            "bg1_retained_directory_changed")
    }
}

struct Bg1RollbackDirectories {
    root: ExistingPrivateDirectory,
    steps: [Option<(ExistingPrivateDirectory, Bg1Transition)>; 3],
}

fn bg1_rollback_directories(root: &Path, environment: &str) -> Result<Bg1RollbackDirectories> {
    let mut directories = [None, None, None];
    let rollback = ExistingPrivateDirectory::open(&root.join("private-rollback"))?;
    let mut count = 0;
    for entry in fs::read_dir(&rollback.path)? {
        count += 1;
        require(count <= 64, "bg1_retained_rollback_extent")?;
        let path = entry?.path();
        let name = path.file_name().and_then(|name| name.to_str()).ok_or("bg1_retained_rollback_name")?;
        if !name.starts_with("experimental-runner-") || name.starts_with("experimental-runner-touch-")
            || name.starts_with("experimental-runner-routing-") {
            continue;
        }
        let selected = ExistingPrivateDirectory::open(&path)?;
        let value: Value = read_json(&path.join("transition.json"))?;
        selected.verify()?;
        if value.get("environment").and_then(Value::as_str) != Some(environment) {
            continue;
        }
        let transition: Bg1Transition = serde_json::from_value(value)?;
        let index = transition.before_revision.checked_sub(1).ok_or("bg1_retained_revision")? as usize;
        require(index < 3 && directories[index].is_none(), "bg1_retained_rollback_ambiguous")?;
        directories[index] = Some((selected, transition));
    }
    rollback.verify()?;
    Ok(Bg1RollbackDirectories { root: rollback, steps: directories })
}

fn bg1_component<'a>(runner: &'a Runner, expected_sha256: &str) -> Result<&'a Artifact> {
    let components: Vec<_> = runner
        .files
        .iter()
        .filter(|file| file.path.file_name().is_some_and(|name| name == "native-command-session.json"))
        .collect();
    require(
        components.len() == 1 && components[0].sha256 == expected_sha256
            && components[0].path == runner.proton.with_file_name("native-command-session.json"),
        "bg1_selected_component_binding",
    )?;
    Ok(components[0])
}

/// Verify the exact historical rollback set before using the selected BG1 V4
/// runtime. No retained file is rewritten and no rollback action is offered.
pub fn verify_selected_bg1_history(m: &Manager, selected: &Environment) -> Result<()> {
    if selected.runner.id != BG1_V4_RUNNER {
        return Ok(());
    }
    verify_bg1_history_with(
        m, selected, BG1_ENVIRONMENT, BG1_CLASS, BG1_V4_RUNNER,
        BG1_V4_COMPONENT, &BG1_STEPS,
    )
}

fn verify_bg1_history_with(
    m: &Manager,
    selected: &Environment,
    environment: &str,
    class: &str,
    runner: &str,
    component: &str,
    steps: &[Bg1Step<'_>; 3],
) -> Result<()> {
    require(
        selected.id == environment
            && selected.revision == 4
            && selected.root == m.root.join("environments").join(environment)
            && selected.runner.id == runner
            && selected.runner.policy == Some(RunnerPolicy::DcompWineBuiltinsReferenceV1)
            && onboarding::runner_key(&selected.runner)? == steps[2].after_key,
        "bg1_selected_environment_binding",
    )?;
    bg1_component(&selected.runner, component)?.verify()?;
    let selected_record: onboarding::Record = read_json(
        &m.root.join("onboarding").join(environment).join("record.json"),
    )?;
    require(
        selected_record.schema == 1
            && selected_record.id == environment
            && selected_record.environment == *selected
            && !selected_record.published,
        "bg1_selected_onboarding_binding",
    )?;

    let mut directories = bg1_rollback_directories(&m.root, environment)?;
    let mut after = selected.clone();
    for index in (0..3).rev() {
        let (directory, transition) = directories.steps[index].take().ok_or("bg1_retained_rollback_absent")?;
        directory.verify()?;
        let path = &directory.path;
        let step = &steps[index];
        let before: Environment = read_json(&path.join("environment.json"))?;
        let record: onboarding::Record = read_json(&path.join("record.json"))?;
        let registry_value: Value = read_json(&path.join("registry.json"))?;
        let retired_value = registry_value
            .get("classes")
            .and_then(|classes| classes.get(class))
            .and_then(Value::as_object)
            .ok_or("bg1_retained_class_absent")?;
        require(
            retired_value.len() == 3
                && ["registration", "publication", "managed_revision"]
                    .iter()
                    .all(|key| retired_value.contains_key(*key)),
            "bg1_retained_publication_schema",
        )?;
        let registry: Registry = serde_json::from_value(registry_value)?;
        let result: Bg1TransitionResult = read_json(&path.join("result.json"))?;
        verify_bg1_step(&transition, &result, step, environment, class)?;
        require(
            before.id == environment
                && before.root == selected.root
                && before.revision == step.before_revision
                && before.runner.id == step.before_runner
                && onboarding::runner_key(&before.runner)? == step.before_key
                && after.revision == step.before_revision + 1
                && after.runner.id == step.after_runner
                && onboarding::runner_key(&after.runner)? == step.after_key
                && record.schema == 1
                && record.id == environment
                && record.environment == before
                && !record.published
                && registry.schema == 1,
            "bg1_retained_rollback_binding",
        )?;
        let retired = registry.classes.get(class).ok_or("bg1_retained_class_absent")?;
        require(
            retired.publication == Publication::Removed
                && retired.registration.environment == before
                && retired.managed_revision.as_ref() == Some(&transition.removed_publication),
            "bg1_retained_publication_binding",
        )?;
        directory.verify()?;
        after = before;
    }
    directories.root.verify()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn installed_v4_transition() -> (Bg1Transition, Bg1TransitionResult) {
        let step = &BG1_STEPS[2];
        (
            Bg1Transition {
                schema: 1,
                state: "prepared".into(),
                environment: BG1_ENVIRONMENT.into(),
                before_revision: 3,
                before_runner_sha256: step.before_key.into(),
                after_revision: 4,
                after_runner_sha256: step.after_key.into(),
                candidate_manifest_sha256: step.manifest.into(),
                candidate_tree_sha256: step.tree.into(),
                retired_class: BG1_CLASS.into(),
                predecessor_candidate: step.predecessor.into(),
                removed_publication: publication::RevisionRef {
                    id: step.removed_id.into(),
                    sha256: step.removed_sha256.into(),
                },
                prefix_recreated: false,
                installation_changed: false,
                historical_results_rewritten: false,
                publication_carried_forward: false,
            },
            Bg1TransitionResult {
                schema: 1,
                state: "completed".into(),
                environment: BG1_ENVIRONMENT.into(),
                revision: 4,
                runner_sha256: step.after_key.into(),
                candidate_manifest_sha256: step.manifest.into(),
            },
        )
    }

    #[test]
    fn retained_v4_transition_has_one_exact_interpretation() {
        let verify_bg1_step = |transition: &Bg1Transition, result: &Bg1TransitionResult, step: &Bg1Step<'_>| {
            super::verify_bg1_step(transition, result, step, BG1_ENVIRONMENT, BG1_CLASS)
        };
        let (transition, result) = installed_v4_transition();
        verify_bg1_step(&transition, &result, &BG1_STEPS[2]).unwrap();
        let mut changed = transition.clone();
        changed.environment = "00".repeat(16);
        assert!(verify_bg1_step(&changed, &result, &BG1_STEPS[2]).is_err());
        changed = transition.clone();
        changed.retired_class = "00".repeat(16);
        assert!(verify_bg1_step(&changed, &result, &BG1_STEPS[2]).is_err());
        changed = transition.clone();
        changed.predecessor_candidate = "00".repeat(32);
        assert!(verify_bg1_step(&changed, &result, &BG1_STEPS[2]).is_err());
        changed = transition.clone();
        changed.before_runner_sha256 = "00".repeat(32);
        assert!(verify_bg1_step(&changed, &result, &BG1_STEPS[2]).is_err());
        changed = transition.clone();
        changed.after_runner_sha256 = "00".repeat(32);
        assert!(verify_bg1_step(&changed, &result, &BG1_STEPS[2]).is_err());
        changed = transition.clone();
        changed.candidate_tree_sha256 = "00".repeat(32);
        assert!(verify_bg1_step(&changed, &result, &BG1_STEPS[2]).is_err());
        changed = transition.clone();
        changed.removed_publication.id = "00".repeat(16);
        assert!(verify_bg1_step(&changed, &result, &BG1_STEPS[2]).is_err());
        changed = transition.clone();
        changed.prefix_recreated = true;
        assert!(verify_bg1_step(&changed, &result, &BG1_STEPS[2]).is_err());
        let mut changed_result = result.clone();
        changed_result.state = "candidate_pending".into();
        assert!(verify_bg1_step(&transition, &changed_result, &BG1_STEPS[2]).is_err());
        assert!(verify_bg1_step(&transition, &result, &BG1_STEPS[1]).is_err());
    }

    #[test]
    fn retained_bg1_rollback_refuses_ambiguity_without_claiming_other_runners() {
        let fixture = test_fixture::Fixture::new();
        let root = fixture.m.root.join("private-rollback");
        private_dir(&root).unwrap();
        let (transition, _) = installed_v4_transition();
        let first = root.join(format!("experimental-runner-{}", "a".repeat(32)));
        private_dir(&first).unwrap();
        atomic_json(&first.join("transition.json"), &transition).unwrap();
        let unrelated = root.join(format!("experimental-runner-{}", "b".repeat(32)));
        private_dir(&unrelated).unwrap();
        let mut other_environment = transition.clone();
        other_environment.environment = "00".repeat(16);
        atomic_json(&unrelated.join("transition.json"), &other_environment).unwrap();
        let touch = root.join("experimental-runner-touch-4db060b14388e41103834fc4dfdd023a");
        private_dir(&touch).unwrap();
        assert!(bg1_rollback_directories(&fixture.m.root, BG1_ENVIRONMENT).unwrap().steps[2].is_some());
        let duplicate = root.join(format!("experimental-runner-{}", "c".repeat(32)));
        private_dir(&duplicate).unwrap();
        atomic_json(&duplicate.join("transition.json"), &transition).unwrap();
        assert!(bg1_rollback_directories(&fixture.m.root, BG1_ENVIRONMENT).is_err());
    }

    #[test]
    fn retained_history_directory_reads_never_create_or_change_authority() {
        let fixture = test_fixture::Fixture::new();
        let root = fixture.m.root.join("private-rollback");
        assert!(bg1_rollback_directories(&fixture.m.root, BG1_ENVIRONMENT).is_err());
        assert!(!root.exists());

        let substitute = fixture.m.root.join("other-private-directory");
        private_dir(&substitute).unwrap();
        std::os::unix::fs::symlink(&substitute, &root).unwrap();
        assert!(bg1_rollback_directories(&fixture.m.root, BG1_ENVIRONMENT).is_err());
        assert_eq!(fs::read_link(&root).unwrap(), substitute);
        fs::remove_file(&root).unwrap();

        private_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(bg1_rollback_directories(&fixture.m.root, BG1_ENVIRONMENT).is_err());
        assert_eq!(fs::symlink_metadata(&root).unwrap().mode() & 0o777, 0o755);
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let transition_dir = root.join(format!("experimental-runner-{}", "a".repeat(32)));
        let (transition, _) = installed_v4_transition();
        private_dir(&transition_dir).unwrap();
        atomic_json(&transition_dir.join("transition.json"), &transition).unwrap();
        let original = fs::read(transition_dir.join("transition.json")).unwrap();
        let held = ExistingPrivateDirectory::open(&transition_dir).unwrap();

        let missing = root.join(format!("experimental-runner-{}", "b".repeat(32)));
        assert!(ExistingPrivateDirectory::open(&missing).is_err());
        assert!(!missing.exists());

        let link = root.join(format!("experimental-runner-{}", "c".repeat(32)));
        std::os::unix::fs::symlink(&transition_dir, &link).unwrap();
        assert!(ExistingPrivateDirectory::open(&link).is_err());
        assert!(bg1_rollback_directories(&fixture.m.root, BG1_ENVIRONMENT).is_err());
        assert_eq!(fs::read_link(&link).unwrap(), transition_dir);
        fs::remove_file(&link).unwrap();

        let public = root.join(format!("experimental-runner-{}", "d".repeat(32)));
        private_dir(&public).unwrap();
        fs::set_permissions(&public, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(ExistingPrivateDirectory::open(&public).is_err());
        assert!(bg1_rollback_directories(&fixture.m.root, BG1_ENVIRONMENT).is_err());
        assert_eq!(fs::symlink_metadata(&public).unwrap().mode() & 0o777, 0o755);
        assert_eq!(fs::read(transition_dir.join("transition.json")).unwrap(), original);

        // A held directory descriptor makes a path replacement observable even
        // if the replacement is otherwise private and has the same contents.
        let moved = root.join("original-held-directory");
        fs::rename(&transition_dir, &moved).unwrap();
        private_dir(&transition_dir).unwrap();
        fs::write(transition_dir.join("transition.json"), &original).unwrap();
        assert!(held.verify().is_err());
        assert_eq!(fs::read(moved.join("transition.json")).unwrap(), original);
        assert_eq!(fs::read(transition_dir.join("transition.json")).unwrap(), original);
    }

    #[test]
    fn selected_v4_component_is_one_exact_pinned_runner_file() {
        let fixture = test_fixture::Fixture::new();
        let mut runner = fixture.r.environment.runner.clone();
        let component = Artifact {
            path: runner.proton.with_file_name("native-command-session.json"),
            sha256: BG1_V4_COMPONENT.into(),
        };
        runner.files.push(component.clone());
        assert_eq!(bg1_component(&runner, BG1_V4_COMPONENT).unwrap(), &component);
        runner.files.push(component.clone());
        assert!(bg1_component(&runner, BG1_V4_COMPONENT).is_err());
        runner.files.pop();
        runner.files.last_mut().unwrap().sha256 = "00".repeat(32);
        assert!(bg1_component(&runner, BG1_V4_COMPONENT).is_err());
        runner.files.last_mut().unwrap().sha256 = BG1_V4_COMPONENT.into();
        runner.files.last_mut().unwrap().path = runner.proton.with_file_name("wrong-command-session.json");
        assert!(bg1_component(&runner, BG1_V4_COMPONENT).is_err());
    }

    #[test]
    fn unrelated_runner_never_requires_bg1_history() {
        let fixture = test_fixture::Fixture::new();
        verify_selected_bg1_history(&fixture.m, &fixture.r.environment).unwrap();
    }

    #[test]
    fn complete_retained_chain_binds_each_backup_and_current_environment() {
        let fixture = test_fixture::Fixture::new();
        let environment = "11".repeat(16);
        let environment_root = fixture.m.root.join("environments").join(&environment);
        private_dir(&environment_root.join("compatdata/pfx/drive_c")).unwrap();
        let class = fixture.r.metadata.class_id.clone();
        let mut versions = Vec::new();
        for (index, name) in ["standard", "bg1-v1", "bg1-v3", BG1_V4_RUNNER].iter().enumerate() {
            let mut value = fixture.r.environment.clone();
            value.id = environment.clone();
            value.root = environment_root.clone();
            value.revision = (index + 1) as u64;
            value.runner.id = (*name).into();
            if index == 3 {
                value.runner.policy = Some(RunnerPolicy::DcompWineBuiltinsReferenceV1);
                let path = value.runner.proton.with_file_name("native-command-session.json");
                fs::write(&path, b"source owned command component").unwrap();
                value.runner.files.push(Artifact { path: path.clone(), sha256: digest(&path).unwrap() });
            }
            versions.push(value);
        }
        atomic_json(&environment_root.join("environment.json"), &versions[3]).unwrap();
        let keys: Vec<_> = versions.iter().map(|value| onboarding::runner_key(&value.runner).unwrap()).collect();
        let steps: [_; 3] = std::array::from_fn(|index| Bg1Step {
            before_revision: (index + 1) as u64,
            before_runner: &versions[index].runner.id,
            before_key: &keys[index],
            after_runner: &versions[index + 1].runner.id,
            after_key: &keys[index + 1],
            manifest: BG1_STEPS[index].manifest,
            tree: BG1_STEPS[index].tree,
            predecessor: BG1_STEPS[index].predecessor,
            removed_id: BG1_STEPS[index].removed_id,
            removed_sha256: BG1_STEPS[index].removed_sha256,
        });
        let record_for = |env: &Environment| onboarding::Record {
            schema: 1,
            id: environment.clone(),
            installer: "aa".repeat(32),
            environment: env.clone(),
            created_at: 1,
            creation_operation: "bb".repeat(16),
            installation_operation: None,
            published: false,
                automatic: None,
            previous_attempt: None,
        };
        let onboarding_dir = fixture.m.root.join("onboarding").join(&environment);
        private_dir(&onboarding_dir).unwrap();
        atomic_json(&onboarding_dir.join("record.json"), &record_for(&versions[3])).unwrap();
        let rollback = fixture.m.root.join("private-rollback");
        private_dir(&rollback).unwrap();
        let mut transitions = Vec::new();
        for index in 0..3 {
            let step = &steps[index];
            let path = rollback.join(format!("experimental-runner-{:032x}", index + 1));
            private_dir(&path).unwrap();
            let transition = Bg1Transition {
                schema: 1,
                state: "prepared".into(),
                environment: environment.clone(),
                before_revision: step.before_revision,
                before_runner_sha256: step.before_key.into(),
                after_revision: step.before_revision + 1,
                after_runner_sha256: step.after_key.into(),
                candidate_manifest_sha256: step.manifest.into(),
                candidate_tree_sha256: step.tree.into(),
                retired_class: class.clone(),
                predecessor_candidate: step.predecessor.into(),
                removed_publication: publication::RevisionRef {
                    id: step.removed_id.into(),
                    sha256: step.removed_sha256.into(),
                },
                prefix_recreated: false,
                installation_changed: false,
                historical_results_rewritten: false,
                publication_carried_forward: false,
            };
            let result = Bg1TransitionResult {
                schema: 1,
                state: "completed".into(),
                environment: environment.clone(),
                revision: step.before_revision + 1,
                runner_sha256: step.after_key.into(),
                candidate_manifest_sha256: step.manifest.into(),
            };
            let mut registration = fixture.r.clone();
            registration.environment = versions[index].clone();
            let mut classes = std::collections::BTreeMap::new();
            classes.insert(class.clone(), Entry {
                registration,
                publication: Publication::Removed,
                managed_revision: Some(transition.removed_publication.clone()),
            });
            let registry = Registry { schema: 1, revision: 10 + index as u64, classes };
            atomic_json(&path.join("environment.json"), &versions[index]).unwrap();
            atomic_json(&path.join("record.json"), &record_for(&versions[index])).unwrap();
            atomic_json(&path.join("registry.json"), &registry).unwrap();
            atomic_json(&path.join("transition.json"), &transition).unwrap();
            atomic_json(&path.join("result.json"), &result).unwrap();
            transitions.push((path, transition));
        }
        let selected = &versions[3];
        let component = selected.runner.files.last().unwrap();
        verify_bg1_history_with(&fixture.m, selected, &environment, &class, BG1_V4_RUNNER, &component.sha256, &steps).unwrap();
        // Exercise the shared keeper owner with an exact source-owned V4
        // lineage. The production entry point supplies the pinned verifier;
        // this fixture supplies the same verifier with fixture-owned keys.
        let software = catalogue::Software {
            installer_launch: None, preparation_kit: None, operator_frontend: None,
            manager: fixture.r.host.clone(), supervisor: fixture.r.host.clone(),
            ownership: fixture.r.host.clone(), host: fixture.r.host.clone(),
            source_manifest: fixture.r.host.clone(), source_sha256: fixture.r.host_source_sha256.clone(),
            native_catalogue: None,
        };
        atomic_json(&fixture.m.root.join("software.json"), &software).unwrap();
        let mut binding: crate::HostBinding = fixture.r.clone().into();
        binding.environment = selected.clone();
        let keepers = crate::Keepers::new(Vec::new());
        let checks = std::sync::atomic::AtomicUsize::new(0);
        let starts = std::sync::atomic::AtomicUsize::new(0);
        let gate = |m: &Manager, env: &Environment| {
            checks.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            verify_bg1_history_with(m, env, &environment, &class, BG1_V4_RUNNER,
                &component.sha256, &steps)
        };
        let start = |_: &catalogue::Software, path: &Path| -> Result<std::process::Child> {
            starts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let job: crate::SessionSpec = read_json(path)?;
            atomic_json(&job.lease, &job.report)?;
            Ok(std::process::Command::new("/bin/sh").args(["-c", "sleep 30"]).spawn()?)
        };
        assert_eq!(crate::stage_keeper_with_history(&fixture.m, &software, &binding, &keepers,
            None, gate, start).unwrap(), crate::KeeperAvailability::Starting);
        let session_root = selected.root.join("compatdata/pfx/drive_c/bridge/sessions");
        let session_count = fs::read_dir(&session_root).unwrap().count();
        let lease_count = fs::read_dir(fixture.m.root.join("runtime/leases")).unwrap().count();
        {
            let active = keepers.lock().unwrap();
            assert_eq!(active.len(), 1);
            atomic_json(&active[0].report, &json!({"ready": true,
                "environment": environment})).unwrap();
        }
        assert_eq!(crate::stage_keeper_with_history(&fixture.m, &software, &binding, &keepers,
            None, gate, start).unwrap(), crate::KeeperAvailability::Ready);
        assert_eq!(checks.load(std::sync::atomic::Ordering::SeqCst), 2);

        let assert_no_new_owner = || {
            assert!(crate::stage_keeper_with_history(&fixture.m, &software, &binding,
                &keepers, None, gate, start).is_err());
            let mut active = keepers.lock().unwrap();
            assert_eq!(active.len(), 1);
            assert!(!active[0].retiring);
            assert!(active[0].child.try_wait().unwrap().is_none());
            drop(active);
            assert_eq!(starts.load(std::sync::atomic::Ordering::SeqCst), 1);
            assert_eq!(fs::read_dir(&session_root).unwrap().count(), session_count);
            assert_eq!(fs::read_dir(fixture.m.root.join("runtime/leases")).unwrap().count(), lease_count);
        };
        let original_transition = fs::read(transitions[2].0.join("transition.json")).unwrap();
        let mut changed = transitions[2].1.clone();
        changed.candidate_tree_sha256 = "00".repeat(32);
        atomic_json(&transitions[2].0.join("transition.json"), &changed).unwrap();
        assert_no_new_owner();
        fs::write(transitions[2].0.join("transition.json"), &original_transition).unwrap();
        let duplicate = rollback.join(format!("experimental-runner-{}", "d".repeat(32)));
        private_dir(&duplicate).unwrap();
        fs::write(duplicate.join("transition.json"), &original_transition).unwrap();
        assert_no_new_owner();
        fs::remove_dir_all(&duplicate).unwrap();
        let missing_step = rollback.join("temporarily-missing-step");
        fs::rename(&transitions[1].0, &missing_step).unwrap();
        assert_no_new_owner();
        fs::rename(&missing_step, &transitions[1].0).unwrap();
        {
            let mut active = keepers.lock().unwrap();
            active[0].child.kill().unwrap();
            active[0].child.wait().unwrap();
        }
        let missing = transitions[1].0.join("transition.json");
        let retained = fs::read(&missing).unwrap();
        fs::remove_file(&missing).unwrap();
        assert!(verify_bg1_history_with(&fixture.m, selected, &environment, &class, BG1_V4_RUNNER, &component.sha256, &steps).is_err());
        assert!(!missing.exists());
        fs::write(&missing, &retained).unwrap();
        let absent_step = rollback.join("absent-transition-step");
        fs::rename(&transitions[1].0, &absent_step).unwrap();
        assert!(verify_bg1_history_with(&fixture.m, selected, &environment, &class, BG1_V4_RUNNER, &component.sha256, &steps).is_err());
        assert!(!transitions[1].0.exists());
        fs::rename(&absent_step, &transitions[1].0).unwrap();
        let mut changed = transitions[2].1.clone();
        changed.candidate_tree_sha256 = "00".repeat(32);
        atomic_json(&transitions[2].0.join("transition.json"), &changed).unwrap();
        assert!(verify_bg1_history_with(&fixture.m, selected, &environment, &class, BG1_V4_RUNNER, &component.sha256, &steps).is_err());
        atomic_json(&transitions[2].0.join("transition.json"), &transitions[2].1).unwrap();
        fs::write(&component.path, b"changed component").unwrap();
        assert!(verify_bg1_history_with(&fixture.m, selected, &environment, &class, BG1_V4_RUNNER, &component.sha256, &steps).is_err());
    }
}
