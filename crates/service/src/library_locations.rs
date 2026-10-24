//! User-selected external directories. The setup record selects an immutable
//! set; the accepted operator job owns the sole before/after revision edge.
use crate::*;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use crate::library_location_plan::{LocationGrant, LocationPlan};

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(deny_unknown_fields)]
pub struct DirectoryIdentity {
    pub volume: String,
    pub handle_type: i32,
    pub handle: Vec<u8>,
}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(deny_unknown_fields)]
pub struct Location {
    pub id:String,
    pub path:PathBuf,
    pub identity:DirectoryIdentity,
    pub drive:String,
    pub read_only:bool,
    /// The inode of the link created in this setup, never its target contents.
    pub link_inode:u64,
}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(deny_unknown_fields)]
pub struct LocationSet { pub schema:u32, pub locations:Vec<Location> }
impl Default for LocationSet { fn default()->Self {Self {schema:1,locations:vec![]}} }
pub fn environment_digest(environment:&Environment)->Result<String> {
    Ok(hex(&Sha256::digest(serde_json::to_vec(environment)?)))
}
fn set_path(m:&Manager,digest:&str)->Result<PathBuf> {
    require(valid_hex(digest,64),"library_location_set_identity")?;
    Ok(m.root.join("library-locations").join(format!("{digest}.json")))
}
pub fn load(m:&Manager,environment:&Environment)->Result<LocationSet> {
    let Some(digest)=&environment.library_locations else {return Ok(LocationSet::default())};
    let set:LocationSet=Artifact{path:set_path(m,digest)?,sha256:digest.clone()}.read_record(256*1024)?;
    require(set.schema==1 && set.locations.len()<=13,"library_location_set_schema")?;
    let mut drives=std::collections::BTreeSet::new();
    let mut identities=std::collections::BTreeSet::new();
    for location in &set.locations {
        require(valid_hex(&location.id,64) && valid_drive(&location.drive)
            && drives.insert(location.drive.clone()) && identities.insert(location.id.clone())
            && location.path.is_absolute() && !location.identity.volume.is_empty()
            && !location.identity.handle.is_empty() && location.identity.handle.len()<=128,
            "library_location_set_binding")?;
    }
    Ok(set)
}
pub fn retain(m:&Manager,set:&LocationSet)->Result<String> {
    // Keep the exact encoded bytes as the capability reference, including an
    // explicit empty set after the last removal.
    let mut bytes=serde_json::to_vec_pretty(set)?; bytes.push(b'\n');
    let digest=hex(&Sha256::digest(&bytes)); let path=set_path(m,&digest)?;
    private_dir(path.parent().unwrap())?;
    if path.try_exists()? {require(fs::read(&path)?==bytes,"library_location_set_collision")?;}
    else {
        let mut file=fs::OpenOptions::new().write(true).create_new(true).mode(0o400).open(&path)?;
        use std::io::Write;file.write_all(&bytes)?;file.sync_all()?;
        fs::File::open(path.parent().unwrap())?.sync_all()?;
    }
    Ok(digest)
}
pub fn requires_capability(m:&Manager)->Result<bool> {
    let root=m.root.join("environments");if !root.try_exists()? {return Ok(false)}
    for (index,entry) in fs::read_dir(root)?.enumerate() {
        require(index<4096,"library_environment_bound")?;
        let path=entry?.path().join("environment.json");
        if path.try_exists()? && read_json::<Environment>(&path)?.library_locations.is_some(){return Ok(true)}
    }
    Ok(false)
}
fn valid_drive(drive:&str)->bool {
    drive.len()==2 && drive.as_bytes()[1]==b':'
        && (b'L'..=b'Y').contains(&drive.as_bytes()[0]) && drive!="S:"
}
pub fn dosdevices(environment:&Environment)->PathBuf {environment.root.join("compatdata/pfx/dosdevices")}
pub fn link_path(environment:&Environment,drive:&str)->Result<PathBuf> {
    require(valid_drive(drive),"library_drive_invalid")?;
    Ok(dosdevices(environment).join(drive.to_ascii_lowercase()))
}
pub fn unused_drive(environment:&Environment,set:&LocationSet)->Result<String> {
    let root=dosdevices(environment); private_dir(&root)?;
    require(root.canonicalize()?==root,"library_dosdevices_symlink")?;
    for letter in b'L'..=b'Y' {
        let drive=format!("{}:",char::from(letter));
        if !valid_drive(&drive) || set.locations.iter().any(|l|l.drive==drive){continue;}
        // Preserve both a Wine mapping and its raw-device companion, including
        // dangling links. Never claim C:, Z: or Proton's S:.
        let occupied=[root.join(drive.to_ascii_lowercase()),root.join(format!("{}:",drive.to_ascii_lowercase())),
            root.join(&drive),root.join(format!("{drive}:"))]
            .iter().try_fold(false,|occupied,path|->Result<bool>{match fs::symlink_metadata(path){
                Ok(_)=>Ok(true),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(occupied),Err(e)=>Err(e.into())}})?;
        if !occupied {return Ok(drive)}
    }
    Err("library_drive_letters_occupied".into())
}
pub fn verify_link(environment:&Environment,location:&Location)->Result<()> {
    let path=link_path(environment,&location.drive)?;let metadata=fs::symlink_metadata(&path)?;
    require(metadata.file_type().is_symlink() && metadata.uid()==unsafe{libc::getuid()}
        && metadata.ino()==location.link_inode && fs::read_link(&path)?==location.path,
        "library_mapping_foreign")
}
/// Reject a system tree, any product tree, or an ancestor exposing either.
fn allowed_path(m:&Manager,path:&Path)->Result<()> {
    require(path.is_absolute() && path!=Path::new("/") && path.to_str().is_some_and(|s|
        !s.chars().any(char::is_control) && !s.contains(':')),"library_path_invalid")?;
    for internal in [&m.root,&m.publications] {
        require(!path.starts_with(internal) && !internal.starts_with(path),"library_product_path")?;
    }
    for system in ["/bin","/sbin","/usr","/etc","/proc","/sys","/dev","/boot","/lib","/lib64","/opt","/var","/tmp","/root"] {
        require(!path.starts_with(system),"library_system_path")?;
    }
    require(!path.starts_with("/run") || path.starts_with("/run/media"),"library_system_path")?;
    require(path!=Path::new("/home") && path!=Path::new("/run") && path!=Path::new("/mnt")
        && path!=Path::new("/media") && path!=Path::new("/run/media"),"library_system_path")
}
#[cfg(target_os="linux")]
fn stable_identity(file:&fs::File)->Result<DirectoryIdentity> {
    use std::io::Read;
    use std::os::unix::fs::FileTypeExt;
    // A btrfs subvolume's st_dev is an anonymous device, not its backing
    // block device. The kernel export handle identifies this exact mount.
    let (mount,handle_type,handle)=directory_handle(file.as_raw_fd())?;
    let mut mountinfo=String::new();
    fs::File::open("/proc/self/mountinfo")?.take(4*1024*1024+1).read_to_string(&mut mountinfo)?;
    require(mountinfo.len()<=4*1024*1024,"library_mountinfo_bound")?;
    let source=mount_source(&mountinfo,mount)?;
    let source_metadata=fs::metadata(&source).map_err(|_|"library_volume_identity_unavailable")?;
    require(source_metadata.file_type().is_block_device(),"library_volume_block_source_required")?;
    let mut volume=None;
    for entry in fs::read_dir("/dev/disk/by-uuid").map_err(|_|"library_volume_identity_unavailable")? {
        let entry=entry?;
        if fs::metadata(entry.path()).is_ok_and(|m|m.file_type().is_block_device() && m.rdev()==source_metadata.rdev()) {
            require(volume.is_none(),"library_volume_identity_ambiguous")?;
            volume=entry.file_name().to_str().map(str::to_owned);
        }
    }
    let volume=volume.ok_or("library_volume_identity_unavailable")?;
    require(!volume.is_empty() && directory_handle(file.as_raw_fd())?==(mount,handle_type,handle.clone())
        && fs::metadata(&source)?.rdev()==source_metadata.rdev(),"library_mount_identity_changed")?;
    Ok(DirectoryIdentity{volume,handle_type,handle})
}
/// Resolve the block source of the exact kernel mount, including subvolumes
/// and bind mounts. Mountpoint paths and directory st_dev are not authority.
#[cfg(any(target_os="linux",test))]
fn mount_source(mountinfo:&str,mount_id:i32)->Result<PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    require(mount_id>0,"library_mount_identity_unavailable")?;
    let mut found=None;
    for line in mountinfo.lines() {
        let fields:Vec<_>=line.split_ascii_whitespace().collect();
        if fields.first().and_then(|id|id.parse::<i32>().ok())!=Some(mount_id){continue;}
        require(found.is_none(),"library_mount_identity_ambiguous")?;
        let separator=fields.iter().position(|field|*field=="-").ok_or("library_mountinfo_invalid")?;
        require(separator>=6 && fields.len()==separator+4,"library_mountinfo_invalid")?;
        let encoded=fields[separator+2].as_bytes();let mut decoded=Vec::new();let mut cursor=0;
        while cursor<encoded.len() {
            if encoded[cursor]==b'\\' {
                require(cursor+3<encoded.len() && encoded[cursor+1..=cursor+3].iter()
                    .all(|b|(b'0'..=b'7').contains(b)),"library_mountinfo_escape")?;
                let value=u16::from(encoded[cursor+1]-b'0')*64
                    +u16::from(encoded[cursor+2]-b'0')*8+u16::from(encoded[cursor+3]-b'0');
                require(value>0 && value<=255,"library_mountinfo_escape")?;
                decoded.push(value as u8);cursor+=4;
            } else {decoded.push(encoded[cursor]);cursor+=1;}
        }
        let source=PathBuf::from(std::ffi::OsString::from_vec(decoded));
        require(source.is_absolute(),"library_volume_block_source_required")?;
        found=Some(source);
    }
    found.ok_or_else(||"library_mount_identity_unavailable".into())
}
#[cfg(not(target_os="linux"))]
fn stable_identity(_: &fs::File)->Result<DirectoryIdentity> {Err("library_persistent_directory_identity_requires_linux".into())}
#[cfg(target_os="linux")]
fn directory_handle(fd:RawFd)->Result<(i32,i32,Vec<u8>)> {
    #[repr(C)] struct Handle {bytes:u32,kind:i32,data:[u8;128]}
    let mut handle=Handle{bytes:128,kind:0,data:[0;128]};let mut mount=0i32;
    let result=unsafe{libc::syscall(libc::SYS_name_to_handle_at,fd,c"".as_ptr(),
        &mut handle as *mut Handle,&mut mount as *mut i32,libc::AT_EMPTY_PATH)};
    require(result==0 && mount>0 && handle.bytes>0 && handle.bytes<=128,"library_persistent_directory_handle_unavailable")?;
    Ok((mount,handle.kind,handle.data[..handle.bytes as usize].to_vec()))
}
pub fn selected_directory(m:&Manager,file:&fs::File)->Result<(PathBuf,DirectoryIdentity)> {
    require(file.metadata()?.is_dir(),"library_directory_descriptor_required")?;
    #[cfg(target_os="linux")]
    let path=fs::read_link(format!("/proc/self/fd/{}",file.as_raw_fd()))?;
    #[cfg(not(target_os="linux"))]
    let path=PathBuf::new();
    allowed_path(m,&path)?;
    require(path.canonicalize()?==path,"library_directory_path_changed")?;
    let current=open_directory(&path)?;
    let a=file.metadata()?;let b=current.metadata()?;
    require(a.dev()==b.dev() && a.ino()==b.ino(),"library_directory_descriptor_changed")?;
    Ok((path,stable_identity(file)?))
}
fn open_directory(path:&Path)->Result<fs::File> {
    Ok(fs::OpenOptions::new().read(true).custom_flags(libc::O_DIRECTORY|libc::O_NOFOLLOW|libc::O_CLOEXEC).open(path)?)
}
pub fn observe(location:&Location)->Result<fs::File> {
    let file=open_directory(&location.path)?;
    require(stable_identity(&file)?==location.identity,"library_directory_unavailable_or_substituted")?;
    Ok(file)
}
pub fn launch_plan(m:&Manager,environment:&Environment)->Result<Option<LocationPlan>> {
    let Some(digest)=&environment.library_locations else {return Ok(None)};
    let mut locations=vec![];
    for location in load(m,environment)?.locations {
        let observed=verify_link(environment,&location).and_then(|_|observe(&location))
            .map_err(|_| format!("Library folder {} ({}) is unavailable. Reconnect its original drive or remove this folder mapping before launching this setup.", location.path.display(), location.drive))?;
        let stat=observed.metadata()?;
        locations.push(LocationGrant{path:location.path,drive:location.drive,read_only:location.read_only,
            available:true,device:stat.dev(),inode:stat.ino(),volume:location.identity.volume,
            handle_type:location.identity.handle_type,handle:location.identity.handle});
    }
    Ok(Some(LocationPlan{schema:1,digest:digest.clone(),locations}))
}

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
#[serde(deny_unknown_fields)]
pub struct RevisionBinding {pub schema:u32,pub operation:String,pub before:Environment,pub environment:Environment,pub cancelled:bool}
pub const REVISION_FILE:&str="library-location-revision.json";
pub fn revision(m:&Manager,operation:&str)->Result<RevisionBinding> {
    require(valid_hex(operation,32),"library_operation_identity")?;
    let directory=m.root.join("operator").join(operation);
    let binding:RevisionBinding=read_json(&directory.join(REVISION_FILE))?;
    let request:operator_model::Request=read_json(&directory.join("request.json"))?;
    let (id,expected)=match &request.action {
        operator_model::Action::AddLibraryLocation{environment,expected_environment,..}
        |operator_model::Action::RemoveLibraryLocation{environment,expected_environment,..}=>(environment,expected_environment),
        _=>return Err("library_revision_request".into()),
    };
    let mut after=binding.before.clone();after.revision=after.revision.checked_add(1).ok_or("library_revision_bound")?;
    after.library_locations=binding.environment.library_locations.clone();
    require(binding.schema==1 && binding.operation==operation && after==binding.environment
        && after.library_locations.is_some() && after.library_locations!=binding.before.library_locations
        && id==&after.id && *expected==environment_digest(&binding.before)?
        && after.root==m.root.join("environments").join(id),"library_revision_binding")?;
    let before_set=load(m,&binding.before)?;let after_set=load(m,&after)?;
    match &request.action {
        operator_model::Action::AddLibraryLocation{read_only,..}=>{
            require(after_set.locations.len()==before_set.locations.len()+1
                && after_set.locations[..before_set.locations.len()]==before_set.locations,
                "library_revision_add_changed_existing")?;
            let added=after_set.locations.last().ok_or("library_revision_add_missing")?;
            require(added.read_only==*read_only && added.id==crate::preparation::key(&(operation,
                &added.path,&added.identity,&added.drive,read_only))?,"library_revision_selection_changed")?;
            let anchor=directory.join("library-directory-link");
            let metadata=fs::symlink_metadata(&anchor)?;
            require(metadata.file_type().is_symlink() && metadata.uid()==unsafe{libc::getuid()}
                && metadata.ino()==added.link_inode && fs::read_link(anchor)?==added.path,
                "library_revision_link_owner_changed")?;
        }
        operator_model::Action::RemoveLibraryLocation{location,..}=>{
            require(before_set.locations.iter().filter(|l|&l.id==location).count()==1
                && after_set.locations==before_set.locations.into_iter().filter(|l|&l.id!=location).collect::<Vec<_>>(),
                "library_revision_remove_changed_existing")?;
        }
        _=>return Err("library_revision_request".into()),
    }
    Ok(binding)
}
pub fn revisions(m:&Manager)->Result<Vec<RevisionBinding>> {
    let root=m.root.join("operator");if !root.try_exists()? {return Ok(vec![])}
    let mut records=vec![];
    for (index,entry) in fs::read_dir(root)?.enumerate() {
        require(index<4096,"library_revision_history_bound")?;
        let entry=entry?;let name=entry.file_name().to_string_lossy().into_owned();
        if valid_hex(&name,32) && entry.path().join(REVISION_FILE).try_exists()? {records.push(revision(m,&name)?);}
    }
    Ok(records)
}

