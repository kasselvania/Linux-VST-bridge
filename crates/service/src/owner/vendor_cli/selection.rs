//! A normal app choice from the existing owned installer's observed registration.
//! Never enumerate a prefix or infer the main image from a filename.
use super::*;
use serde_json::Value;

/// Observe one explicitly chosen Windows executable; names and installer history
/// do not confer permission. Keep its adjacent resources in their original place.
pub(crate) fn program_artifact(path: &Path) -> Result<Artifact> {
    use std::io::{Read, Seek, SeekFrom};
    let path = path.canonicalize()?;
    require(path.to_str().is_some_and(|p|p.len()<=4096
        && !p.chars().any(char::is_control) && !p.contains('"')),
        "The selected program needs a Windows-compatible path")?;
    let mut source = file(&path)?;
    let metadata = source.metadata()?;
    require(metadata.len() >= 64 && metadata.len() <= 256 * 1024 * 1024,
        "Choose a Windows executable smaller than 256 MiB")?;
    let mut header = [0_u8;64]; source.read_exact(&mut header)?;
    require(&header[..2] == b"MZ", "This file is not a Windows executable")?;
    let offset = u32::from_le_bytes(header[60..64].try_into()?) as u64;
    require(offset.checked_add(24).is_some_and(|end|end <= metadata.len()),
        "The Windows executable has an invalid PE header")?;
    source.seek(SeekFrom::Start(offset))?;
    let mut pe = [0_u8;24]; source.read_exact(&mut pe)?;
    let machine = u16::from_le_bytes(pe[4..6].try_into()?);
    let flags = u16::from_le_bytes(pe[22..24].try_into()?);
    require(&pe[..4] == b"PE\0\0" && flags & 2 != 0 && flags & 0x2000 == 0,
        "Choose a Windows program, rather than a DLL")?;
    require(matches!(machine,0x14c | 0x8664),
        "The selected runtime needs a Windows x86 or x64 executable")?;
    Ok(Artifact {sha256:digest(&path)?,path})
}

pub(crate) fn choose_program(m: &Manager, environment: &str, expected_environment: &str,
    executable: &Artifact, previous: Option<&str>) -> Result<Application> {
    let retired = operator_cli::vendor_maintenance::observe_retired(m, &operator_cli::environment_install::target(m, environment)?)?;
    choose_program_with(m,environment,expected_environment,executable,previous,
        ||retired.verify(m).map(|()|true))
}
fn choose_program_with(m: &Manager, environment: &str, expected_environment: &str,
    executable: &Artifact, previous: Option<&str>, retired: impl FnOnce()->Result<bool>) -> Result<Application> {
    let guard = m.lock("registry.lock")?;
    capacity::require_environment_inactive(m, &operator_cli::environment_install::target(m, environment)?)?;
    require(retired()?,
        "Close the running program before changing this selection")?;
    let environment = operator_cli::environment_install::target(m,environment)?;
    require(operator_cli::environment_runtime::identity(&environment)? == expected_environment,
        "The setup changed; choose the program again")?;
    let selected = program_artifact(&executable.path)?;
    require(selected == *executable, "The program changed again; review its current version")?;
    let current = superseded(m)?;
    let previous = if previous.is_none() {
        let mut existing = selectors(m)?.into_iter().filter(|s|!current.contains(s))
            .map(|s|application(m,&s).map(|a|(s,a))).collect::<Result<Vec<_>>>()?
            .into_iter().filter(|(_,app)|app.user_selection.is_some()
                && app.environment.id == environment.id && app.executable.path == selected.path);
        let retained = existing.next();
        require(existing.next().is_none(), "program_selection_conflict")?;
        if let Some((_,app)) = retained.as_ref() {
            if app.executable == selected {return Ok(app.clone());}
        }
        retained.map(|(s,_)|s)
    } else { previous.map(str::to_owned) };
    let selection = if let Some(previous) = previous.as_deref() {
        let old = application(m,previous)?;
        require(old.user_selection.is_some() && old.environment == environment
            && old.executable.path == selected.path && old.executable.sha256 != selected.sha256,
            "program_adoption_binding")?;
        require(!current.contains(previous), "This program selection has already changed")?;
        let mut options = old.user_selection.ok_or("program_selection_missing")?;
        options.previous = Some(previous.into()); options
    } else {
        vendor_application::ProgramSelection {working_directory:selected.path.parent()
            .ok_or("program_working_directory")?.into(),arguments:vec![],previous:None}
    };
    let app = Application {schema:3,id:ApplicationId::Selected,environment,
        executable:selected,helpers:vec![],installer_sha256:None,installer_result:None,
        observed_installer_version:String::new(),user_selection:Some(selection)};
    register_application(m,&app,&guard)?;
    Ok(app)
}

pub(crate) fn authority(m: &Manager) -> Result<Vec<Value>> {
    selectors(m)?.into_iter().filter_map(|selector| match application(m,&selector) {
        Ok(app) if app.user_selection.is_some() => Some(Ok(json!({"application":app,
            "file":observation::ModuleStamp::read(&app.executable.path).ok()}))),
        Ok(_) => None, Err(error) => Some(Err(error)),
    }).collect()
}

