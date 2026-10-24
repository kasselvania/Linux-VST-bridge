use super::*;
use std::os::unix::fs::{symlink, PermissionsExt};
struct Fixture {
    path: PathBuf,
    root: PathBuf,
    storage: PathBuf,
    binding: Binding,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fn writable(path: &Path) {
            if std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir()) {
                let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
                if let Ok(children) = std::fs::read_dir(path) {
                    for child in children.flatten() {
                        writable(&child.path());
                    }
                }
            }
        }
        writable(&self.path);
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
fn fixture() -> Fixture {
    let path = std::env::temp_dir().join(format!(
        "bridge-preset-test-{}",
        crate::random_id().unwrap()
    ));
    std::fs::create_dir(&path).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = path.canonicalize().unwrap();
    let root = path.join("setup/user-presets");
    let storage = path.join("backups");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir(&storage).unwrap();
    let product = Product {
        environment: "ab".repeat(16),
        module_sha256: "cd".repeat(32),
        class_id: "ef".repeat(16),
    };
    let profile = Profile {
        id: "reference".into(),
        module_sha256: product.module_sha256.clone(),
        class_id: product.class_id.clone(),
        root: Root::Setup {
            relative: "user-presets".into(),
        },
        extensions: vec!["lvbpreset".into()],
        compatibility: "reference-v1".into(),
    };
    let binding = Binding::observe(product, &path.join("setup"), &profile).unwrap();
    Fixture {
        path,
        root,
        storage,
        binding,
    }
}
fn save(f: &Fixture, name: &str, bytes: &[u8]) {
    std::fs::write(f.root.join(name), bytes).unwrap();
}
#[test]
fn configured_root_selects_only_declared_key_and_revalidates_path() {
    let f = fixture();
    let setup = f.path.join("setup");
    let config = setup.join("prefs.json");
    let first = setup.join("drive_c/users/fixture/Documents/First/Presets/User");
    let second = setup.join("drive_c/users/fixture/Documents/Second/Presets/User");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    let write = |directory: &str| {
        std::fs::write(
            &config,
            serde_json::to_vec(&serde_json::json!({
                "User Presets Path":format!("C:\\users\\fixture\\Documents\\{directory}\\"),
                "unrelated-private-value": {"must-not-be-retained":"opaque fixture"}
            }))
            .unwrap(),
        )
        .unwrap()
    };
    write("First");
    let mut profile = f.binding.profile.clone();
    profile.root = Root::ConfiguredJson {
        file: "prefs.json".into(),
        key: "User Presets Path".into(),
        drive_c: "drive_c".into(),
        suffix: "Presets/User".into(),
    };
    let before = Binding::observe(f.binding.product.clone(), &setup, &profile).unwrap();
    assert_eq!(before.root, first);
    assert!(!serde_json::to_string(&before)
        .unwrap()
        .contains("opaque fixture"));
    let record = backup(&f.storage, &before, &"55".repeat(16)).unwrap();
    write("Second");
    assert!(inspect(&before).is_err());
    let after = Binding::observe(f.binding.product.clone(), &setup, &profile).unwrap();
    assert_ne!(before.token().unwrap(), after.token().unwrap());
    assert!(recovery(&after, &record).is_err());
    for value in [
        "Z:\\other",
        "C:\\users\\..\\other",
        "\\\\host\\share",
        "relative",
    ] {
        std::fs::write(
            &config,
            serde_json::to_vec(&serde_json::json!({"User Presets Path":value})).unwrap(),
        )
        .unwrap();
        assert!(Binding::observe(f.binding.product.clone(), &setup, &profile).is_err());
    }
    std::fs::write(
        &config,
        br#"{"User Presets Path":"C:\\first","User Presets Path":"C:\\second"}"#,
    )
    .unwrap();
    assert!(Binding::observe(f.binding.product.clone(), &setup, &profile).is_err());
    write("First");
    std::fs::remove_dir(&first).unwrap();
    symlink(&second, &first).unwrap();
    assert!(Binding::observe(f.binding.product.clone(), &setup, &profile).is_err());
}
#[test]
fn failed_final_revalidation_does_not_publish_partial_import() {
    let f = fixture();
    let root = f.binding.open().unwrap();
    let result = fs::write_new_checked(&root, "import.lvbpreset", b"complete bytes", || {
        Err::<(), _>("preset_selected_source_changed".into())
    });
    assert!(result.is_err());
    assert!(!f.root.join("import.lvbpreset").exists());
    assert!(fs::names(&root).unwrap().is_empty());
}
#[test]
fn linux_parent_creation_refuses_ancestor_links_before_writing() {
    let f = fixture();
    let home = f.path.join("home");
    let outside = f.path.join("outside");
    std::fs::create_dir(&home).unwrap();
    std::fs::create_dir(&outside).unwrap();
    symlink(&outside, home.join("Music")).unwrap();
    assert!(linux_parent(&home).is_err());
    assert!(!outside.join("Plug-in Presets").exists());
}
#[test]
fn import_rejects_changing_source_before_publication_and_replaced_parent() {
    let f = fixture();
    let path = f.path.join("selected.lvbpreset");
    std::fs::write(&path, b"before").unwrap();
    let selected = File::open(&path).unwrap();
    assert!(import_with(&f.binding, "import.lvbpreset", &selected, || {
        std::fs::write(&path, b"changed")?;
        Ok(())
    })
    .is_err());
    assert!(!f.root.join("import.lvbpreset").exists());
    let root = f.binding.open().unwrap();
    std::fs::create_dir(f.root.join("child")).unwrap();
    assert!(
        fs::write_new_checked(&root, "child/missing.lvbpreset", b"new", || {
            std::fs::rename(f.root.join("child"), f.root.join("moved"))?;
            std::fs::create_dir(f.root.join("child"))?;
            Ok(())
        })
        .is_err()
    );
    assert!(!f.root.join("moved/missing.lvbpreset").exists());
    assert!(!f.root.join("child/missing.lvbpreset").exists());
}
#[test]
fn fixture_import_load_update_remove_restore_rollback_reload() {
    let f = fixture();
    save(&f, "first.lvbpreset", b"reference gain=0.25");
    save(&f, "activation.dat", b"never copy");
    save(&f, "factory.db", b"never copy");
    let selected = f.path.join("selected.lvbpreset");
    std::fs::write(&selected, b"reference gain=0.75").unwrap();
    import(
        &f.binding,
        "second.lvbpreset",
        &File::open(&selected).unwrap(),
    )
    .unwrap();
    assert_eq!(std::fs::read(&selected).unwrap(), b"reference gain=0.75");
    let load = || std::fs::read(f.root.join("second.lvbpreset")).unwrap();
    assert_eq!(load(), b"reference gain=0.75");
    let before = backup(&f.storage, &f.binding, &"11".repeat(16)).unwrap();
    assert_eq!(before.files.len(), 2);
    let id = before.digest().unwrap();
    assert!(!f.storage.join(&id).join("files/activation.dat").exists());
    std::fs::remove_file(f.root.join("second.lvbpreset")).unwrap();
    let missing = recovery(&f.binding, &before).unwrap();
    assert_eq!(missing.missing.len(), 1);
    let after = restore_missing(&f.storage, &f.binding, &id, &missing).unwrap();
    assert_eq!(after.unchanged, 2);
    assert_eq!(load(), b"reference gain=0.75");
    let rollback = backup(&f.storage, &f.binding, &"22".repeat(16)).unwrap();
    std::fs::remove_file(f.root.join("second.lvbpreset")).unwrap();
    restore_missing(
        &f.storage,
        &f.binding,
        &rollback.digest().unwrap(),
        &recovery(&f.binding, &rollback).unwrap(),
    )
    .unwrap();
    assert_eq!(load(), b"reference gain=0.75");
}
#[test]
fn conflicts_and_stale_missing_offers_never_overwrite() {
    let f = fixture();
    save(&f, "a.lvbpreset", b"one");
    save(&f, "b.lvbpreset", b"two");
    let b = backup(&f.storage, &f.binding, &"33".repeat(16)).unwrap();
    let digest = b.digest().unwrap();
    save(&f, "a.lvbpreset", b"different");
    std::fs::remove_file(f.root.join("b.lvbpreset")).unwrap();
    let offer = recovery(&f.binding, &b).unwrap();
    assert_eq!(offer.conflicts, 1);
    assert_eq!(offer.missing.len(), 1);
    save(&f, "b.lvbpreset", b"new user file");
    assert!(restore_missing(&f.storage, &f.binding, &digest, &offer).is_err());
    assert_eq!(
        std::fs::read(f.root.join("a.lvbpreset")).unwrap(),
        b"different"
    );
    assert_eq!(
        std::fs::read(f.root.join("b.lvbpreset")).unwrap(),
        b"new user file"
    );
    assert!(import(
        &f.binding,
        "a.lvbpreset",
        &File::open(f.root.join("b.lvbpreset")).unwrap()
    )
    .is_err());
}
#[test]
fn links_root_replacement_and_hardlinks_refuse() {
    let f = fixture();
    save(&f, "a.lvbpreset", b"one");
    let external = f.path.join("external");
    std::fs::create_dir(&external).unwrap();
    symlink(&external, f.root.join("escape")).unwrap();
    assert!(inspect(&f.binding).is_err());
    std::fs::remove_file(f.root.join("escape")).unwrap();
    std::fs::hard_link(f.root.join("a.lvbpreset"), f.root.join("alias.lvbpreset")).unwrap();
    assert!(inspect(&f.binding).is_err());
    std::fs::remove_file(f.root.join("alias.lvbpreset")).unwrap();
    let old = f.path.join("old");
    std::fs::rename(&f.root, &old).unwrap();
    std::fs::create_dir(&f.root).unwrap();
    assert!(inspect(&f.binding).is_err());
}
#[test]
fn outward_link_preserves_vendor_folder_and_unrelated_linux_path() {
    let f = fixture();
    save(&f, "a.lvbpreset", b"one");
    let linked = link(&f.binding, &f.path, "Presets").unwrap();
    assert_eq!(std::fs::read_link(linked).unwrap(), f.root);
    assert!(!std::fs::symlink_metadata(&f.root)
        .unwrap()
        .file_type()
        .is_symlink());
    std::fs::create_dir(f.path.join("unrelated")).unwrap();
    assert!(link(&f.binding, &f.path, "unrelated").is_err());
    assert!(f.path.join("unrelated").is_dir());
}
#[test]
fn incomplete_and_changed_backups_never_restore() {
    let f = fixture();
    save(&f, "a.lvbpreset", b"one");
    let b = backup(&f.storage, &f.binding, &"44".repeat(16)).unwrap();
    let digest = b.digest().unwrap();
    let payload = f.storage.join(&digest).join("files/a.lvbpreset");
    std::fs::set_permissions(&payload, std::fs::Permissions::from_mode(0o600)).unwrap();
    std::fs::write(payload, b"bad").unwrap();
    assert!(verify(&f.storage, &digest).is_err());
    let incomplete = "55".repeat(32);
    std::fs::create_dir(f.storage.join(&incomplete)).unwrap();
    assert!(verify(&f.storage, &incomplete).is_err());
}
#[test]
fn unknown_catalog_never_qualifies_historical_vendor_paths() {
    assert!(profiles()
        .unwrap()
        .iter()
        .all(|profile| profile.module_sha256 != "aa".repeat(32)));
    let f = fixture();
    let mut other = f.binding.product.clone();
    other.module_sha256 = "aa".repeat(32);
    assert!(Binding::observe(other, &f.path.join("setup"), &f.binding.profile).is_err());
    for bad in ["../outside", "/absolute", "a/../../bad", "a\\b"] {
        assert!(fs::relative(bad).is_err());
    }
}
#[test]
fn external_known_folder_backup_is_independent_of_setup_snapshot() {
    let f = fixture();
    let external = f.path.join("Documents");
    std::fs::create_dir(&external).unwrap();
    std::fs::create_dir(external.join("ReferenceUser")).unwrap();
    symlink(&external, f.path.join("setup/Documents")).unwrap();
    let mut profile = f.binding.profile.clone();
    profile.root = Root::KnownFolder {
        link: "Documents".into(),
        suffix: "ReferenceUser".into(),
    };
    let bound =
        Binding::observe(f.binding.product.clone(), &f.path.join("setup"), &profile).unwrap();
    assert!(bound.external_to_setup);
    std::fs::write(bound.root.join("user.lvbpreset"), b"user").unwrap();
    let backup = backup(&f.storage, &bound, &"66".repeat(16)).unwrap();
    std::fs::remove_file(bound.root.join("user.lvbpreset")).unwrap();
    restore_missing(
        &f.storage,
        &bound,
        &backup.digest().unwrap(),
        &recovery(&bound, &backup).unwrap(),
    )
    .unwrap();
    assert_eq!(
        std::fs::read(bound.root.join("user.lvbpreset")).unwrap(),
        b"user"
    );
    let replacement = f.path.join("OtherDocuments");
    std::fs::create_dir_all(replacement.join("ReferenceUser")).unwrap();
    std::fs::remove_file(f.path.join("setup/Documents")).unwrap();
    symlink(&replacement, f.path.join("setup/Documents")).unwrap();
    assert!(
        inspect(&bound).is_err(),
        "a known-folder redirect invalidates custody"
    );
    assert!(
        verify(&f.storage, &backup.digest().unwrap()).is_ok(),
        "completed external backup survives root redirection"
    );
}
#[test]
fn same_bytes_replacement_changes_source_identity_but_is_not_a_restore_conflict() {
    let f = fixture();
    save(&f, "a.lvbpreset", b"same");
    let before = inspect(&f.binding).unwrap();
    let b = backup(&f.storage, &f.binding, &"77".repeat(16)).unwrap();
    std::fs::rename(f.root.join("a.lvbpreset"), f.path.join("old.lvbpreset")).unwrap();
    save(&f, "a.lvbpreset", b"same");
    assert_ne!(inspect(&f.binding).unwrap(), before);
    assert_eq!(recovery(&f.binding, &b).unwrap().unchanged, 1);
}
#[test]
fn writable_selected_descriptors_and_nonpreset_formats_are_refused() {
    let f = fixture();
    save(&f, "a.lvbpreset", b"data");
    let writable = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(f.root.join("a.lvbpreset"))
        .unwrap();
    assert!(import(&f.binding, "b.lvbpreset", &writable).is_err());
    assert!(import(
        &f.binding,
        "serial.lic",
        &File::open(f.root.join("a.lvbpreset")).unwrap()
    )
    .is_err());
    assert!(!f.root.join("b.lvbpreset").exists());
}
#[test]
fn replacement_during_backup_and_interruption_leave_no_completed_backup() {
    for interrupted in [false, true] {
        let f = fixture();
        save(&f, "a.lvbpreset", b"same");
        let result = backup_with(&f.storage, &f.binding, &"88".repeat(16), || {
            if interrupted {
                return Err("fixture interruption".into());
            }
            std::fs::rename(f.root.join("a.lvbpreset"), f.path.join("old.lvbpreset"))?;
            save(&f, "a.lvbpreset", b"same");
            Ok(())
        });
        assert!(result.is_err());
        for entry in std::fs::read_dir(&f.storage).unwrap() {
            let path = entry.unwrap().path();
            assert!(!path.join("manifest.json").exists());
        }
        assert_eq!(std::fs::read(f.root.join("a.lvbpreset")).unwrap(), b"same");
    }
}
#[test]
fn restore_does_not_follow_a_new_directory_link() {
    let f = fixture();
    std::fs::create_dir(f.root.join("nested")).unwrap();
    save(&f, "nested/a.lvbpreset", b"a");
    let b = backup(&f.storage, &f.binding, &"99".repeat(16)).unwrap();
    std::fs::remove_file(f.root.join("nested/a.lvbpreset")).unwrap();
    let offer = recovery(&f.binding, &b).unwrap();
    std::fs::remove_dir(f.root.join("nested")).unwrap();
    let unrelated = f.path.join("unrelated");
    std::fs::create_dir(&unrelated).unwrap();
    symlink(&unrelated, f.root.join("nested")).unwrap();
    assert!(restore_missing(&f.storage, &f.binding, &b.digest().unwrap(), &offer).is_err());
    assert!(std::fs::read_dir(unrelated).unwrap().next().is_none());
}