#[cfg(test)]
mod mount_identity_tests {
    use super::*;
    #[test]
    fn mount_source_uses_exact_id_not_anonymous_btrfs_device_or_path_prefix() {
        let table="21 1 0:31 /@ / rw - btrfs /dev/nvme0n1p2 rw,subvol=/@\n\
22 21 0:32 /@home /home rw - btrfs /dev/nvme0n1p2 rw,subvol=/@home\n\
23 21 8:17 / /run/media/Samples rw shared:7 - ext4 /dev/disk/by-label/My\\040Samples rw\n";
        assert_eq!(mount_source(table,22).unwrap(),PathBuf::from("/dev/nvme0n1p2"));
        assert_eq!(mount_source(table,23).unwrap(),PathBuf::from("/dev/disk/by-label/My Samples"));
        assert!(mount_source(table,99).is_err());
        assert!(mount_source(&format!("{table}{table}"),22).is_err());
        assert!(mount_source("42 1 0:42 / /mnt rw - tmpfs tmpfs rw",42).is_err());
        assert!(mount_source(r"42 1 8:17 / /mnt rw - ext4 /dev/bad\999 rw",42).is_err());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordinary_environment_bytes_are_unchanged_but_empty_set_keeps_capability() {
        let f=test_fixture::Fixture::new();let environment=f.r.environment.clone();
        let old=serde_json::to_value(&environment).unwrap();
        assert!(old.get("library_locations").is_none());
        let mut next=environment.clone();
        next.library_locations=Some(retain(&f.m,&LocationSet::default()).unwrap());
        assert_ne!(serde_json::to_vec(&environment).unwrap(),serde_json::to_vec(&next).unwrap());
        assert_eq!(load(&f.m,&next).unwrap(),LocationSet::default());
        let mut unsupported=old;unsupported["unknown_locations"]=serde_json::json!([]);
        assert!(serde_json::from_value::<Environment>(unsupported).is_err());
    }
    #[test]
    fn occupied_dangling_and_raw_drive_names_never_get_replaced() {
        let f=test_fixture::Fixture::new();let env=&f.r.environment;
        let root=dosdevices(env);private_dir(&root).unwrap();
        for name in ["c:","z:","s:","l:","m::","N:","O::"] {
            symlink("/a/nonexistent/foreign/drive",root.join(name)).unwrap();
        }
        assert_eq!(unused_drive(env,&LocationSet::default()).unwrap(),"P:");
        for name in ["c:","z:","s:","l:","m::","N:","O::"] {
            assert_eq!(fs::read_link(root.join(name)).unwrap(),Path::new("/a/nonexistent/foreign/drive"));
        }
        assert!(!valid_drive("S:") && !valid_drive("C:") && !valid_drive("Z:"));
    }
    #[test]
    fn internal_system_and_ancestor_locations_refuse() {
        let m=Manager::new(PathBuf::from("/home/owner/.local/share/bridge"),PathBuf::from("/home/owner/.vst3"));
        for path in ["/","/etc","/proc/1","/usr/share","/home/owner","/home/owner/.local/share/bridge/scratch",
            "/home/owner/.vst3","/mnt","/run/media"] {
            assert!(allowed_path(&m,Path::new(path)).is_err(),"{path}");
        }
        assert!(allowed_path(&m,Path::new("/mnt/Samples" )).is_ok());
    }
    #[test]
    fn missing_location_is_retained_with_letter_and_refuses_launch() {
        let f=test_fixture::Fixture::new();let mut env=f.r.environment.clone();
        let root=dosdevices(&env);private_dir(&root).unwrap();
        let missing=f.outer.join("removed-drive");symlink(&missing,root.join("l:")).unwrap();
        let location=Location{id:"ab".repeat(32),path:missing,drive:"L:".into(),read_only:true,
            identity:DirectoryIdentity{volume:"original-volume".into(),handle_type:1,handle:vec![1]},
            link_inode:fs::symlink_metadata(root.join("l:")).unwrap().ino()};
        let set=LocationSet{schema:1,locations:vec![location]};
        env.library_locations=Some(retain(&f.m,&set).unwrap());
        let error=launch_plan(&f.m,&env).unwrap_err().to_string();
        assert!(error.contains("removed-drive") && error.contains("L:") && error.contains("Reconnect"));
        assert_eq!(load(&f.m,&env).unwrap(),set);
        // A substituted empty directory is never treated as that original.
        fs::create_dir(&set.locations[0].path).unwrap();
        assert!(launch_plan(&f.m,&env).is_err());
        assert_eq!(load(&f.m,&env).unwrap(),set);
    }
    #[test]
    fn foreign_replacement_link_is_not_owned_even_with_equal_target() {
        let f=test_fixture::Fixture::new();let env=&f.r.environment;
        let root=dosdevices(env);private_dir(&root).unwrap();
        let target=f.outer.join("external");fs::create_dir(&target).unwrap();
        fs::write(target.join("user-sample"),b"preserved").unwrap();
        symlink(&target,root.join("l:")).unwrap();
        let location=Location{id:"ab".repeat(32),path:target.clone(),drive:"L:".into(),read_only:false,
            identity:DirectoryIdentity{volume:"fixture".into(),handle_type:1,handle:vec![1]},
            link_inode:fs::symlink_metadata(root.join("l:")).unwrap().ino()};
        verify_link(env,&location).unwrap();
        // Keep old inode alive so inode reuse cannot make this test ambiguous.
        fs::rename(root.join("l:"),root.join("retained-original")).unwrap();
        symlink(&target,root.join("l:")).unwrap();
        assert!(verify_link(env,&location).is_err());
        assert_eq!(fs::read(target.join("user-sample")).unwrap(),b"preserved");
    }
}

/// Reinterpret a retained inventory's setup binding only across exact location
/// edits. No installer, runtime or other mutation can borrow this evidence.
pub fn only_location_revisions(m:&Manager,before:&Environment,after:&Environment)->Result<bool> {
    if before==after {return Ok(true)}
    if before.id!=after.id || before.root!=after.root || before.runner!=after.runner
        || before.revision>=after.revision {return Ok(false)}
    let records=revisions(m)?;
    let mut current=before.clone();
    for _ in 0..records.len() {
        if current==*after {return Ok(true)}
        let mut next=records.iter().filter(|r|!r.cancelled && r.before==current);
        let Some(edge)=next.next() else {return Ok(false)};
        require(next.next().is_none(),"library_revision_ownership_conflict")?;
        current=edge.environment.clone();
    }
    Ok(current==*after)
}
