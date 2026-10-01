//! Signed PKG0 payload intake without writes to the protected operating system.
//! Intake stages package files only. PKG0 still owns selection and rollback.
use crate::{atomic_json, digest, file, hex, private_dir, random_id, require, Result};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, io::{Read, Seek, SeekFrom, Write},
    os::{fd::AsRawFd, unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt}},
    path::{Component, Path, PathBuf}};

const DOMAIN: &[u8] = b"Linux VST Bridge user package v1\0";
const FOOTER: &[u8; 8] = b"LVBUSER1";
const MANIFEST_MAX: u64 = 4 * 1024 * 1024;
const PAYLOAD_MAX: u64 = 2_000_000_000;
const ADOPTION: &str = "usr/share/linux-vst-bridge/pkg0-manifest.json";
const SETUP: &str = "linux-vst-bridge-setup.desktop";

fn directory(path:&Path)->Result<()> {
    fs::DirBuilder::new().recursive(true).mode(0o700).create(path)?;
    private_dir(path)
}

#[derive(Clone)]
pub struct Trust { key: VerifyingKey, class: String }
impl Trust {
    pub fn new(key: [u8; 32], class: &str) -> Result<Self> {
        require(matches!(class, "release" | "internal_test"), "user_package_key_class")?;
        let key = VerifyingKey::from_bytes(&key)?;
        require(!key.is_weak(), "user_package_weak_key")?;
        Ok(Self { key, class: class.into() })
    }
    /// A bundle cannot supply its own trust root. Only the public release key
    /// compiled into this application authorizes user-owned package inputs.
    pub fn compiled() -> Result<Self> {
        let public = option_env!("LVB_PORTABLE_ED25519_PUBLIC_KEY").unwrap_or("");
        let class = option_env!("LVB_PORTABLE_KEY_CLASS").unwrap_or("");
        require(public.len() == 64 && public.bytes().all(|b| b.is_ascii_hexdigit()),
            "user_package_release_key_unselected")?;
        let mut key = [0; 32];
        for (i, byte) in key.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&public[i * 2..i * 2 + 2], 16)?;
        }
        Self::new(key, class)
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Release {
    pub schema: u32, pub package: String, pub version: String, pub pkgrel: u32,
    pub source_head: String, pub source_tree: String, pub payload_sha256: String,
    pub files: Vec<ReleaseFile>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseFile {
    pub destination: String, pub sha256: String, pub size: u64,
    pub mode: String, pub kind: String, pub component: String,
}
fn hexadecimal(value: &str, length: usize) -> bool {
    value.len() == length && value.bytes().all(|b| b.is_ascii_digit() || matches!(b,b'a'..=b'f'))
}
fn allowed(row: &ReleaseFile) -> bool {
    let p = Path::new(&row.destination);
    let canonical = row.destination.len() <= 512 && p.components().count() <= 16
        && p.components().all(|c| matches!(c, Component::Normal(_)))
        && p.components().next() == Some(Component::Normal("usr".as_ref()))
        && !row.destination.contains("//") && !row.destination.ends_with('/');
    let role = match row.destination.as_str() {
        "usr/bin/linux-vst-bridge" => row.kind == "manager",
        "usr/bin/linux-audio-compatibility-manager" => row.kind == "frontend",
        "usr/lib/linux-vst-bridge/supervisor/session.pyc" => row.kind == "supervisor",
        "usr/lib/linux-vst-bridge/supervisor/ownership.pyc" => row.kind == "ownership",
        "usr/lib/linux-vst-bridge/host/bridge-host.exe" => row.kind == "windows_host",
        "usr/lib/linux-vst-bridge/host/source-manifest.json" => row.kind == "host_source",
        "usr/lib/linux-vst-bridge/preparation/preparation-kit.zip" => row.kind == "preparation_kit",
        ADOPTION => row.kind == "adoption_manifest",
        "usr/share/applications/linux-vst-bridge-setup.desktop" => row.kind == "system_desktop",
        "usr/share/doc/linux-vst-bridge-beta/START_HERE.html" => row.kind == "guide",
        "usr/share/doc/linux-vst-bridge-beta/THIRD_PARTY_NOTICES.txt" => row.kind == "notices",
        "usr/share/doc/linux-vst-bridge-beta/SBOM.spdx.json" => row.kind == "sbom",
        "usr/share/doc/linux-vst-bridge-beta/COMPLIANCE_MANIFEST.json" => row.kind == "compliance",
        name => [
            ("proxy", "usr/lib/linux-vst-bridge/proxy/"),
            ("fixture", "usr/lib/linux-vst-bridge/self-test/"),
            ("fixture_resource", "usr/lib/linux-vst-bridge/self-test/"),
            ("profile", "usr/lib/linux-vst-bridge/profiles/"),
            ("license", "usr/share/doc/linux-vst-bridge-beta/licenses/"),
        ].iter().any(|(kind, prefix)| row.kind == *kind && name.starts_with(prefix)),
    };
    canonical && role && row.mode == if matches!(row.kind.as_str(), "manager"|"frontend"|"proxy") { "0555" } else { "0444" }
        && hexadecimal(&row.sha256,64) && row.size <= PAYLOAD_MAX
        && !row.component.is_empty() && row.component.len() <= 128
}
fn signed_release(bytes: &[u8], signature: &[u8], base_sha256:&[u8;32], trust: &Trust) -> Result<Release> {
    require(bytes.len() <= MANIFEST_MAX as usize && signature.len() == 64, "user_package_signature_extent")?;
    let message = [DOMAIN, trust.class.as_bytes(), b"\0", base_sha256.as_slice(), bytes].concat();
    trust.key.verify_strict(&message, &Signature::from_slice(signature)?)?;
    let release: Release = serde_json::from_slice(bytes)?;
    require(release.schema == 1 && release.package == "linux-vst-bridge-beta"
        && release.pkgrel == 1 && hexadecimal(&release.source_head,40)
        && hexadecimal(&release.source_tree,40) && hexadecimal(&release.payload_sha256,64)
        && !release.version.is_empty() && release.version.len() <= 80
        && release.version.as_bytes()[0].is_ascii_digit()
        && release.version.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.')
        && (12..=512).contains(&release.files.len()), "user_package_manifest")?;
    let mut names = BTreeSet::new();
    let mut size = 0u64;
    for row in &release.files {
        size = size.checked_add(row.size).ok_or("user_package_extent")?;
        require(allowed(row) && names.insert(row.destination.as_str()) && size <= PAYLOAD_MAX,
            "user_package_roster")?;
    }
    for required in ["usr/bin/linux-vst-bridge", "usr/bin/linux-audio-compatibility-manager",
        "usr/lib/linux-vst-bridge/supervisor/session.pyc", "usr/lib/linux-vst-bridge/supervisor/ownership.pyc",
        "usr/lib/linux-vst-bridge/host/bridge-host.exe", "usr/lib/linux-vst-bridge/host/source-manifest.json",
        "usr/lib/linux-vst-bridge/preparation/preparation-kit.zip", ADOPTION,
        "usr/share/applications/linux-vst-bridge-setup.desktop",
        "usr/share/doc/linux-vst-bridge-beta/START_HERE.html",
        "usr/share/doc/linux-vst-bridge-beta/THIRD_PARTY_NOTICES.txt",
        "usr/share/doc/linux-vst-bridge-beta/SBOM.spdx.json",
        "usr/share/doc/linux-vst-bridge-beta/COMPLIANCE_MANIFEST.json"] {
        require(names.contains(required), "user_package_required_component")?;
    }
    for kind in ["proxy", "fixture", "profile", "license"] {
        require(release.files.iter().any(|r|r.kind==kind), "user_package_required_component")?;
    }
    Ok(release)
}

pub struct Bundle {
    file: fs::File, identity: crate::DigestFileIdentity, offset: u64, length: u64,
    manifest: Vec<u8>, signature: Vec<u8>, base_sha256:[u8;32], release: Release, trust: Trust,
}
impl Bundle {
    pub fn open(path: &Path, trust: Trust) -> Result<Self> {
        let mut f = file(path)?;
        let identity = crate::DigestFileIdentity::from(&f.metadata()?);
        let size = f.metadata()?.len();
        require((40..=PAYLOAD_MAX + 256 * 1024 * 1024).contains(&size),
            "user_package_bundle_extent")?;
        f.seek(SeekFrom::End(-40))?;
        let mut footer = [0; 40]; f.read_exact(&mut footer)?;
        require(&footer[..8] == FOOTER, "user_package_bundle_footer")?;
        let number = |offset| u64::from_le_bytes(footer[offset..offset+8].try_into().unwrap());
        let (offset, manifest_len, signature_len, length) = (number(8),number(16),number(24),number(32));
        require((4..=256*1024*1024).contains(&offset)
            && manifest_len <= MANIFEST_MAX && signature_len == 64 && length <= PAYLOAD_MAX
            && offset.checked_add(manifest_len).and_then(|n|n.checked_add(64))
                .and_then(|n|n.checked_add(length)).and_then(|n|n.checked_add(40)) == Some(size),
            "user_package_bundle_extent")?;
        f.seek(SeekFrom::Start(offset))?;
        let mut manifest = vec![0;manifest_len as usize]; f.read_exact(&mut manifest)?;
        let mut signature = vec![0;64]; f.read_exact(&mut signature)?;
        f.seek(SeekFrom::Start(0))?;
        let mut magic=[0;4];f.read_exact(&mut magic)?;
        require(magic==*b"\x7fELF","user_package_installer_format")?;
        f.seek(SeekFrom::Start(0))?;
        let mut hash=Sha256::new();let mut remaining=offset;let mut block=[0;65536];
        while remaining!=0 {
            let bound=block.len().min(remaining as usize);f.read_exact(&mut block[..bound])?;
            hash.update(&block[..bound]);remaining-=bound as u64;
        }
        let base_sha256=hash.finalize().into();
        let release = signed_release(&manifest,&signature,&base_sha256,&trust)?;
        require(crate::DigestFileIdentity::from(&f.metadata()?)==identity,
            "user_package_bundle_changed")?;
        Ok(Self {file:f,identity,offset:offset+manifest_len+64,length,manifest,signature,base_sha256,release,trust})
    }
    pub fn release(&self) -> &Release { &self.release }
    pub fn key_class(&self) -> &str { &self.trust.class }
    /// Builder/read-only diagnostic check. It creates no directory or selection.
    pub fn verify_payload(&mut self) -> Result<()> {
        require(crate::DigestFileIdentity::from(&self.file.metadata()?) == self.identity,
            "user_package_bundle_changed")?;
        self.file.seek(SeekFrom::Start(self.offset))?;
        let mut hash=Sha256::new();let mut remaining=self.length;let mut block=[0;65536];
        while remaining!=0 {
            let bound=block.len().min(remaining as usize);
            self.file.read_exact(&mut block[..bound])?;
            hash.update(&block[..bound]);remaining-=bound as u64;
        }
        require(hex(&hash.finalize())==self.release.payload_sha256
            && crate::DigestFileIdentity::from(&self.file.metadata()?)==self.identity,
            "user_package_payload_changed")
    }
    pub fn stage(&mut self, home: &Path) -> Result<PathBuf> {
        self.stage_with_space(home, |lock| {
            let mut space:libc::statvfs = unsafe{std::mem::zeroed()};
            require(unsafe{libc::fstatvfs(lock.as_raw_fd(),&mut space)} == 0,
                "user_package_space_unavailable")?;
            Ok((space.f_bavail as u128)*(space.f_frsize as u128))
        })
    }
    fn stage_with_space(&mut self, home:&Path,
        available:impl FnOnce(&fs::File)->Result<u128>) -> Result<PathBuf> {
        require(home.is_absolute() && home.canonicalize()? == home,
            "user_package_home_identity")?;
        let parent = home.join(".local/share/linux-vst-bridge/packages");
        directory(&parent)?;
        require(parent.canonicalize()? == parent, "user_package_directory_identity")?;
        let lock = fs::OpenOptions::new().read(true).write(true).create(true).truncate(false)
            .mode(0o600).custom_flags(libc::O_NOFOLLOW).open(parent.join("intake.lock"))?;
        require(lock.metadata()?.uid() == unsafe{libc::getuid()}
            && unsafe{libc::flock(lock.as_raw_fd(),libc::LOCK_EX|libc::LOCK_NB)} == 0,
            "user_package_intake_busy")?;
        let id = hex(&Sha256::digest(&self.manifest));
        let destination = parent.join(&id);
        if destination.try_exists()? {
            verify_directory(&destination,&self.trust)?;
            return Ok(destination.join("usr/bin/linux-audio-compatibility-manager"));
        }
        let needed = self.release.files.iter().try_fold(64*1024*1024u64, |n,r|n.checked_add(r.size))
            .ok_or("user_package_extent")?;
        require(available(&lock)? >= needed as u128,
            "Not enough disk space. Existing application and projects were retained")?;
        let staged = parent.join(format!(".installing-{}",random_id()?));
        directory(&staged)?;
        let result = self.extract(&staged).and_then(|()| {
            fs::write(staged.join("RELEASE_MANIFEST.json"),&self.manifest)?;
            fs::write(staged.join("RELEASE_MANIFEST.ed25519"),&self.signature)?;
            fs::write(staged.join("INSTALLER_BASE.sha256"),hex(&self.base_sha256))?;
            for name in ["RELEASE_MANIFEST.json","RELEASE_MANIFEST.ed25519","INSTALLER_BASE.sha256"] {
                fs::set_permissions(staged.join(name),fs::Permissions::from_mode(0o444))?;
                file(&staged.join(name))?.sync_all()?;
            }
            verify_directory(&staged,&self.trust)?;
            fs::rename(&staged,&destination)?;
            fs::File::open(&parent)?.sync_all()?;
            Ok(())
        });
        if result.is_err() { let _ = fs::remove_dir_all(&staged); }
        result?;
        Ok(destination.join("usr/bin/linux-audio-compatibility-manager"))
    }
    fn extract(&mut self, destination:&Path) -> Result<()> {
        self.verify_payload()?;
        self.file.seek(SeekFrom::Start(self.offset))?;
        let mut archive=tar::Archive::new((&mut self.file).take(self.length));
        let mut seen=BTreeSet::new();
        for entry in archive.entries()? {
            let mut entry=entry?;
            let path=entry.path()?.into_owned();
            let name=path.to_str().ok_or("user_package_path")?;
            let row=self.release.files.iter().find(|r|r.destination==name)
                .ok_or("user_package_undeclared_entry")?;
            require(entry.header().entry_type().is_file() && entry.size()==row.size
                && seen.insert(name.to_owned()), "user_package_entry_identity")?;
            let target=destination.join(&path);
            directory(target.parent().ok_or("user_package_parent")?)?;
            let mut output=fs::OpenOptions::new().write(true).create_new(true)
                .mode(0o600).custom_flags(libc::O_NOFOLLOW).open(&target)?;
            std::io::copy(&mut entry,&mut output)?;
            output.sync_all()?;
            require(digest(&target)? == row.sha256, "user_package_entry_changed")?;
            output.set_permissions(fs::Permissions::from_mode(u32::from_str_radix(&row.mode,8)?))?;
        }
        require(seen.len()==self.release.files.len()
            && crate::DigestFileIdentity::from(&self.file.metadata()?) == self.identity,
            "user_package_bundle_changed")?;
        Ok(())
    }
}

fn verified_bytes(path:&Path, maximum:u64) -> Result<Vec<u8>> {
    let mut f=file(path)?;
    let meta=f.metadata()?;
    require(meta.uid()==unsafe{libc::getuid()} && meta.mode() & 0o222==0 && meta.len()<=maximum,
        "user_package_file_custody")?;
    let before=crate::DigestFileIdentity::from(&meta);
    let mut bytes=Vec::new(); (&mut f).take(maximum+1).read_to_end(&mut bytes)?;
    require(bytes.len() as u64==meta.len() && crate::DigestFileIdentity::from(&f.metadata()?)==before
        && crate::DigestFileIdentity::from(&fs::symlink_metadata(path)?)==before,
        "user_package_file_changed")?;
    Ok(bytes)
}
pub fn verify_directory(root:&Path, trust:&Trust) -> Result<Release> {
    require(root.is_absolute() && root.canonicalize()?==root,
        "user_package_directory_identity")?;
    let bytes=verified_bytes(&root.join("RELEASE_MANIFEST.json"),MANIFEST_MAX)?;
    let signature=verified_bytes(&root.join("RELEASE_MANIFEST.ed25519"),64)?;
    let base=verified_bytes(&root.join("INSTALLER_BASE.sha256"),64)?;
    let base=std::str::from_utf8(&base)?;
    require(hexadecimal(base,64),"user_package_installer_identity")?;
    let mut base_sha256=[0;32];
    for (i,byte) in base_sha256.iter_mut().enumerate() {
        *byte=u8::from_str_radix(&base[i*2..i*2+2],16)?;
    }
    let release=signed_release(&bytes,&signature,&base_sha256,trust)?;
    let mut expected: BTreeSet<_> = release.files.iter().map(|r|PathBuf::from(&r.destination)).collect();
    expected.insert("RELEASE_MANIFEST.json".into());expected.insert("RELEASE_MANIFEST.ed25519".into());
    expected.insert("INSTALLER_BASE.sha256".into());
    let mut todo=vec![root.to_path_buf()];let mut visited=0usize;
    while let Some(directory)=todo.pop() {
        let metadata=fs::symlink_metadata(&directory)?;
        require(metadata.is_dir() && metadata.uid()==unsafe{libc::getuid()}
            && metadata.mode() & 0o077==0, "user_package_directory_custody")?;
        for entry in fs::read_dir(&directory)? {
            visited+=1;require(visited<=4096,"user_package_directory_extent")?;
            let entry=entry?;let path=entry.path();let metadata=fs::symlink_metadata(&path)?;
            if metadata.is_dir() {todo.push(path);}
            else {require(metadata.is_file() && expected.remove(path.strip_prefix(root)?),
                "user_package_roster_changed")?;}
        }
    }
    require(expected.is_empty(),"user_package_roster_changed")?;
    for row in &release.files {
        let p=root.join(&row.destination);
        require(p.canonicalize()?==p,"user_package_link")?;
        let meta=file(&p)?.metadata()?;
        require(meta.uid()==unsafe{libc::getuid()} && meta.is_file() && meta.len()==row.size
            && meta.mode() & 0o777==u32::from_str_radix(&row.mode,8)?,"user_package_file_custody")?;
        require(digest(&p)?==row.sha256,"user_package_file_changed")?;
    }
    // Package bytecode is coupled to the target Python minor version. Refuse
    // before adoption rather than create a selected service that cannot start.
    verify_python(root)?;
    Ok(release)
}
fn verify_python(root:&Path) -> Result<()> {
    use std::{process::{Command,Stdio},time::{Duration,Instant}};
    let mut expected=None;
    for name in ["session.pyc","ownership.pyc"] {
        let mut source=file(&root.join("usr/lib/linux-vst-bridge/supervisor").join(name))?;
        let mut header=[0;16];source.read_exact(&mut header)?;
        require(matches!(u32::from_le_bytes(header[4..8].try_into()?),0|1|3),
            "user_package_python_header")?;
        let magic=hex(&header[..4]);
        require(expected.as_ref().is_none_or(|old|old==&magic),"user_package_python_header")?;
        expected=Some(magic);
    }
    let mut child=Command::new("/usr/bin/python3").args(["-I","-S","-c",
        "import importlib.util; print(importlib.util.MAGIC_NUMBER.hex())"])
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
    let deadline=Instant::now()+Duration::from_secs(3);
    let status=loop {
        if let Some(status)=child.try_wait()? {break status;}
        if Instant::now()>=deadline {let _=child.kill();let _=child.wait();return Err("user_package_python_probe_timeout".into());}
        std::thread::sleep(Duration::from_millis(10));
    };
    let mut bytes=Vec::new();child.stdout.take().ok_or("user_package_python_probe")?
        .take(32).read_to_end(&mut bytes)?;
    require(status.success() && std::str::from_utf8(&bytes)?.trim()==expected.as_deref().unwrap_or(""),
        "This build requires a different system Python version. Your selected application was retained")
}
/// Only the running source-package executable selects its source. A selected
/// generation executable is an ordinary product route, not a package input.
pub fn input_root(executable:&Path, home:&Path) -> Result<Option<PathBuf>> {
    let base=home.join(".local/share/linux-vst-bridge/packages");
    if !executable.starts_with(&base) { return Ok(None); }
    require(executable.canonicalize()?==executable,"user_package_executable_identity")?;
    let relative=executable.strip_prefix(&base)?;
    let parts=relative.iter().collect::<Vec<_>>();
    require(parts.len()==4 && parts[0].to_str().is_some_and(|s|hexadecimal(s,64))
        && parts[1]=="usr" && parts[2]=="bin"
        && matches!(parts[3].to_str(),Some("linux-vst-bridge"|"linux-audio-compatibility-manager")),
        "user_package_executable_identity")?;
    let root=base.join(parts[0]);
    let bytes=verified_bytes(&root.join("RELEASE_MANIFEST.json"),MANIFEST_MAX)?;
    require(Some(hex(&Sha256::digest(&bytes)).as_str())==parts[0].to_str(),
        "user_package_directory_identity")?;
    verify_directory(&root,&Trust::compiled()?)?;
    Ok(Some(root.join("usr")))
}

/// The new Setup entry points to staged, verified files. Selection and starting
/// the bridge remain explicit actions in the ordinary PKG0 frontend.
pub fn expose_setup(frontend:&Path, home:&Path) -> Result<()> {
    let root=input_root(frontend,home)?.ok_or("user_package_source_required")?;
    let apps=home.join(".local/share/applications");
    fs::create_dir_all(&apps)?;
    let metadata=fs::symlink_metadata(&apps)?;
    require(metadata.is_dir() && metadata.uid()==unsafe{libc::getuid()}
        && metadata.mode() & 0o022==0 && apps.canonicalize()?==apps,
        "user_package_desktop_directory")?;
    let path=apps.join(SETUP);
    let escape=|p:&Path| -> Result<String> {
        let text=p.to_str().ok_or("user_package_desktop_path")?;
        require(!text.chars().any(char::is_control),"user_package_desktop_path")?;
        Ok(text.replace('\\',"\\\\").replace('"',"\\\"").replace('`',"\\`").replace('$',"\\$").replace('%',"%%"))
    };
    let prefix="[Desktop Entry]\nType=Application\nName=Linux VST Bridge Setup and Updates\nX-LVB-User-Package=1\n";
    if path.try_exists()? {
        let old=file(&path)?;let meta=old.metadata()?;
        require(meta.uid()==unsafe{libc::getuid()} && meta.len()<=8192,"user_package_desktop_owner")?;
        let mut text=String::new();old.take(8193).read_to_string(&mut text)?;
        require(text.starts_with(prefix),"Existing Setup entry is not owned by this installer; it was retained")?;
    }
    let text=format!("{prefix}Exec=\"{}\"\nIcon=audio-card\nTerminal=false\nCategories=AudioVideo;Audio;\nStartupNotify=true\n",escape(&root.join("bin/linux-audio-compatibility-manager"))?);
    let temporary=apps.join(format!(".lvb-setup-{}",random_id()?));
    let mut output=fs::OpenOptions::new().create_new(true).write(true).mode(0o600).open(&temporary)?;
    output.write_all(text.as_bytes())?;output.sync_all()?;
    fs::rename(&temporary,&path)?;fs::File::open(&apps)?.sync_all()?;
    atomic_json(&home.join(".local/share/linux-vst-bridge/packages/setup.json"),
        &serde_json::json!({"schema":1,"frontend":frontend}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer,SigningKey};
    use std::{io::Cursor,process::Command};
    struct Fixture {home:PathBuf,key:SigningKey,rows:Vec<(ReleaseFile,Vec<u8>)>}
    impl Fixture {
        fn new()->Self {
            let home=std::env::temp_dir().join(format!("lvb-user-package-{}",random_id().unwrap()));
            directory(&home).unwrap();
            let output=Command::new("/usr/bin/python3").args(["-I","-S","-c",
                "import importlib.util,sys;sys.stdout.buffer.write(importlib.util.MAGIC_NUMBER)"])
                .output().unwrap();assert!(output.status.success());
            let paths=[
                ("usr/bin/linux-vst-bridge","manager"),
                ("usr/bin/linux-audio-compatibility-manager","frontend"),
                ("usr/lib/linux-vst-bridge/supervisor/session.pyc","supervisor"),
                ("usr/lib/linux-vst-bridge/supervisor/ownership.pyc","ownership"),
                ("usr/lib/linux-vst-bridge/host/bridge-host.exe","windows_host"),
                ("usr/lib/linux-vst-bridge/host/source-manifest.json","host_source"),
                ("usr/lib/linux-vst-bridge/preparation/preparation-kit.zip","preparation_kit"),
                (ADOPTION,"adoption_manifest"),
                ("usr/share/applications/linux-vst-bridge-setup.desktop","system_desktop"),
                ("usr/share/doc/linux-vst-bridge-beta/START_HERE.html","guide"),
                ("usr/share/doc/linux-vst-bridge-beta/THIRD_PARTY_NOTICES.txt","notices"),
                ("usr/share/doc/linux-vst-bridge-beta/SBOM.spdx.json","sbom"),
                ("usr/share/doc/linux-vst-bridge-beta/COMPLIANCE_MANIFEST.json","compliance"),
                ("usr/lib/linux-vst-bridge/proxy/Fixture.so","proxy"),
                ("usr/lib/linux-vst-bridge/self-test/Fixture.exe","fixture"),
                ("usr/lib/linux-vst-bridge/profiles/fixture.json","profile"),
                ("usr/share/doc/linux-vst-bridge-beta/licenses/fixture.txt","license"),
            ];
            let rows=paths.into_iter().map(|(name,kind)| {
                let bytes=if matches!(kind,"supervisor"|"ownership") {
                    [output.stdout.as_slice(), &[0;12]].concat()
                } else {format!("exact fixture {name}").into_bytes()};
                (ReleaseFile{destination:name.into(),sha256:hex(&Sha256::digest(&bytes)),size:bytes.len() as u64,
                    kind:kind.into(),mode:if matches!(kind,"manager"|"frontend"|"proxy") {"0555"} else {"0444"}.into(),
                    component:"source-owned-test".into()},bytes)
            }).collect();
            Self{home,key:SigningKey::from_bytes(&[0x51;32]),rows}
        }
        fn trust(&self)->Trust {Trust::new(self.key.verifying_key().to_bytes(),"internal_test").unwrap()}
        fn bundle(&self,version:&str)->PathBuf {
            let mut payload=Vec::new();
            {let mut archive=tar::Builder::new(&mut payload);
                for (row,bytes) in &self.rows {
                    let mut header=tar::Header::new_gnu();header.set_size(bytes.len() as u64);
                    header.set_mode(u32::from_str_radix(&row.mode,8).unwrap());header.set_cksum();
                    archive.append_data(&mut header,&row.destination,Cursor::new(bytes)).unwrap();
                }
                archive.finish().unwrap();
            }
            let manifest=serde_json::to_vec(&Release{schema:1,package:"linux-vst-bridge-beta".into(),
                version:version.into(),pkgrel:1,source_head:"ab".repeat(20),source_tree:"cd".repeat(20),
                payload_sha256:hex(&Sha256::digest(&payload)),files:self.rows.iter().map(|(row,_)|row.clone()).collect()}).unwrap();
            let base=b"\x7fELF source-owned test; never executed";
            let base_sha256=Sha256::digest(base);
            let message=[DOMAIN,b"internal_test",b"\0",&base_sha256[..],manifest.as_slice()].concat();
            let signature=self.key.sign(&message).to_bytes();
            let path=self.home.join(format!("installer-{version}"));
            let bytes=[base.as_slice(),manifest.as_slice(),signature.as_slice(),payload.as_slice(),
                FOOTER.as_slice(),&(base.len() as u64).to_le_bytes(),&(manifest.len() as u64).to_le_bytes(),
                &64u64.to_le_bytes(),&(payload.len() as u64).to_le_bytes()].concat();
            fs::write(&path,bytes).unwrap();path
        }
        fn old_state(&self)->Vec<u8> {
            let path=self.home.join(".local/share/linux-vst-bridge/managed/software.json");
            directory(path.parent().unwrap()).unwrap();fs::write(&path,b"working predecessor").unwrap();
            fs::write(self.home.join("music.project"),b"work created after update").unwrap();
            fs::read(path).unwrap()
        }
        fn unchanged(&self,old:&[u8]) {
            assert_eq!(fs::read(self.home.join(".local/share/linux-vst-bridge/managed/software.json")).unwrap(),old);
            assert_eq!(fs::read(self.home.join("music.project")).unwrap(),b"work created after update");
        }
    }
    impl Drop for Fixture {fn drop(&mut self){let _=fs::remove_dir_all(&self.home);}}
    #[test]
    fn signed_stage_retains_selected_software_projects_and_both_package_versions() {
        let f=Fixture::new();let old=f.old_state();
        let first=Bundle::open(&f.bundle("1.0"),f.trust()).unwrap().stage(&f.home).unwrap();
        let second=Bundle::open(&f.bundle("1.1"),f.trust()).unwrap().stage(&f.home).unwrap();
        assert_ne!(first,second);assert!(first.exists() && second.exists());
        f.unchanged(&old);
        assert_eq!(Bundle::open(&f.bundle("1.1"),f.trust()).unwrap().stage(&f.home).unwrap(),second);
        fs::set_permissions(&first,fs::Permissions::from_mode(0o755)).unwrap();
        assert!(Bundle::open(&f.bundle("1.0"),f.trust()).unwrap().stage(&f.home).is_err());
        f.unchanged(&old);
    }
    #[test]
    fn wrong_key_class_signature_and_payload_refuse_before_selection() {
        let f=Fixture::new();let old=f.old_state();let path=f.bundle("1.0");
        let wrong=SigningKey::from_bytes(&[0x52;32]);
        assert!(Bundle::open(&path,Trust::new(wrong.verifying_key().to_bytes(),"internal_test").unwrap()).is_err());
        assert!(Bundle::open(&path,Trust::new(f.key.verifying_key().to_bytes(),"release").unwrap()).is_err());
        let mut bytes=fs::read(&path).unwrap();bytes[60]^=1;fs::write(&path,&bytes).unwrap();
        assert!(Bundle::open(&path,f.trust()).is_err());
        // The unchanged signed payload cannot authorize a substituted launcher.
        let path=f.bundle("1.0");let mut bytes=fs::read(&path).unwrap();
        bytes[5]^=1;fs::write(&path,bytes).unwrap();
        assert!(Bundle::open(&path,f.trust()).is_err());f.unchanged(&old);
        let path=f.bundle("1.0");let bundle=Bundle::open(&path,f.trust()).unwrap();
        let mut bytes=fs::read(&path).unwrap();bytes[bundle.offset as usize+100]^=1;fs::write(&path,bytes).unwrap();
        let mut bundle=Bundle::open(&path,f.trust()).unwrap();
        assert!(bundle.stage(&f.home).is_err());f.unchanged(&old);
        assert!(!fs::read_dir(f.home.join(".local/share/linux-vst-bridge/packages")).unwrap()
            .any(|e|e.unwrap().file_name().to_string_lossy().starts_with(".installing-")));
    }
    #[test]
    fn low_disk_and_python_mismatch_preserve_working_state() {
        let mut f=Fixture::new();let old=f.old_state();let path=f.bundle("1.0");
        let mut bundle=Bundle::open(&path,f.trust()).unwrap();
        let error=bundle.stage_with_space(&f.home, |_|Ok(0)).unwrap_err().to_string();
        assert!(error.contains("Not enough disk space"));f.unchanged(&old);
        for (row,bytes) in &mut f.rows {
            if matches!(row.kind.as_str(),"supervisor"|"ownership") {
                bytes[..4].fill(0);row.sha256=hex(&Sha256::digest(bytes));
            }
        }
        let error=Bundle::open(&f.bundle("1.1"),f.trust()).unwrap().stage(&f.home).unwrap_err().to_string();
        assert!(error.contains("different system Python"));f.unchanged(&old);
    }
    #[test]
    fn signed_unsafe_paths_and_duplicates_cannot_escape_custody() {
        let mut f=Fixture::new();let old=f.old_state();
        for name in ["../escape","usr/bin/../../escape","/tmp/escape","usr/bin//escape"] {
            f.rows[13].0.destination=name.into();
            assert!(!allowed(&f.rows[13].0));
        }
        f.rows[13].0.destination=f.rows[0].0.destination.clone();
        f.rows[13].0.kind=f.rows[0].0.kind.clone();
        assert!(Bundle::open(&f.bundle("1.0"),f.trust()).is_err());
        f.unchanged(&old);
    }
    #[test]
    fn symlink_substitution_refuses_even_identical_bytes() {
        let f=Fixture::new();let old=f.old_state();let path=f.bundle("1.0");
        let frontend=Bundle::open(&path,f.trust()).unwrap().stage(&f.home).unwrap();
        let replacement=f.home.join("same-bytes");fs::write(&replacement,fs::read(&frontend).unwrap()).unwrap();
        fs::remove_file(&frontend).unwrap();std::os::unix::fs::symlink(&replacement,&frontend).unwrap();
        assert!(Bundle::open(&path,f.trust()).unwrap().stage(&f.home).is_err());
        f.unchanged(&old);
    }
}
