//! Declared plug-in service dependencies, selected only when registered in this setup.
use super::*;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependency { service: String, name: String }
fn registered(hive: &str, service: &str) -> bool {
    hive.lines().filter_map(|line| line.strip_prefix('[')?.split_once(']').map(|(key,_)| key))
        .any(|key| {
            let parts: Vec<_> = key.split("\\\\").collect();
            parts.len()==4 && parts[0].eq_ignore_ascii_case("System")
                && (parts[1].eq_ignore_ascii_case("CurrentControlSet")
                    || parts[1].strip_prefix("ControlSet").is_some_and(|suffix|
                        suffix.len()==3 && suffix.bytes().all(|c|c.is_ascii_digit())))
                && parts[2].eq_ignore_ascii_case("Services") && parts[3].eq_ignore_ascii_case(service)
        })
}
pub(super) fn selected(environment: &Environment) -> Result<Vec<String>> {
    let path=environment.root.join("compatdata/pfx/system.reg");
    if !path.try_exists()? {return Ok(vec![])}
    require(file(&path)?.metadata()?.len()<=16*1024*1024,"plugin_service_registry_bound")?;
    let hive=fs::read_to_string(path)?;
    let dependencies:Vec<Dependency>=serde_json::from_str(include_str!("plugin_services.json"))?;
    Ok(dependencies.into_iter().filter(|d| !d.name.is_empty() && registered(&hive,&d.service))
        .map(|d|d.service).collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_exact_registered_declared_service_selects_dependency() {
        for key in [r"System\\CurrentControlSet\\Services\\PaceLicenseDServices",
            r"System\\ControlSet001\\Services\\PaceLicenseDServices"] {
            assert!(registered(&format!("[{key}] 1\n"),"PaceLicenseDServices"));
        }
        for key in [r"Software\\PaceLicenseDServices",r"System\\ControlSet001\\Services\\Bonjour",
            r"System\\ControlSet001\\Services\\PaceLicenseDServices\\Parameters"] {
            assert!(!registered(&format!("[{key}] 1\n"),"PaceLicenseDServices"));
        }
    }
}
