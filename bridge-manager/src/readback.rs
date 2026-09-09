//! Canonical physical readback. Presentation consumes these typed results.
use crate::{profiles::*, publication::RevisionRef, *};
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RefusalCode {
    ProfileInvalid,
    NoMatch,
    AmbiguousMatch,
    ModuleChanged,
    StaleCensus,
    ClassMismatch,
    MetadataMismatch,
    RunnerMismatch,
    EnvironmentMismatch,
    HostMismatch,
    NativeMismatch,
    UnsupportedCapability,
    ActiveDevice,
    ForeignPublication,
    RecoveryPending,
    SelectionRequired,
    LocalRecordOrIoFailure,
}
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Refusal {
    pub code: RefusalCode,
    pub detail: String,
}
pub fn refusal(e: &(dyn std::error::Error + Send + Sync)) -> Refusal {
    let detail = e.to_string();
    let code = match detail.as_str() {
        "profile_no_match" => RefusalCode::NoMatch,
        "profile_ambiguous" => RefusalCode::AmbiguousMatch,
        "module_digest_changed" => RefusalCode::ModuleChanged,
        "stale_census" | "census_report_changed" => RefusalCode::StaleCensus,
        "class_absent" | "class_mismatch" => RefusalCode::ClassMismatch,
        "role_mismatch" | "metadata_mismatch" => RefusalCode::MetadataMismatch,
        "runner_mismatch" => RefusalCode::RunnerMismatch,
        "environment_mismatch" | "environment_family_conflict" => RefusalCode::EnvironmentMismatch,
        "installed_host_mismatch" => RefusalCode::HostMismatch,
        "native_artifact_mismatch" | "native_artifact_absent" => RefusalCode::NativeMismatch,
        "precision_mismatch" | "metadata_tier_unsupported" => RefusalCode::UnsupportedCapability,
        "active_device_lease" | "active_lease_unresolved" => RefusalCode::ActiveDevice,
        "foreign_publication"
        | "foreign_or_missing_publication"
        | "foreign_transaction_pointer"
        | "foreign_publication_race" => RefusalCode::ForeignPublication,
        "publication_recovery_pending" => RefusalCode::RecoveryPending,
        "environment_selection_required" | "product_selection_required" => {
            RefusalCode::SelectionRequired
        }
        s if s.starts_with("profile_") || s == "duplicate_profile_revision" => {
            RefusalCode::ProfileInvalid
        }
        _ => RefusalCode::LocalRecordOrIoFailure,
    };
    Refusal { code, detail }
}

