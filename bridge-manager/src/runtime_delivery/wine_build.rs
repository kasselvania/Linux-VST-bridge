//! Release-builder construction identity, not runtime policy or qualification.
//! The release catalog seals this declaration; consumers compare selected bytes.
use super::*;

pub(super) const NAME: &str = "wine-build.json";
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WineBuild {
    schema: u32,
    kind: String,
    recipe_sha256: String,
    wine_revision: String,
    sdk_image: String,
    configuration_sha256: BTreeMap<String, String>,
    invocation: Invocation,
    helper_abi: u32,
    helper_sources: BTreeMap<String, String>,
    winebuild_sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Invocation {
    target: String,
    source_date_epoch: u64,
    base_source_date_epoch: u64,
    build_name: String,
    make_arguments: Vec<String>,
}
impl WineBuild {
    fn validate(&self) -> Result<()> {
        let helpers=["tools/direct-audio-handoff/linux_wait.h",
            "tools/direct-audio-handoff/linux_wait.rs", "tools/direct-audio-handoff/shim.cpp"];
        require(self.schema==1 && self.kind=="wine_build" && self.helper_abi==1
            && valid_hex(&self.recipe_sha256,64) && valid_hex(&self.wine_revision,40)
            && valid_hex(&self.winebuild_sha256,64),"runtime_build_identity")?;
        let Some((repository,digest))=self.sdk_image.split_once("@sha256:") else {
            return Err("runtime_build_sdk_identity".into());
        };
        require(!repository.is_empty() && !repository.chars().any(char::is_whitespace)
            && valid_hex(digest,64),"runtime_build_sdk_identity")?;
        require(self.configuration_sha256.len()==2
            && ["i386","x86_64"].iter().all(|name|self.configuration_sha256.get(*name)
                .is_some_and(|value|valid_hex(value,64)))
            && self.helper_sources.len()==helpers.len()
            && helpers.iter().all(|name|self.helper_sources.get(*name)
                .is_some_and(|value|valid_hex(value,64))),"runtime_build_input_identity")?;
        require(self.invocation.target=="wine" && self.invocation.source_date_epoch>0
            && self.invocation.base_source_date_epoch>0
            && (1..=128).contains(&self.invocation.build_name.len())
            && self.invocation.build_name.bytes().all(|c|c.is_ascii_alphanumeric() || b"._-".contains(&c))
            && (1..=16).contains(&self.invocation.make_arguments.len())
            && self.invocation.make_arguments.iter().all(|arg|!arg.is_empty() && arg.len()<=256
                && arg.bytes().all(|c|c.is_ascii_graphic() && c!=b'/')),
            "runtime_build_invocation")
    }
}

pub(super) fn validate(path: &Path) -> Result<()> {
    let bytes=read_control_bytes(path,65536)?;
    let identity:WineBuild=serde_json::from_slice(&bytes)?;
    identity.validate()
}

#[cfg(test)]
pub(super) fn fixture() -> serde_json::Value {
    serde_json::json!({"schema":1,"kind":"wine_build","recipe_sha256":"11".repeat(32),
        "wine_revision":"22".repeat(20),"sdk_image":format!("registry/sdk@sha256:{}","33".repeat(32)),
        "configuration_sha256":{"i386":"44".repeat(32),"x86_64":"55".repeat(32)},
        "invocation":{"target":"wine","source_date_epoch":1,"base_source_date_epoch":1,
            "build_name":"source-fixture","make_arguments":["-j3","J=3","CONTAINER=1"]},
        "helper_abi":1,"helper_sources":{"tools/direct-audio-handoff/linux_wait.h":"66".repeat(32),
            "tools/direct-audio-handoff/linux_wait.rs":"77".repeat(32),
            "tools/direct-audio-handoff/shim.cpp":"88".repeat(32)},"winebuild_sha256":"99".repeat(32)})
}
