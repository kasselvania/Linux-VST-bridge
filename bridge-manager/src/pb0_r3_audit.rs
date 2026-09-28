//! PB0-R3 source probe. This is absent from ordinary release builds.
//! It reads the fixed installed root and a bounded package manifest on stdin;
//! it has no mutation, caller path, process or executable selection.
use super::*;

const ENVIRONMENT: &str = "7d8fce354e68595cdc20485f754a892d";
const BEAM: &str = "ABCDEF019182FAEB4C756E6170726F43";
const BEAM_FILTER: &str = "ABCDEF019182FAEB4C756E61666C744C";

fn bounded_entries(path: &Path, limit: usize, refusal: &str) -> Result<Vec<fs::DirEntry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        require(entries.len() < limit, refusal)?;
        entries.push(entry?);
    }
    Ok(entries)
}

pub(super) fn run(m: &Manager) -> Result<()> {
    let mut input = Vec::new();
    std::io::stdin().take(64 * 1024 + 1).read_to_end(&mut input)?;
    require(!input.is_empty() && input.len() <= 64 * 1024,
        "pb0_r3_manifest_extent")?;
    let retained = onboarding::retained_environment(m, ENVIRONMENT)?
        .ok_or("pb0_r3_bg1_environment_absent")?;
    let registry = m.registry()?;
    for class in [BEAM, BEAM_FILTER] {
        let entry = registry.classes.get(class).ok_or("pb0_r3_bg1_class_absent")?;
        require(entry.registration.environment == retained.environment,
            "pb0_r3_bg1_environment_changed")?;
    }
    experimental_runner::verify_selected_bg1_history(m, &retained.environment)?;
    let command_component = retained.environment.runner.files.iter()
        .find(|artifact| artifact.path ==
            retained.environment.runner.proton.with_file_name("native-command-session.json"))
        .ok_or("pb0_r3_bg1_component_absent")?;
    let candidates = preparation::retained_candidates(m)?;
    let mut observations = 0usize;
    let mut reviews = 0usize;
    let mut guided_checks = 0usize;
    let mut guided_results = 0usize;
    let mut selections = std::collections::BTreeSet::new();
    for candidate in &candidates {
        // Historical candidates may predate a selected environment transition.
        // Their immutable identity and evidence remain readable without
        // pretending their old runtime is the current executable authority.
        selections.insert(candidate.selection.id()?);
        observations += preparation::observations(m, candidate)?.len();
        reviews += preparation::decisions(m, candidate)?.len();
        guided_results += preparation::guided_results(m, &candidate.id()?)?.len();
    }
    for selection in selections {
        guided_checks += preparation::guided_checks(m, &selection)?.len();
    }
    let mut imported_installers = 0usize;
    for entry in bounded_entries(&m.root.join("installers"), 128, "pb0_r3_installer_bound")? {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(id) = name.strip_suffix(".json") else { continue };
        if !valid_hex(id, 64) { continue; }
        installer_import::load_record(m, id)?;
        imported_installers += 1;
    }
    let mut historical_requests = 0usize;
    let mut private_schema_11_requests = 0usize;
    let operator = m.root.join("operator");
    for entry in bounded_entries(&operator, 2048, "pb0_r3_operation_bound")? {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !valid_hex(&name, 32) { continue; }
        require(entry.file_type()?.is_dir(), "pb0_r3_operation_type")?;
        let path = entry.path().join("request.json");
        if !path.try_exists()? { continue; }
        require(file(&path)?.metadata()?.len() <= 64 * 1024,
            "pb0_r3_request_extent")?;
        let request: operator_model::Request = read_json(&path)?;
        // Older wire generations are historical evidence only. The ordinary
        // recovery owner keeps its narrower admission for executable replay.
        require(matches!(request.schema, 1..=11), "pb0_r3_historical_request_schema")?;
        historical_requests += 1;
        private_schema_11_requests += usize::from(request.schema == 11);
    }
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME absent")?);
    let package = package_authority::audit_predecessor_from_bytes(m, &home, &input)?;
    println!("{}", serde_json::to_string(&serde_json::json!({
        "schema": 1,
        "bg1_environment": ENVIRONMENT,
        "bg1_runner": retained.environment.runner.id,
        "bg1_command_component_sha256": command_component.sha256,
        "bg1_history_verified": true,
        "selected_classes": [BEAM, BEAM_FILTER],
        "retained_candidates_decoded": candidates.len(),
        "retained_observations": observations,
        "retained_reviews": reviews,
        "retained_guided_checks": guided_checks,
        "retained_guided_results": guided_results,
        "imported_installers_decoded": imported_installers,
        "historical_operator_requests": historical_requests,
        "historical_private_schema_11_requests": private_schema_11_requests,
        "package_predecessor": package,
        "writes": false,
    }))?);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unrelated_entries_count_toward_the_bound() {
        let f = test_fixture::Fixture::new();
        let directory = f.outer.join("bounded-audit-entries");
        fs::create_dir(&directory).unwrap();
        for n in 0..3 { fs::write(directory.join(format!("unrelated-{n}")), b"").unwrap(); }
        assert_eq!(bounded_entries(&directory, 3, "overflow").unwrap().len(), 3);
        assert_eq!(bounded_entries(&directory, 2, "overflow").unwrap_err().to_string(), "overflow");
    }
}