#[derive(Debug, Serialize)]
pub struct ProfileSelection {
    pub id: String,
    pub revision: u32,
    pub claim: Claim,
}
#[derive(Debug, Serialize)]
pub struct RollbackTarget {
    pub revision: RevisionRef,
    pub target: Option<PathBuf>,
    pub valid: bool,
}
#[derive(Debug, Serialize)]
pub struct ProductStatus {
    pub selection: usize,
    pub name: String,
    pub role: Role,
    pub class_id: String,
    pub external_ids: [String; 2],
    pub profile: Option<ProfileSelection>,
    pub module_sha256: String,
    pub build: String,
    pub module_valid: bool,
    pub environment: String,
    pub environment_revision: u64,
    pub runner: String,
    pub environment_valid: bool,
    pub runner_valid: bool,
    pub installed_host_valid: bool,
    pub native_artifact_valid: bool,
    pub publication: Publication,
    pub recorded_revision: Option<RevisionRef>,
    pub active_revision: Option<RevisionRef>,
    pub expected_target: Option<PathBuf>,
    pub physical_target: Option<PathBuf>,
    pub publication_valid: bool,
    pub prior_rollback: Option<RollbackTarget>,
    pub performance: Option<Performance>,
    pub recommended_frames: u32,
    pub performance_policy: PerformancePolicy,
    pub compatibility: Compatibility,
    pub capabilities: Option<Capabilities>,
    pub limitations: Option<Vec<Limitation>>,
    pub recovery_pending: bool,
    pub refusal: Option<Refusal>,
}
#[derive(Debug, Serialize)]
pub struct ManagedStatus {
    pub schema: u32,
    pub products: Vec<ProductStatus>,
}
impl Manager {
    pub fn managed_status(&self, installed_host: &Artifact, source: &str) -> Result<ManagedStatus> {
        let _lock = self.lock("registry.lock")?;
        let db = self.registry()?;
        let mut products = Vec::new();
        for (index, (key, e)) in db.classes.iter().enumerate() {
            let r = &e.registration;
            let physical = crate::publication::physical(&self.link(key));
            let target = self.entry_target(e);
            let loaded = e
                .managed_revision
                .as_ref()
                .map(|r| self.load_revision(key, r))
                .transpose();
            let native = if e.managed_revision.is_some() {
                r.native.clone()
            } else {
                Artifact {
                    path: self.native_path(r),
                    sha256: r.native.sha256.clone(),
                }
            };
            let native_valid = native.verify().is_ok() && loaded.is_ok();
            let module_valid = r.module.verify().is_ok();
            let environment_valid =
                read_json::<Environment>(&r.environment.root.join("environment.json"))
                    .is_ok_and(|current| current == r.environment);
            let runner_valid = r.environment.runner.verify().is_ok();
            let host_valid = r.host.verify().is_ok()
                && r.host.sha256 == installed_host.sha256
                && r.host_source_sha256 == source
                && installed_host.verify().is_ok();
            let pending = self.publication_pending(key)?;
            let physical_valid = match e.publication {
                Publication::Published => {
                    matches!((&target,&physical),(Ok(expected),Ok(Some(actual))) if expected==actual)
                }
                Publication::Removed => matches!(&physical, Ok(None)),
                Publication::Pending => false,
            };
            let performance = self.performance(key);
            let inactive = self.require_inactive(Some(key));
            let error: Option<Box<dyn std::error::Error + Send + Sync>> = if pending {
                Some("publication_recovery_pending".into())
            } else if !module_valid {
                Some("module_digest_changed".into())
            } else if !environment_valid {
                Some("environment_mismatch".into())
            } else if !runner_valid {
                Some("runner_mismatch".into())
            } else if !host_valid {
                Some("installed_host_mismatch".into())
            } else if !native_valid {
                Some("native_artifact_mismatch".into())
            } else if !physical_valid {
                Some("foreign_or_missing_publication".into())
            } else if let Err(e) = &performance {
                Some(e.to_string().into())
            } else {
                inactive.err()
            };
            let pending_physical = if pending {
                physical
                    .as_ref()
                    .ok()
                    .and_then(|p| p.as_ref())
                    .and_then(|target| self.pending_physical_revision(key, target).ok().flatten())
            } else {
                None
            };
            let active_revision = if let Some((reference, _)) = &pending_physical {
                Some(reference.clone())
            } else if physical_valid && native_valid && e.publication == Publication::Published {
                e.managed_revision.clone()
            } else {
                None
            };
            let revision = pending_physical
                .map(|(_, r)| r)
                .or_else(|| loaded.ok().flatten());
            let prior = revision
                .as_ref()
                .and_then(|r| r.parent.as_ref())
                .map(|reference| {
                    let prior = self.load_revision(key, reference);
                    RollbackTarget {
                        revision: reference.clone(),
                        valid: prior.is_ok(),
                        target: prior.ok().map(|r| r.target),
                    }
                });
            let publication_valid = physical_valid && native_valid;
            products.push(ProductStatus {
                selection: index + 1,
                name: r.metadata.name.clone(),
                role: role(&r.metadata)?,
                class_id: key.clone(),
                external_ids: external_ids(key)?,
                profile: revision.as_ref().map(|r| ProfileSelection {
                    id: r.profile.id.clone(),
                    revision: r.profile.revision,
                    claim: r.profile.claim.clone(),
                }),
                module_sha256: r.module.sha256.clone(),
                build: r.metadata.version.clone(),
                module_valid,
                environment: r.environment.id.clone(),
                environment_revision: r.environment.revision,
                runner: r.environment.runner.id.clone(),
                environment_valid,
                runner_valid,
                installed_host_valid: host_valid,
                native_artifact_valid: native_valid,
                publication: e.publication.clone(),
                recorded_revision: e.managed_revision.clone(),
                active_revision,
                expected_target: target.ok(),
                physical_target: physical.ok().flatten(),
                publication_valid,
                prior_rollback: prior,
                performance: performance.ok(),
                recommended_frames: 512,
                performance_policy: PerformancePolicy::Frames512Recommended256Unqualified,
                compatibility: r.compatibility.clone(),
                capabilities: revision.as_ref().map(|r| r.profile.capabilities.clone()),
                limitations: revision.as_ref().map(|r| r.profile.limitations.clone()),
                recovery_pending: pending,
                refusal: error.as_ref().map(|e| refusal(e.as_ref())),
            });
        }
        Ok(ManagedStatus {
            schema: 1,
            products,
        })
    }
}