pub(crate) fn superseded(m: &Manager) -> Result<std::collections::BTreeSet<String>> {
    let mut previous = std::collections::BTreeSet::new();
    for selector in selectors(m)? {
        let app = application(m,&selector)?;
        if let Some(parent) = app.user_selection.and_then(|s|s.previous) {
            let old = application(m,&parent)?;
            require(old.user_selection.is_some() && old.environment.id == app.environment.id
                && old.executable.path == app.executable.path, "program_adoption_history")?;
            previous.insert(parent);
        }
    }
    Ok(previous)
}

#[derive(Clone)]
pub(crate) struct Choice {
    pub application: Application,
    pub name: String,
    pub image_name: String,
    pub missing_capability: Option<String>,
}

fn relative_image(value: &str) -> Result<PathBuf> {
    let path = PathBuf::from(value);
    require(!value.is_empty() && !value.contains('\\')
        && value.split('/').all(|part| !part.is_empty() && !matches!(part,"." | "..")
            && !part.contains(':') && !part.chars().any(char::is_control))
        && !path.is_absolute() && path.components().all(|c|
        matches!(c, std::path::Component::Normal(_)))
        && path.extension().is_some_and(|s| s.eq_ignore_ascii_case("exe")),
        "vendor_application_observed_image_path")?;
    Ok(path)
}

// These are roles observed in this installer's own snapshot, not filename or
// vendor-name guesses. Keep the files as helpers; only their automatic Open
// choices are excluded. Explicit user program selection remains independent.
fn non_app_images(transaction: &Value) -> std::collections::BTreeSet<String> {
    let mut images = std::collections::BTreeSet::new();
    for (kind, field) in [("uninstall", "DerivedUninstaller"), ("services", "image")] {
        let Some(rows) = transaction["after"][kind].as_object() else { continue };
        for row in rows.values() {
            let Some(value) = row[field].as_str() else { continue };
            let normalized = value.replace('\\', "/");
            let relative = if normalized.get(..3).is_some_and(|drive| drive.eq_ignore_ascii_case("c:/")) {
                &normalized[3..]
            } else { normalized.as_str() };
            if relative_image(relative).is_ok() { images.insert(relative.to_lowercase()); }
        }
    }
    images
}

