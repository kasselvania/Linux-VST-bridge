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
fn stable(md: &fs::Metadata) -> (u64,u64,u64,i64,i64,i64,i64) {
    (md.dev(),md.ino(),md.len(),md.mtime(),md.mtime_nsec(),md.ctime(),md.ctime_nsec())
}
/// Bounded exact-file hashing outside registry authority; rejects all path aliases.
pub fn verify_image(image: &Image) -> Result<()> {
    require(image.size > 0 && image.size <= 256*1024*1024, "renderer_image_bound")?;
    require(image.artifact.path.canonicalize()? == image.artifact.path, "renderer_image_alias")?;
    let mut f=file(&image.artifact.path)?;
    let before=f.metadata()?;
    require(before.nlink()==1 && before.len()==image.size, "renderer_image_extent_or_alias")?;
    let mut h=Sha256::new(); let mut buf=[0u8;65536]; let mut left=image.size;
    while left>0 {let n=f.read(&mut buf[..(left as usize).min(65536)])?;require(n>0,"renderer_image_short")?;h.update(&buf[..n]);left-=n as u64;}
    require(hex(&h.finalize())==image.artifact.sha256 && stable(&before)==stable(&f.metadata()?)
        && stable(&before)==stable(&fs::symlink_metadata(&image.artifact.path)?),"renderer_image_changed")
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
    pub fn verify(&self, root: &Path) -> Result<()> {
        let observation=observed()?;
        require(self.schema==1 && self.id==ID && self.environment.id==ENVIRONMENT
            && self.environment.root==root.join("environments").join(ENVIRONMENT)
            && read_json::<Environment>(&self.environment.root.join("environment.json"))?==self.environment,
            "renderer_application_environment")?;
        require(self.observation_sha256==hex(&Sha256::digest(OBSERVATION)) && self.source_seal_sha256==SEAL,"renderer_observation_identity")?;
        let expected=observation["identity"]["files"].as_object().ok_or("renderer_identity_evidence")?;
        require(self.files.len()==expected.len(),"renderer_resource_set")?;
        let directory=self.executable()?.artifact.path.parent().ok_or("renderer_directory")?;
        require(directory.starts_with(self.environment.root.join("compatdata/pfx/drive_c")),"renderer_directory")?;
        require(roots(&self.environment.root.join("compatdata/pfx/drive_c"))? == vec![directory.to_owned()], "renderer_application_ambiguous_or_missing")?;
        let mut total=0;
        for (name,witness) in expected {
            let image=self.files.get(name).ok_or("renderer_resource_missing")?;
            require(image.artifact.path==directory.join(name) && witness["sha256"]==image.artifact.sha256
                && witness["size"]==image.size,"renderer_resource_identity")?;
            total+=image.size;require(total<=512*1024*1024,"renderer_hash_budget")?;verify_image(image)?;
        }
        require(self.installation.path==root.join("onboarding").join(ENVIRONMENT).join(format!("{INSTALLATION}-result.json"))
            && observation["input"]["result_sha256"]==self.installation.sha256,"renderer_installation_identity")?;
        self.installation.verify()?;self.environment.runner.verify()
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
    #[test] fn closed_modes_and_observations() {
        assert_eq!(RendererPolicy::Inherited.arguments(), &[] as &[&str]);
        assert_eq!(RendererPolicy::SoftwareRendering.arguments(), &["--disable-gpu"]);
        for s in ["--disable-gpu","no_sandbox","--no-sandbox","other"] {assert!(serde_json::from_value::<RendererPolicy>(json!(s)).is_err());}
        for s in ["blank_white","rendered_nonblank","unavailable"] {assert!(serde_json::from_value::<Presentation>(json!(s)).is_ok());}
        assert!(serde_json::from_value::<Presentation>(json!("screenshot")).is_err());
    }
}
