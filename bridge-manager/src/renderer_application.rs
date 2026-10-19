//! NAUI2 exact application identity and closed operation binding. No vendor launch here.
use crate::{catalogue::Software, *};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const ID: &str = "native-access";
pub const ENVIRONMENT: &str = "627d2cba97edbecf113c22504eb4c81b";
pub const INSTALLATION: &str = "e4143128adc87de3fdbbdfb44f186ee5";
pub const OBSERVATION: &[u8] = include_bytes!("../../evidence/naui1/observation.json");
pub const SEAL: &str = "539bcf48c961e18b93ce6bc54c4aed20ab7a82822e7563e34b8025b6836b4be9";

pub use crate::operator_model::{RendererPolicy, Presentation};
impl RendererPolicy {
    pub fn arguments(self) -> &'static [&'static str] {
        match self { Self::Inherited => &[], Self::SoftwareRendering => &["--disable-gpu"] }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Image { pub artifact: Artifact, pub size: u64 }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Application {
    pub schema: u32,
    pub id: String,
    pub environment: Environment,
    pub files: BTreeMap<String, Image>,
    pub installation: Artifact,
    pub observation_sha256: String,
    pub source_seal_sha256: String,
}
fn observed() -> Result<Value> { Ok(serde_json::from_slice::<Value>(OBSERVATION)?["result"].clone()) }
/// Observe a bounded resource through one held file. Reused digests remain tied
/// to the complete file identity and the current verification scope.
pub(crate) fn observe_image(path: &Path) -> Result<Image> {
    observe_image_with(path, |_| {})
}
fn observe_image_with(path: &Path, mut hashed: impl FnMut(usize)) -> Result<Image> {
    require(path.canonicalize()? == path, "renderer_image_alias")?;
    let mut f = file(path)?;
    let metadata = f.metadata()?;
    let before = DigestFileIdentity::from(&metadata);
    require(before.length > 0 && before.length <= 256*1024*1024, "renderer_image_bound")?;
    require(metadata.nlink() == 1, "renderer_image_extent_or_alias")?;
    let cached = SCOPED_DIGESTS.with(|cache| cache.borrow().as_ref()
        .and_then(|records| records.get(path))
        .filter(|(identity, _)| *identity == before)
        .map(|(_, value)| value.clone()));
    let value = if let Some(value) = cached { value } else {
        let mut h = Sha256::new();
        let mut buf = [0u8;65536];
        let mut left = before.length;
        while left > 0 {
            let n = f.read(&mut buf[..(left as usize).min(65536)])?;
            require(n > 0, "renderer_image_short")?;
            h.update(&buf[..n]); hashed(n); left -= n as u64;
        }
        hex(&h.finalize())
    };
    require(DigestFileIdentity::from(&f.metadata()?) == before
        && DigestFileIdentity::from(&fs::symlink_metadata(path)?) == before,
        "renderer_image_changed")?;
    SCOPED_DIGESTS.with(|cache| {
        if let Some(records) = cache.borrow_mut().as_mut() {
            records.insert(path.to_owned(), (before, value.clone()));
        }
    });
    Ok(Image {artifact: Artifact {path: path.to_owned(), sha256: value}, size: before.length})
}
/// Bounded exact-file verification outside registry authority; rejects aliases.
pub fn verify_image(image: &Image) -> Result<()> {
    require(image.size > 0 && image.size <= 256*1024*1024, "renderer_image_bound")?;
    require(observe_image(&image.artifact.path)? == *image, "renderer_image_changed")
}
fn roots(prefix: &Path) -> Result<Vec<PathBuf>> {
    let mut pending=vec![prefix.to_owned()];let mut found=vec![];let mut count=0;
    while let Some(dir)=pending.pop() {
        for entry in fs::read_dir(dir)? {
            count+=1;require(count<=100_000,"renderer_census_bound")?;
            let e=entry?;let ty=e.file_type()?;
            if ty.is_dir(){pending.push(e.path());}
            if e.file_name().to_string_lossy().eq_ignore_ascii_case("Native Access.exe") {
                require(ty.is_file(),"renderer_executable_alias")?;
                found.push(e.path().parent().ok_or("renderer_root")?.to_owned());
            }
        }
    }
    Ok(found)
}
impl Application {
    /// An installation survives a runner revision. All vendor bytes, paths,
    /// prefix identity and installation evidence must still match exactly.
    pub fn same_installation(&self, installed: &Self) -> bool {
        if self == installed { return true; }
        if self.environment.revision <= installed.environment.revision { return false; }
        let mut historical = self.clone();
        historical.environment.runner = installed.environment.runner.clone();
        historical.environment.revision = installed.environment.revision;
        historical == *installed
    }
    pub fn identity(&self) -> Result<String> {Ok(hex(&Sha256::digest(serde_json::to_vec(&serde_json::to_value(self)?)?)))}
    pub fn executable(&self) -> Result<&Image> {self.files.get("Native Access.exe").ok_or_else(||"renderer_executable_missing".into())}
    /// Historical identity readback. Current bytes and process/runner health are
    /// separate launch checks and cannot invalidate a retained revision edge.
    pub fn validate_record(&self, root: &Path) -> Result<()> {
        let observation=observed()?;
        require(self.schema==1 && self.id==ID && self.environment.id==ENVIRONMENT
            && self.environment.root==root.join("environments").join(ENVIRONMENT),
            "renderer_application_environment")?;
        require(self.observation_sha256==hex(&Sha256::digest(OBSERVATION))
            && self.source_seal_sha256==SEAL,"renderer_observation_identity")?;
        let expected=observation["identity"]["files"].as_object().ok_or("renderer_identity_evidence")?;
        require(self.files.len()==expected.len(),"renderer_resource_set")?;
        let directory=self.executable()?.artifact.path.parent().ok_or("renderer_directory")?;
        require(directory.starts_with(self.environment.root.join("compatdata/pfx/drive_c"))
            && directory.components().all(|part|!matches!(part,
                std::path::Component::ParentDir | std::path::Component::CurDir)),"renderer_directory")?;
        let mut total=0;
        for (name,witness) in expected {
            let image=self.files.get(name).ok_or("renderer_resource_missing")?;
            require(image.artifact.path==directory.join(name) && witness["sha256"]==image.artifact.sha256
                && witness["size"]==image.size,"renderer_resource_identity")?;
            total+=image.size;require(total<=512*1024*1024,"renderer_hash_budget")?;
        }
        require(self.installation.path==root.join("onboarding").join(ENVIRONMENT).join(format!("{INSTALLATION}-result.json"))
            && observation["input"]["result_sha256"]==self.installation.sha256,"renderer_installation_identity")
    }
    pub fn verify(&self, root: &Path) -> Result<()> {
        self.verify_with_runtime(root, true)
    }
    /// Application files and installation stay exact; the runtime tree census is
    /// skipped. For messages to a launch the manager already verified, such as the
    /// Native Access sign-in return.
    pub fn verify_for_running_launch(&self, root: &Path) -> Result<()> {
        self.verify_with_runtime(root, false)
    }
    fn verify_with_runtime(&self, root: &Path, runtime_tree: bool) -> Result<()> {
        self.validate_record(root)?;
        require(read_json::<Environment>(&self.environment.root.join("environment.json"))?==self.environment,
            "renderer_application_environment")?;
        let directory=self.executable()?.artifact.path.parent().ok_or("renderer_directory")?;
        require(roots(&self.environment.root.join("compatdata/pfx/drive_c"))? == vec![directory.to_owned()], "renderer_application_ambiguous_or_missing")?;
        for image in self.files.values() {verify_image(image)?;}
        self.installation.verify()?;
        if runtime_tree { self.environment.runner.verify() } else { self.environment.runner.verify_pinned_files() }
    }
}
pub fn discover(root: &Path) -> Result<Application> {
    let environment:Environment=read_json(&root.join("environments").join(ENVIRONMENT).join("environment.json"))?;
    require(environment.root==root.join("environments").join(ENVIRONMENT),"renderer_environment_location")?;
    let found=roots(&environment.root.join("compatdata/pfx/drive_c"))?;
    require(found.len()==1,"renderer_application_ambiguous_or_missing")?;
    let observation=observed()?;let mut files=BTreeMap::new();
    for (name,v) in observation["identity"]["files"].as_object().ok_or("renderer_identity_evidence")? {
        files.insert(name.clone(),Image{artifact:Artifact{path:found[0].join(name),sha256:v["sha256"].as_str().ok_or("renderer_digest")?.into()},size:v["size"].as_u64().ok_or("renderer_size")?});
    }
    let app=Application{schema:1,id:ID.into(),environment,files,
        installation:Artifact{path:root.join("onboarding").join(ENVIRONMENT).join(format!("{INSTALLATION}-result.json")),sha256:observation["input"]["result_sha256"].as_str().ok_or("renderer_result")?.into()},
        observation_sha256:hex(&Sha256::digest(OBSERVATION)),source_seal_sha256:SEAL.into()};
    app.verify(root)?;Ok(app)
}
/// Closed production binding, also consumed by the source-owned qualification owner.
/// Input Application has already passed discovery/verification under the manager.
pub fn bind(app: &Application, sw: &Software, operation: &str, policy: RendererPolicy, report: &Path) -> Result<Value> {
    require(valid_hex(operation,32),"renderer_operation")?;
    let adapter=sw.installer_launch.as_ref().ok_or("renderer_adapter_missing")?;
    for a in [&sw.manager,&sw.supervisor,&sw.ownership,adapter] {a.verify()?;}
    let software=serde_json::to_value(sw)?;
    let software_sha256=hex(&Sha256::digest(serde_json::to_vec(&software)?));
    Ok(json!({"schema":1,"kind":"renderer_application","application":app,"application_identity":app.identity()?,
        "operation":operation,"renderer_policy":policy,"report":report,
        "software":software,"software_sha256":software_sha256,"installer_launch":adapter}))
}

#[cfg(test)] mod tests {
    use super::*;
    #[test]
    fn image_scope_reuses_bytes_and_rejects_replacements_and_aliases() {
        let directory = std::env::temp_dir().canonicalize().unwrap().join(format!("lvb-image-{}", random_id().unwrap()));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("resource");
        fs::write(&path, b"first").unwrap();
        let mut bytes = 0;
        with_launch_verification(|| {
            let image = observe_image_with(&path, |n| bytes += n).unwrap();
            verify_image(&image).unwrap();
            assert_eq!(observe_image_with(&path, |n| bytes += n).unwrap(), image);
            assert_eq!(bytes, 5);
            let mut false_claim = image.clone();
            false_claim.artifact.sha256 = "0".repeat(64);
            assert!(verify_image(&false_claim).is_err());
            false_claim = image.clone(); false_claim.size += 1;
            assert!(verify_image(&false_claim).is_err());
            let modified = fs::metadata(&path).unwrap().modified().unwrap();
            let replacement = directory.join("replacement");
            fs::write(&replacement, b"other").unwrap();
            File::options().write(true).open(&replacement).unwrap()
                .set_times(fs::FileTimes::new().set_modified(modified)).unwrap();
            fs::rename(&replacement, &path).unwrap();
            assert!(verify_image(&image).is_err());
            let alias = directory.join("alias");
            fs::hard_link(&path, &alias).unwrap();
            assert!(observe_image(&path).is_err());
            fs::remove_file(&alias).unwrap();
            std::os::unix::fs::symlink(&path, &alias).unwrap();
            assert!(observe_image(&alias).is_err());
        });
        bytes = 0;
        with_launch_verification(|| { observe_image_with(&path, |n| bytes += n).unwrap(); });
        assert_eq!(bytes, 5, "next launch must read bytes again");
        fs::write(&path, vec![0u8; 131072]).unwrap();
        let short = observe_image_with(&path, |_| {
            File::options().write(true).open(&path).unwrap().set_len(0).unwrap();
        }).unwrap_err();
        assert_eq!(short.to_string(), "renderer_image_short");
        fs::write(&path, []).unwrap();
        assert!(observe_image(&path).is_err());
        File::options().write(true).open(&path).unwrap().set_len(256*1024*1024+1).unwrap();
        assert!(observe_image(&path).is_err());
        fs::remove_dir_all(directory).unwrap();
    }
    #[test] fn closed_modes_and_observations() {
        assert_eq!(RendererPolicy::Inherited.arguments(), &[] as &[&str]);
        assert_eq!(RendererPolicy::SoftwareRendering.arguments(), &["--disable-gpu"]);
        for s in ["--disable-gpu","no_sandbox","--no-sandbox","other"] {assert!(serde_json::from_value::<RendererPolicy>(json!(s)).is_err());}
        for s in ["blank_white","rendered_nonblank","unavailable"] {assert!(serde_json::from_value::<Presentation>(json!(s)).is_ok());}
        assert!(serde_json::from_value::<Presentation>(json!("screenshot")).is_err());
    }
}