/// The app choices this setup's installation observation offers, read once
/// for each composition: an observation can be several megabytes.
pub(crate) fn choices(m: &Manager, record: &onboarding::Record) -> Result<Vec<Choice>> {
    m.derive_once(&format!("vendor_choices:{}", serde_json::to_string(record)?), || observed_choices(m, record))
}
fn observed_choices(m: &Manager, record: &onboarding::Record) -> Result<Vec<Choice>> {
    let (operation,installer,receipt,result)=if let Some(replacement)=operator_cli::environment_install::latest(m,&record.id)? {
        let receipt=Artifact {path:operator_cli::job_dir(m,&replacement.operation)?.join("installation-result.json"),
            sha256:digest(&operator_cli::job_dir(m,&replacement.operation)?.join("installation-result.json"))?};
        let result=operator_cli::environment_install::result(m,&replacement)?;
        (replacement.operation,replacement.installer,receipt,result)
    } else {
        let Some(operation)=&record.installation_operation else {return Ok(vec![])};
        let path=onboarding::directory(m,&record.id)?.join(format!("{operation}-result.json"));
        (operation.clone(),record.installer.clone(),Artifact {sha256:digest(&path)?,path},onboarding::result(m,record)?)
    };
    if result["state"] != "completed" || result["raw_exit"] != 0
        || !onboarding::retired(&result) { return Ok(vec![]); }
    let directory = receipt.path.parent().ok_or("vendor_application_installation_binding")?;
    let path = directory.join(format!("{operation}-transaction-private.json"));
    if !path.exists() { return Ok(vec![]); }
    require(file(&path)?.metadata()?.len() <= 16 * 1024 * 1024,
        "vendor_application_observation_bound")?;
    let transaction: Value = read_json(&path)?;
    require(transaction["schema"] == 1 && transaction["operation"] == *operation,
        "vendor_application_observation_binding")?;
    let Some(registrations) = transaction["durable_installation"]["application_registrations"].as_array()
        else { return Ok(vec![]) };
    require(registrations.len() <= 64, "vendor_application_registration_bound")?;
    let non_apps = non_app_images(&transaction);
    let mut out = vec![];
    for registration in registrations {
        let key = registration["registration"].as_str().ok_or("vendor_application_registration_binding")?;
        require(valid_hex(key, 64), "vendor_application_registration_binding")?;
        require(["added", "changed"].into_iter().any(|kind|
            transaction["durable_installation"]["delta"]["uninstall"][kind].as_array()
                .is_some_and(|rows|rows.iter().any(|v|v == key))),
            "vendor_application_registration_observation")?;
        let observed = &transaction["after"]["uninstall"][key];
        let name = observed["DisplayName"].as_str().ok_or("vendor_application_name_missing")?;
        require(!name.trim().is_empty() && name.len() <= 1024 && !name.chars().any(char::is_control),
            "vendor_application_name_bound")?;
        // Installers that omit InstallLocation are located from their icon or
        // uninstaller folder, recorded separately by the supervisor.
        let location = observed["InstallLocation"].as_str().filter(|v| !v.is_empty())
            .or_else(|| observed["DerivedInstallLocation"].as_str())
            .ok_or("vendor_application_location_missing")?.replace('\\', "/");
        let relative = location.strip_prefix("C:/").or_else(|| location.strip_prefix("c:/"))
            .ok_or("vendor_application_location_binding")?;
        let root = PathBuf::from(relative.trim_end_matches('/'));
        require(!root.as_os_str().is_empty()
            && relative.trim_end_matches('/').split('/').all(|part| !part.is_empty()
                && !matches!(part,"." | "..") && !part.contains(':') && !part.chars().any(char::is_control))
            && !root.is_absolute() && root.components().all(|c|
            matches!(c, std::path::Component::Normal(_))), "vendor_application_location_binding")?;
        let images = registration["images"].as_array().ok_or("vendor_application_images_missing")?;
        require(images.len() <= 64, "vendor_application_image_bound")?;
        let mut artifacts = vec![];
        for image in images {
            let selected = image.as_str().ok_or("vendor_application_image_binding")?;
            let path = PathBuf::from(selected);
            if !path.extension().is_some_and(|s| s.eq_ignore_ascii_case("exe")) { continue; }
            let path = relative_image(selected)?;
            let mut components = path.components();
            require(root.components().all(|expected|components.next().is_some_and(|actual|
                actual.as_os_str().to_string_lossy().to_lowercase()
                    == expected.as_os_str().to_string_lossy().to_lowercase())),
                "vendor_application_image_location")?;
            let changed = ["added", "changed"].into_iter().any(|kind|
                transaction["durable_installation"]["delta"]["files"][kind].as_array()
                    .is_some_and(|rows| rows.iter().any(|v| v == selected)));
            let row = &transaction["after"]["files"][selected];
            require(changed && row["format"] == "pe_executable",
                "vendor_application_image_observation")?;
            let machine = row["pe"]["machine"].as_u64().ok_or("vendor_application_machine_missing")?;
            let characteristics = row["pe"]["characteristics"].as_u64()
                .ok_or("vendor_application_image_type_missing")?;
            if characteristics & 0x2000 != 0 || characteristics & 2 == 0 { continue; }
            let sha256 = row["sha256"].as_str().filter(|s|valid_hex(s,64))
                .ok_or("vendor_application_image_digest_missing")?;
            let artifact = Artifact { path: record.environment.root.join("compatdata/pfx/drive_c").join(path),
                sha256:sha256.into() };
            require(!artifacts.iter().any(|(a, _)|a == &artifact), "vendor_application_image_duplicate")?;
            artifacts.push((artifact, machine));
        }
        require(artifacts.len() <= 17, "vendor_application_selection_bound")?;
        // The registration's icon names the application itself; its siblings
        // (uninstallers, service setups, elevation helpers) stay helpers.
        let application = registration["application_image"].as_str()
            .map(|image| relative_image(image).map(|path|
                record.environment.root.join("compatdata/pfx/drive_c").join(path)))
            .transpose()?
            .filter(|path| artifacts.iter().any(|(a, _)| &a.path == path));
        for (selected, machine) in &artifacts {
            let relative = selected.path.strip_prefix(record.environment.root.join("compatdata/pfx/drive_c"))?;
            if non_apps.contains(&relative.to_string_lossy().to_lowercase()) { continue; }
            if application.as_ref().is_some_and(|path| &selected.path != path) { continue; }
            let image_name = selected.path.file_stem().ok_or("vendor_application_image_name")?
                .to_str().ok_or("vendor_application_image_name")?;
            require(!image_name.is_empty() && image_name.len() <= 256
                && !image_name.chars().any(char::is_control), "vendor_application_image_name_bound")?;
            out.push(Choice { name:name.into(), image_name:image_name.into(),
                missing_capability:if matches!(machine, 0x14c | 0x8664) {None} else {
                    Some(format!("The compatibility runtime cannot open this app's Windows processor architecture (PE machine {machine:#06x})."))
                },
                application:Application { schema:2, id:ApplicationId::Selected,
                    environment:record.environment.clone(), executable:selected.clone(),
                    helpers:artifacts.iter().filter(|(a, _)|a != selected).map(|(a, _)|a.clone()).collect(),
                    installer_sha256:Some(installer.clone()),installer_result:Some(receipt.clone()),
                    observed_installer_version:observed["DisplayVersion"].as_str().unwrap_or("").into(),
                    user_selection:None } });
        }
    }
    Ok(out)
}

pub(crate) fn observed(m: &Manager, selector: &str) -> Result<Option<Choice>> {
    for record in onboarding::history_records(m)? {
        // A malformed observation belongs to that setup. It cannot revoke
        // another exact installation's positively observed app choice.
        let Ok(choices) = choices(m, &record) else { continue; };
        for choice in choices {
            if choice.application.selector()? == selector { return Ok(Some(choice)); }
        }
    }
    Ok(None)
}

pub(crate) fn select(m: &Manager, selector: &str) -> Result<Application> {
    if directory(m, selector)?.join("application.json").exists() { return application(m, selector); }
    let choice = observed(m, selector)?.ok_or("vendor_application_choice_unavailable")?;
    if let Some(reason) = &choice.missing_capability { return Err(reason.clone().into()); }
    // Execution rechecks the exact installed content and its real original witness.
    choice.application.verify(&m.root)?;
    let retirement = operator_cli::vendor_maintenance::observe_retired(m, &choice.application.environment)?;
    let guard = m.lock("registry.lock")?;
    capacity::require_environment_inactive(m, &choice.application.environment)?;
    require(!operator_cli::vendor_maintenance::blocks(m, &choice.application.environment)?,
        "setup_maintenance_active")?;
    retirement.verify(m)?;
    // Regenerate the exact receipt-bound choice under registration admission.
    let current = observed(m, selector)?.ok_or("vendor_application_choice_unavailable")?;
    require(current.application == choice.application, "vendor_application_choice_changed")?;
    register_application(m, &choice.application, &guard)?;
    let selected = application(m, selector)?;
    require(selected == choice.application, "vendor_application_choice_changed")?;
    Ok(selected)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn dsp_lease(f: &test_fixture::Fixture, environment: Option<&Environment>) -> PathBuf {
        let session = random_id().unwrap();
        let lease = f.m.root.join("runtime/leases").join(format!("{session}.json"));
        private_dir(lease.parent().unwrap()).unwrap();
        let report = f.m.root.join("runtime/results").join(format!("windows-{session}.json"));
        let root = environment.unwrap_or(&f.r.environment).root.clone();
        let owner = root.join("compatdata/pfx/drive_c/bridge/sessions").join(&session).join("owner.json");
        private_dir(owner.parent().unwrap()).unwrap();
        let mut registration = json!({"metadata":{"class_id":"01".repeat(16)}});
        if let Some(environment) = environment { registration["environment"] = json!(environment); }
        atomic_json(&owner, &json!({"session":session,"report":report,"lease":lease,
            "keeper":false,"inspect":false,"vendor_access":false,"registration":registration})).unwrap();
        atomic_json(&lease, &report).unwrap();
        lease
    }

    #[test]
    fn first_observed_registration_is_scoped_to_its_setup() {
        for owner in ["other", "same", "unknown", "corrupt_target"] {
            let (f, record, _) = installed_app();
            let choice = choices(&f.m, &record).unwrap().remove(0);
            let selector = choice.application.selector().unwrap();
            assert!(!directory(&f.m, &selector).unwrap().join("application.json").exists());
            let mut other = record.environment.clone();
            other.id = random_id().unwrap();other.root = f.m.root.join("environments").join(&other.id);
            let mut units = vec![onboarding::unit(record.installation_operation.as_ref().unwrap()).unwrap()];
            let lease = if owner == "corrupt_target" {
                let sibling = choices(&f.m, &record).unwrap().remove(1).application;
                let guard = f.m.lock("registry.lock").unwrap();
                register_application(&f.m, &sibling, &guard).unwrap();drop(guard);
                let sibling_selector = sibling.selector().unwrap();
                units.push(unit(&sibling_selector).unwrap());
                fs::write(directory(&f.m, &sibling_selector).unwrap().join("operation.json"), b"invalid receipt").unwrap();
                None
            } else { Some(dsp_lease(&f, match owner {"other"=>Some(&other),"same"=>Some(&record.environment),_=>None})) };
            let result = readiness::with_unit_state_fixture(&units, "inactive", || select(&f.m, &selector));
            if owner == "other" {
                assert_eq!(result.unwrap(), choice.application);
            } else {
                assert!(result.is_err(), "{owner} owner must refuse first registration");
                assert!(!directory(&f.m, &selector).unwrap().join("application.json").exists());
            }
            if let Some(lease) = lease { assert!(lease.exists(), "registration must preserve the running instance"); }
        }
    }

    pub(crate) fn installed_app() -> (test_fixture::Fixture, onboarding::Record, PathBuf) {
        let mut f = test_fixture::Fixture::new();
        let old = f.r.environment.root.clone();
        let id = random_id().unwrap();
        let root = f.m.root.join("environments").join(&id);
        fs::rename(&old, &root).unwrap();
        f.r.module.path = root.join(f.r.module.path.strip_prefix(&old).unwrap());
        f.r.environment.id = id.clone();
        f.r.environment.root = root.clone();
        atomic_json(&root.join("environment.json"), &f.r.environment).unwrap();
        let programs = root.join("compatdata/pfx/drive_c/Program Files/Vendor");
        private_dir(&programs).unwrap();
        let mut pe = vec![0_u8; 1024];
        pe[..2].copy_from_slice(b"MZ");
        pe[60..64].copy_from_slice(&128_u32.to_le_bytes());
        pe[128..132].copy_from_slice(b"PE\0\0");
        pe[132..134].copy_from_slice(&0x8664_u16.to_le_bytes());
        pe[148]=2;pe[150]=0x22;
        pe[152..154].copy_from_slice(&0x20b_u16.to_le_bytes());
        fs::write(programs.join("Main.exe"), &pe).unwrap();
        pe[1023] = 1;
        fs::write(programs.join("Worker.exe"), &pe).unwrap();
        let input=f.outer.join("bootstrap.exe");fs::write(&input,&pe).unwrap();
        let installer=installer_import::import(&f.m,file(&input).unwrap()).unwrap().id;
        let operation = random_id().unwrap();
        let record = onboarding::Record { schema:1, id, installer,
            environment:f.r.environment.clone(), created_at:1, creation_operation:operation.clone(),
            installation_operation:Some(operation.clone()), published:false, automatic:None, previous_attempt:None };
        let directory = onboarding::directory(&f.m, &record.id).unwrap();
        private_dir(&directory).unwrap();
        atomic_json(&directory.join("record.json"), &record).unwrap();
        atomic_json(&directory.join(format!("{operation}-result.json")), &json!({"schema":2,
            "operation":operation,"state":"completed","raw_exit":0,"owned_live":0,
            "cleanup_confirmed":true,"error":null})).unwrap();
        let mut files = serde_json::Map::new();
        for name in ["Main.exe", "Worker.exe"] {
            files.insert(format!("Program Files/Vendor/{name}"), json!({"format":"pe_executable",
                "sha256":digest(&programs.join(name)).unwrap(),"pe":{"machine":0x8664,"characteristics":0x22}}));
        }
        let key = "cd".repeat(32);
        let images = files.keys().cloned().collect::<Vec<_>>();
        let transaction = directory.join(format!("{operation}-transaction-private.json"));
        atomic_json(&transaction, &json!({"schema":1,"operation":operation,
            "after":{"files":files,"uninstall":{key.clone():{"DisplayName":"Vendor App",
                "DisplayVersion":"1.0","InstallLocation":"C:\\PROGRAM FILES\\VENDOR"}}},
            "durable_installation":{"application_registrations":[{"registration":key,"images":images}],
                "delta":{"files":{"added":images,"changed":[]},"uninstall":{"added":[key],"changed":[]}},
                "incomplete":["unrelated_observation_bound"]}})).unwrap();
        (f, record, transaction)
    }

    #[test]
    fn exact_observed_choices_use_windows_case_and_leave_history_unchanged() {
        let (f, record, _) = installed_app();
        let before = test_fixture::snapshot(&f.m.root);
        let choices = choices(&f.m, &record).unwrap();
        assert_eq!(choices.len(), 2);
        for choice in choices {
            assert_eq!(choice.name, "Vendor App");
            assert!(choice.missing_capability.is_none());
            assert_eq!(choice.application.helpers.len(), 1);
            choice.application.verify(&f.m.root).unwrap();
        }
        assert_eq!(test_fixture::snapshot(&f.m.root), before);
    }

    #[test]
    fn exact_uninstaller_and_service_witnesses_filter_open_choices_without_name_guesses() {
        let (f, record, path) = installed_app();
        let mut transaction: Value = read_json(&path).unwrap();
        let programs = record.environment.root.join("compatdata/pfx/drive_c/Program Files/Vendor");
        for (index, name) in ["RemoveSaved.exe", "BackgroundHost.exe", "unins000.exe"].into_iter().enumerate() {
            let mut bytes = fs::read(programs.join("Main.exe")).unwrap();
            bytes[1023] = (index + 2) as u8;
            let image = programs.join(name); fs::write(&image, bytes).unwrap();
            let relative = format!("Program Files/Vendor/{name}");
            transaction["after"]["files"][&relative] = json!({"format":"pe_executable",
                "sha256":digest(&image).unwrap(),"pe":{"machine":0x8664,"characteristics":0x22}});
            transaction["durable_installation"]["application_registrations"][0]["images"]
                .as_array_mut().unwrap().push(relative.clone().into());
            transaction["durable_installation"]["delta"]["files"]["added"]
                .as_array_mut().unwrap().push(relative.into());
        }
        transaction["after"]["uninstall"]["cd".repeat(32)]["DerivedUninstaller"] =
            "PROGRAM FILES/VENDOR/RemoveSaved.exe".into();
        transaction["after"]["services"] = json!({"ef".repeat(32):{
            "image":"C:\\Program Files\\Vendor\\BackgroundHost.exe"}});
        atomic_json(&path, &transaction).unwrap();
        let before = test_fixture::snapshot(&f.m.root);
        let choices = choices(&f.m, &record).unwrap();
        let names:std::collections::BTreeSet<_> = choices.iter().map(|c|c.image_name.as_str()).collect();
        assert_eq!(names, ["Main", "Worker", "unins000"].into_iter().collect());
        for choice in &choices {
            assert_eq!(choice.application.helpers.len(), 4, "non-app files remain witnessed helpers");
            choice.application.verify(&f.m.root).unwrap();
        }
        assert_eq!(test_fixture::snapshot(&f.m.root), before);
        // Even an icon naming the known service does not turn it into an app
        // or silently substitute another executable.
        transaction["durable_installation"]["application_registrations"][0]["application_image"] =
            "Program Files/Vendor/BackgroundHost.exe".into();
        atomic_json(&path, &transaction).unwrap();
        assert!(super::choices(&f.m, &record).unwrap().is_empty());
        // Filtering suggestions does not revoke the user's deliberate choice.
        let selected = program_artifact(&programs.join("RemoveSaved.exe")).unwrap();
        let explicit = choose_program_with(&f.m, &record.environment.id,
            &operator_cli::environment_runtime::identity(&record.environment).unwrap(),
            &selected, None, ||Ok(true)).unwrap();
        assert_eq!(explicit.executable, selected);
        assert!(explicit.user_selection.is_some());
    }

    #[test]
    fn derived_location_finds_an_app_whose_installer_omits_install_location() {
        // Native Access 3.26 registers only DisplayIcon and UninstallString.
        let (f, record, path) = installed_app();
        let mut observed: Value = read_json(&path).unwrap();
        let entry = &mut observed["after"]["uninstall"]["cd".repeat(32)];
        entry.as_object_mut().unwrap().remove("InstallLocation");
        atomic_json(&path, &observed).unwrap();
        assert!(choices(&f.m, &record).err().unwrap().to_string().contains("vendor_application_location_missing"));
        observed["after"]["uninstall"]["cd".repeat(32)]["DerivedInstallLocation"] = "C:\\Program Files\\Vendor".into();
        atomic_json(&path, &observed).unwrap();
        let choices = choices(&f.m, &record).unwrap();
        assert_eq!(choices.len(), 2);
        for choice in &choices { choice.application.verify(&f.m.root).unwrap(); }
        observed["after"]["uninstall"]["cd".repeat(32)]["InstallLocation"] = "".into();
        atomic_json(&path, &observed).unwrap();
        assert_eq!(super::choices(&f.m, &record).unwrap().len(), 2, "an empty InstallLocation falls back");
        // The icon-named image is the one application; its sibling is a helper.
        observed["durable_installation"]["application_registrations"][0]["application_image"] =
            "Program Files/Vendor/Main.exe".into();
        atomic_json(&path, &observed).unwrap();
        let named = super::choices(&f.m, &record).unwrap();
        assert_eq!(named.len(), 1);
        assert_eq!(named[0].image_name, "Main");
        assert_eq!(named[0].application.helpers.len(), 1);
        observed["durable_installation"]["application_registrations"][0]["application_image"] =
            "Program Files/Vendor/Absent.exe".into();
        atomic_json(&path, &observed).unwrap();
        assert_eq!(super::choices(&f.m, &record).unwrap().len(), 2, "an unobserved icon image changes nothing");
        observed["after"]["uninstall"]["cd".repeat(32)]["DerivedInstallLocation"] = "C:/Program Files/VendorSibling".into();
        atomic_json(&path, &observed).unwrap();
        assert!(super::choices(&f.m, &record).is_err());
    }

    pub(crate) fn structured_application(seed: u8) -> (test_fixture::Fixture, Application, PathBuf) {
        let (f, record, transaction) = installed_app();
        let directory = record.environment.root.join("compatdata/pfx/drive_c/Program Files/Vendor");
        let old = "Program Files/Vendor/Main.exe";
        let new = "Program Files/Vendor/Native Access.exe";
        fs::rename(directory.join("Main.exe"), directory.join("Native Access.exe")).unwrap();
        let mut bytes = fs::read(directory.join("Native Access.exe")).unwrap();
        bytes[1023] = seed;
        fs::write(directory.join("Native Access.exe"), &bytes).unwrap();
        let mut observation: Value = read_json(&transaction).unwrap();
        let files = observation["after"]["files"].as_object_mut().unwrap();
        let mut image = files.remove(old).unwrap();
        image["sha256"] = json!(digest(&directory.join("Native Access.exe")).unwrap());
        files.insert(new.into(), image);
        for pointer in ["/durable_installation/application_registrations/0/images",
            "/durable_installation/delta/files/added"] {
            for path in observation.pointer_mut(pointer).unwrap().as_array_mut().unwrap() {
                if *path == old { *path = json!(new); }
            }
        }
        atomic_json(&transaction, &observation).unwrap();
        let profile = linux_vst_bridge::vendor_profile::native_access().unwrap();
        for name in &profile.resources {
            let path = directory.join(name);
            private_dir(path.parent().unwrap()).unwrap();
            fs::write(path, [seed, 42, 13]).unwrap();
        }
        let bundled = directory.join(&profile.dependency.installer_directory);
        private_dir(&bundled).unwrap();
        fs::write(bundled.join(format!("NTKDaemon {seed}.0 Setup PC.exe")), &bytes).unwrap();
        let app = choices(&f.m, &record).unwrap().into_iter().find(|choice|
            choice.application.executable.path.ends_with("Native Access.exe")).unwrap().application;
        (f, app, directory)
    }

    #[test]
    fn assisted_app_capabilities_bind_actual_installations_and_unknown_version_bytes() {
        let (first, a, _) = structured_application(1);
        let (second, b, _) = structured_application(99);
        assert_ne!(a.environment.id, b.environment.id);
        assert_ne!(a.installer_result, b.installer_result);
        let before = test_fixture::snapshot(&first.m.root);
        let left = linux_vst_bridge::vendor_profile::inspect(&a, &first.m.root).unwrap().capabilities.unwrap();
        let right = linux_vst_bridge::vendor_profile::inspect(&b, &second.m.root).unwrap().capabilities.unwrap();
        assert_ne!(left.application, right.application);
        assert_ne!(left.resources, right.resources);
        assert_ne!(left.dependency.installer, right.dependency.installer);
        left.verify(&a, &first.m.root).unwrap();
        right.verify(&b, &second.m.root).unwrap();
        assert!(left.verify(&b, &second.m.root).is_err());
        assert_eq!(before, test_fixture::snapshot(&first.m.root));
    }

    #[test]
    fn assisted_app_structure_refusal_preserves_generic_choice_and_exact_identity() {
        for failure in ["missing", "ambiguous", "symlink", "architecture", "missing_and_symlink", "missing_and_ambiguous"] {
            let (f, app, directory) = structured_application(2);
            match failure {
                "missing" => fs::remove_file(directory.join("resources/app.asar")).unwrap(),
                "ambiguous" | "missing_and_ambiguous" => {
                    if failure=="missing_and_ambiguous" {fs::remove_file(directory.join("chrome_100_percent.pak")).unwrap();}
                    let bundled = directory.join("resources/daemon/win");
                    fs::copy(bundled.join("NTKDaemon 2.0 Setup PC.exe"),
                        bundled.join("NTKDaemon 3.0 Setup PC.exe")).unwrap();
                },
                "symlink" | "missing_and_symlink" => {
                    if failure=="missing_and_symlink" {fs::remove_file(directory.join("chrome_100_percent.pak")).unwrap();}
                    let path = directory.join("resources/app.asar");
                    fs::remove_file(&path).unwrap();
                    std::os::unix::fs::symlink(directory.join("resources.pak"), path).unwrap();
                },
                "architecture" => {
                    let path = directory.join("resources/daemon/win/NTKDaemon 2.0 Setup PC.exe");
                    let mut bytes = fs::read(&path).unwrap();
                    bytes[132..134].copy_from_slice(&0xaa64_u16.to_le_bytes());
                    fs::write(path, bytes).unwrap();
                },
                _ => unreachable!(),
            }
            let inspected = linux_vst_bridge::vendor_profile::inspect(&app, &f.m.root);
            if matches!(failure,"ambiguous"|"symlink"|"missing_and_symlink"|"missing_and_ambiguous") {
                assert!(inspected.is_err(), "{failure}");
            } else {
                let inspected=inspected.unwrap();
                assert!(inspected.capabilities.is_none(), "{failure}");
                assert!(inspected.missing_capability.is_some(), "{failure}");
            }
            app.verify(&f.m.root).unwrap();
            assert!(select(&f.m, &app.selector().unwrap()).is_ok());
            let chosen=choose_program_with(&f.m,&app.environment.id,
                &operator_cli::environment_runtime::identity(&app.environment).unwrap(),&app.executable,None,||Ok(true)).unwrap();
            let generic=linux_vst_bridge::vendor_profile::inspect(&chosen,&f.m.root).unwrap();
            assert!(generic.capabilities.is_none());
            assert!(generic.missing_capability.is_some());
        }
    }
    #[test]
    fn unreadable_candidate_structure_refuses_automatic_fallback_but_keeps_explicit_choice() {
        use std::os::unix::fs::PermissionsExt;
        assert_ne!(unsafe {libc::geteuid()},0,"Run ownership fixtures as an ordinary user");
        let (f,app,directory)=structured_application(2);
        let bundled=directory.join("resources/daemon/win");
        fs::copy(bundled.join("NTKDaemon 2.0 Setup PC.exe"),bundled.join("NTKDaemon 3.0 Setup PC.exe")).unwrap();
        fs::set_permissions(&bundled,fs::Permissions::from_mode(0o300)).unwrap();
        let inspected=linux_vst_bridge::vendor_profile::inspect(&app,&f.m.root);
        fs::set_permissions(&bundled,fs::Permissions::from_mode(0o700)).unwrap();
        assert!(inspected.err().unwrap().to_string().contains("support files could not be read"));
        let chosen=choose_program_with(&f.m,&app.environment.id,
            &operator_cli::environment_runtime::identity(&app.environment).unwrap(),&app.executable,None,||Ok(true)).unwrap();
        assert!(linux_vst_bridge::vendor_profile::inspect(&chosen,&f.m.root).unwrap().capabilities.is_none());
    }

    #[test]
    fn unsafe_registration_or_image_binding_refuses_before_registration() {
        let (f, record, path) = installed_app();
        let original: Value = read_json(&path).unwrap();
        for case in ["empty_root", "outside", "not_changed", "wrong_operation", "label_control"] {
            let mut changed = original.clone();
            match case {
                "empty_root" => changed["after"]["uninstall"]["cd".repeat(32)]["InstallLocation"] = "C:/".into(),
                "outside" => changed["after"]["uninstall"]["cd".repeat(32)]["InstallLocation"] = "C:/Program Files/VendorSibling".into(),
                "not_changed" => changed["durable_installation"]["delta"]["files"]["added"] = json!([]),
                "wrong_operation" => changed["operation"] = random_id().unwrap().into(),
                "label_control" => changed["after"]["uninstall"]["cd".repeat(32)]["DisplayName"] = "Vendor\nApp".into(),
                _ => unreachable!(),
            }
            atomic_json(&path, &changed).unwrap();
            assert!(choices(&f.m, &record).is_err(), "{case}");
            assert!(!f.m.root.join("vendor-applications").exists());
        }
        for unsafe_path in ["../Main.exe", "/tmp/Main.exe", "Main.dll", "Program Files/./Main.exe", "Program Files//Main.exe"] {
            assert!(relative_image(unsafe_path).is_err());
        }
    }

    #[test]
    fn unsupported_main_image_stays_visible_with_a_specific_capability() {
        let (f, record, path) = installed_app();
        let mut transaction: Value = read_json(&path).unwrap();
        transaction["after"]["files"]["Program Files/Vendor/Main.exe"]["pe"]["machine"] = json!(0xaa64);
        atomic_json(&path, &transaction).unwrap();
        let choices = choices(&f.m, &record).unwrap();
        assert_eq!(choices.len(), 2);
        let unsupported = choices.iter().find(|c|c.image_name == "Main").unwrap();
        assert!(unsupported.missing_capability.as_deref().unwrap().contains("0xaa64"));
        assert!(select(&f.m, &unsupported.application.selector().unwrap()).is_err());
        assert!(choices.iter().find(|c|c.image_name == "Worker").unwrap().missing_capability.is_none());
    }

    #[test]
    fn selected_content_change_or_incomplete_installation_cannot_execute() {
        let (f, record, _) = installed_app();
        let selected = choices(&f.m, &record).unwrap().remove(0);
        fs::write(&selected.application.executable.path, b"different").unwrap();
        assert!(selected.application.verify(&f.m.root).is_err());
        let result = onboarding::directory(&f.m, &record.id).unwrap()
            .join(format!("{}-result.json",record.installation_operation.as_ref().unwrap()));
        let mut failed: Value = read_json(&result).unwrap();
        failed["owned_live"] = json!(1);
        atomic_json(&result, &failed).unwrap();
        assert!(choices(&f.m, &record).unwrap().is_empty());
    }

    #[test]
    fn portable_selection_and_explicit_adoption_preserve_setup_and_exact_history() {
        let (f, record, _) = installed_app();
        f.m.register(f.r.clone()).unwrap();
        fs::remove_dir_all(onboarding::directory(&f.m,&record.id).unwrap()).unwrap();
        fs::remove_dir_all(f.m.root.join("installers")).unwrap();
        let folder = f.outer.join("Portable program");private_dir(&folder).unwrap();
        let path = folder.join("Any name.exe");
        fs::copy(record.environment.root.join("compatdata/pfx/drive_c/Program Files/Vendor/Main.exe"),&path).unwrap();
        fs::write(folder.join("resource.txt"),b"retained adjacent resource").unwrap();
        let selected = program_artifact(&path).unwrap();
        let identity = operator_cli::environment_runtime::identity(&record.environment).unwrap();
        let protected = test_fixture::snapshot(&f.m.root);
        let first = choose_program_with(&f.m,&record.id,&identity,&selected,None,||Ok(true)).unwrap();
        let old_selector = first.selector().unwrap();
        assert!(first.installer_result.is_none() && first.installer_sha256.is_none());
        assert_eq!(first.environment,record.environment);
        assert_eq!(first.user_selection.as_ref().unwrap().working_directory,folder);
        first.verify(&f.m.root).unwrap();
        let old_record = fs::read(directory(&f.m,&old_selector).unwrap().join("application.json")).unwrap();
        let mut changed = fs::read(&path).unwrap();changed[1023]=9;fs::write(&path,&changed).unwrap();
        assert!(first.verify(&f.m.root).is_err());
        let second_bytes = program_artifact(&path).unwrap();
        changed[1023]=10;fs::write(&path,&changed).unwrap();
        assert!(choose_program_with(&f.m,&record.id,&identity,&second_bytes,Some(&old_selector),||Ok(true)).is_err(),
            "The displayed replacement cannot authorize a later third build");
        fs::write(&path,{let mut b=changed.clone();b[1023]=9;b}).unwrap();
        assert!(choose_program_with(&f.m,&record.id,&identity,&second_bytes,Some(&old_selector),||Ok(false)).is_err());
        let second = choose_program_with(&f.m,&record.id,&identity,&second_bytes,Some(&old_selector),||Ok(true)).unwrap();
        let successor = second.selector().unwrap();assert_ne!(successor,old_selector);
        assert_eq!(superseded(&f.m).unwrap(),std::collections::BTreeSet::from([old_selector.clone()]));
        let reopened = application(&f.m,&successor).unwrap();reopened.verify(&f.m.root).unwrap();
        assert_eq!(reopened,second,"A fresh manager read remembers the adopted exact bytes");
        assert_eq!(fs::read(directory(&f.m,&old_selector).unwrap().join("application.json")).unwrap(),old_record);
        assert_eq!(application(&f.m,&old_selector).unwrap(),first);
        assert!(first.verify(&f.m.root).is_err(),"History cannot claim vanished older bytes remain runnable");
        for (path, bytes) in protected {
            assert_eq!(test_fixture::snapshot(&f.m.root).get(&path),Some(&bytes),"{path:?}");
        }
        assert_eq!(fs::read(folder.join("resource.txt")).unwrap(),b"retained adjacent resource");
    }

    #[test]
    fn chooser_refuses_stale_setup_or_selected_bytes_without_a_new_record() {
        let (f, record, _) = installed_app();
        drop(f.m.lock("registry.lock").unwrap());
        let image = program_artifact(&record.environment.root.join("compatdata/pfx/drive_c/Program Files/Vendor/Main.exe")).unwrap();
        let before = test_fixture::snapshot(&f.m.root);
        assert!(choose_program_with(&f.m,&record.id,&"00".repeat(32),&image,None,||Ok(true)).is_err());
        let mut changed = image;changed.sha256="ab".repeat(32);
        let identity = operator_cli::environment_runtime::identity(&record.environment).unwrap();
        assert!(choose_program_with(&f.m,&record.id,&identity,&changed,None,||Ok(true)).is_err());
        assert_eq!(test_fixture::snapshot(&f.m.root),before);
    }
}
