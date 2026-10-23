//! Read-only language and routing over the canonical operator snapshot, and
//! the window's shared theme. Plug-in and offer wording lives in the screen model.
use crate::model::{BridgeBuild, SetupPackage, Product, Snapshot};
use crate::screen_model::{AttemptKey, Destination, ProductKey};
use crate::screen_model::product::{daw_name, last_load_text, product_activity, product_issue, terminal_text};
#[cfg(test)]
use crate::screen_model::product::{activity_certainty, ActivityCertainty};
use eframe::egui;

/// Shared readable, touch-sized presentation for the ordinary app and previews.
pub fn configure_style(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();
    style.spacing.interact_size.y = 44.0;
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    for (kind, size) in [(egui::TextStyle::Heading, 23.0), (egui::TextStyle::Body, 15.0),
        (egui::TextStyle::Button, 14.0), (egui::TextStyle::Small, 13.0)] {
        style.text_styles.insert(kind, egui::FontId::proportional(size));
    }
    ctx.set_global_style(style);
    let mut visuals = ctx.global_style().visuals.clone();
    if visuals.dark_mode {
        visuals.override_text_color = Some(egui::Color32::from_rgb(224, 227, 232));
        visuals.panel_fill = egui::Color32::from_rgb(24, 27, 32);
        visuals.faint_bg_color = egui::Color32::from_rgb(32, 37, 44);
        visuals.widgets.noninteractive.bg_stroke.color = egui::Color32::from_rgb(71, 79, 91);
    } else {
        visuals.override_text_color = Some(egui::Color32::from_rgb(34, 42, 54));
        visuals.panel_fill = egui::Color32::from_rgb(248, 250, 253);
        visuals.faint_bg_color = egui::Color32::from_rgb(236, 241, 248);
    }
    ctx.set_visuals(visuals);
}

pub fn primary_button(label: impl Into<egui::WidgetText>, ui: &egui::Ui) -> egui::Button<'static> {
    egui::Button::new(label).fill(ui.visuals().selection.bg_fill)
        .min_size(egui::vec2(0.0, 44.0))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attention {
    pub title: String,
    pub detail: String,
    pub destination: Destination,
}

pub fn attentions(snapshot: &Snapshot) -> Vec<Attention> {
    let mut items = Vec::new();
    let mut seen_sessions = std::collections::HashSet::new();
    let system = &snapshot.system;
    if !system.capacity_available() {
        items.push(Attention {
            title: "Bridge readback unavailable".into(),
            detail: "Active plug-ins and safe admission cannot be confirmed. Refresh the manager."
                .into(),
            destination: Destination::Diagnostics,
        });
    }
    if system.capacity_available() && system.cleanup_unconfirmed {
        items.push(Attention {
            title: "Cleanup is unconfirmed".into(),
            detail: "The manager is blocking unsafe new work until ownership is resolved.".into(),
            destination: Destination::Diagnostics,
        });
    }
    if system.pending_transactions > 0 {
        items.push(Attention {
            title: "Publication transaction needs review".into(),
            detail: format!(
                "{} pending transaction(s). Open Setup for the manager's reconcile control.",
                system.pending_transactions
            ),
            destination: Destination::Setup,
        });
    }
    if system.stale_transports > 0 && system.capacity_available() {
        items.push(Attention {
            title: "Old bridge transport remains".into(),
            detail: format!(
                "{} stale transport(s) reported by the manager.",
                system.stale_transports
            ),
            destination: Destination::Diagnostics,
        });
    }
    for (index, row) in snapshot.active_sessions.iter().enumerate() {
        let Some(class_id) = row["class_id"].as_str().filter(|id| !id.is_empty()) else {
            continue;
        };
        let Some(terminal) = row["terminal"].as_str() else {
            continue;
        };
        if snapshot
            .products
            .iter()
            .filter(|product| product.class_id == class_id)
            .count()
            < 2
        {
            continue;
        }
        let session = row["session"].as_str().filter(|id| !id.is_empty());
        if !seen_sessions.insert(session.map_or_else(|| format!("legacy-{index}"), str::to_owned)) {
            continue;
        }
        items.push(Attention {
            title: "Plug-in session needs attention".into(),
            detail: format!("{}. Several installed builds share this class; the exact build is unavailable in the session record.", terminal_text(terminal)),
            destination: session.map_or(Destination::Activity, |id| Destination::Session(id.into())),
        });
    }
    for product in &snapshot.products {
        let activity = product_activity(snapshot, product);
        let problematic = matches!(
            product.disposition.as_str(),
            "needs_attention" | "quarantined" | "another_configuration"
        );
        let detail = if problematic && activity.shared_class {
            Some(
                product_issue(product)
                    .unwrap_or_else(|| "Review this plug-in's manager state.".into()),
            )
        } else if !activity.failed_live.is_empty() {
            (!activity.shared_class).then(|| format!("Live session: {}.", activity.failed_live[0]))
        } else if activity.recent_cleanup_unconfirmed {
            (!activity.shared_class)
                .then(|| "A recent failed session has unconfirmed cleanup.".into())
        } else if let Some(refused) = (!problematic).then(|| last_load_text(product)).flatten() {
            Some(refused)
        } else if problematic {
            Some(
                product_issue(product)
                    .unwrap_or_else(|| "Review this plug-in's manager state.".into()),
            )
        } else {
            activity
                .recent_failure
                .filter(|_| !activity.shared_class)
                .map(|failure| format!("Most recent session: {failure}."))
        };
        if let Some(detail) = detail {
            items.push(Attention {
                title: product.name.clone(),
                detail,
                destination: Destination::Product(ProductKey::from(product)),
            });
        }
    }
    for attempt in &snapshot.onboarding {
        if matches!(
            attempt.state.as_str(),
            "needs_attention" | "cleanup_unconfirmed" | "failed"
        ) {
            items.push(Attention {
                title: "Installation needs attention".into(),
                detail: attempt.required_human_action.clone(),
                destination: Destination::Attempt(AttemptKey {
                    installer: attempt.installer.clone(),
                    environment: attempt.environment.clone(),
                }),
            });
        }
    }
    items
}

pub fn session_product<'a>(snapshot: &'a Snapshot, class_id: &str) -> Option<&'a Product> {
    let matching: Vec<_> = snapshot
        .products
        .iter()
        .filter(|product| !class_id.is_empty() && product.class_id == class_id)
        .collect();
    (matching.len() == 1)
        .then(|| matching.into_iter().next())
        .flatten()
}

pub fn session_display_name<'a>(snapshot: &'a Snapshot, class_id: &str) -> &'a str {
    let mut matches = snapshot
        .products
        .iter()
        .filter(|product| !class_id.is_empty() && product.class_id == class_id);
    match (matches.next(), matches.next()) {
        (Some(product), None) => &product.name,
        (Some(_), Some(_)) => "Shared plug-in class · exact build unavailable",
        _ => "Plug-in class unavailable",
    }
}

/// "Installed: Linux VST Bridge 0.12.0test144", from the installed generation record.
pub fn installed_line(build: &BridgeBuild) -> String {
    match &build.installed {
        Some(version) => format!("Installed: Linux VST Bridge {version}"),
        None => "Installed: Linux VST Bridge (version unknown)".into(),
    }
}

/// Title-row note, only for a package the musician can apply.
pub fn setup_title_note(build: &BridgeBuild) -> Option<String> {
    match &build.setup {
        SetupPackage::Newer(version) => Some(format!("Update ready: {version}")),
        SetupPackage::DifferentBuild(_) => Some("Different build ready in Setup".into()),
        _ => None,
    }
}

/// What Setup holds beside the installed bridge, for Diagnostics and reports.
pub fn setup_line(build: &BridgeBuild) -> Option<String> {
    match &build.setup {
        SetupPackage::Newer(version) => Some(format!("Update ready in Setup: {version}")),
        SetupPackage::DifferentBuild(version) =>
            Some(format!("A different build of {version} is ready in Setup")),
        SetupPackage::Older(version) => Some(format!("Setup holds an older version ({version})")),
        SetupPackage::Unchecked => Some("Setup's package could not be checked".into()),
        SetupPackage::Nothing => None,
    }
}

/// This window's own build, from its generation record, read once. It names
/// a version before the manager answers.
pub fn own_version() -> Option<&'static str> {
    static VERSION: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    VERSION.get_or_init(|| std::env::current_exe().ok()?.parent()
        .and_then(linux_vst_bridge::package_version::generation_version)).as_deref()
}

/// A product-delivered runtime's revision ("r5"). Any other runner id may name
/// a person or machine, so it is never shown.
fn runtime_name(id: &str) -> String {
    match id.strip_prefix("managed-").and_then(|rest| rest.rsplit_once("-r")) {
        Some((_, n)) if !n.is_empty() && n.len() <= 4 && n.bytes().all(|b| b.is_ascii_digit()) =>
            format!("r{n}"),
        _ => "custom runtime".into(),
    }
}

/// Compatibility runtimes the plug-in setups use, else the ones installed.
pub fn runtime_summary(snapshot: &Snapshot) -> Option<String> {
    let mut names: Vec<_> = snapshot.environments.iter().map(|e| runtime_name(&e.runner)).collect();
    if names.is_empty() {
        names = snapshot.managed_runtime_records.iter()
            .filter(|record| record["failure"].is_null())
            .filter_map(|record| record["id"].as_str().map(runtime_name)).collect();
    }
    names.sort();
    names.dedup();
    (!names.is_empty()).then(|| names.join(", "))
}

/// The distribution's own name from os-release, without user or path detail.
pub fn os_release_name(text: &str) -> Option<String> {
    let value = |key: &str| text.lines().find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .map(|v| v.trim().trim_matches(['"', '\'']).to_owned())
        .filter(|v| !v.is_empty() && v.len() <= 80 && !v.chars().any(char::is_control));
    value("PRETTY_NAME").or_else(|| Some(match (value("NAME")?, value("VERSION_ID")) {
        (name, Some(version)) => format!("{name} {version}"),
        (name, None) => name,
    }))
}

/// This computer's distribution, read once.
pub fn distribution() -> Option<&'static str> {
    static NAME: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    NAME.get_or_init(|| os_release_name(&linux_vst_bridge::read_os_release())).as_deref()
}

/// DAWs chosen for installations, when any was chosen.
fn chosen_daws(snapshot: &Snapshot) -> Option<String> {
    let mut daws: Vec<_> = snapshot.installer_setups.iter()
        .filter_map(|setup| setup.journey.as_ref().map(|journey| daw_name(journey.daw))).collect();
    daws.sort();
    daws.dedup();
    (!daws.is_empty()).then(|| daws.join(", "))
}

/// Plain text for a bug report. Versions and names only: no account, licence,
/// token or path.
pub fn version_details(snapshot: &Snapshot, distribution: Option<&str>) -> String {
    let mut lines = vec![installed_line(&snapshot.bridge)];
    lines.extend(setup_line(&snapshot.bridge));
    lines.push(format!("Compatibility runtime: {}",
        runtime_summary(snapshot).as_deref().unwrap_or("not installed")));
    lines.push(format!("Distribution: {}", distribution.unwrap_or("unknown")));
    if let Some(daws) = chosen_daws(snapshot) {
        lines.push(format!("DAW chosen in Setup: {daws}"));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Snapshot {
        serde_json::from_str(include_str!("../examples/library-preview.json")).unwrap()
    }

    #[test]
    fn version_details_name_build_runtime_and_distribution_only() {
        let mut s = snapshot();
        s.bridge = BridgeBuild { installed: Some("0.12.0test144".into()), setup: SetupPackage::Nothing };
        for environment in &mut s.environments {
            environment.runner = "managed-ge-proton11-7-slr4-20260805-r5".into();
        }
        assert_eq!(setup_title_note(&s.bridge), None);
        assert_eq!(version_details(&s, Some("CachyOS Linux")), "Installed: Linux VST Bridge 0.12.0test144\n\
            Compatibility runtime: r5\nDistribution: CachyOS Linux");
        s.bridge.setup = SetupPackage::Newer("0.12.0test145".into());
        s.environments[0].runner = "managed-ge-proton11-7-slr4-20260805-r4".into();
        assert_eq!(setup_title_note(&s.bridge).as_deref(), Some("Update ready: 0.12.0test145"));
        assert_eq!(version_details(&s, None), "Installed: Linux VST Bridge 0.12.0test144\n\
            Update ready in Setup: 0.12.0test145\nCompatibility runtime: r4, r5\nDistribution: unknown");
        // Only a newer or different build is offered in the title row.
        for (setup, line) in [
            (SetupPackage::DifferentBuild("0.12.0test144".into()),
                "A different build of 0.12.0test144 is ready in Setup"),
            (SetupPackage::Older("0.12.0test140".into()), "Setup holds an older version (0.12.0test140)"),
            (SetupPackage::Unchecked, "Setup's package could not be checked"),
        ] {
            s.bridge.setup = setup.clone();
            assert_eq!(setup_line(&s.bridge).as_deref(), Some(line));
            assert_eq!(setup_title_note(&s.bridge).is_some(),
                matches!(setup, SetupPackage::DifferentBuild(_)));
            assert!(version_details(&s, None).contains(line));
        }
        // Without plug-in setups, the installed runtime is the one named.
        s.bridge = BridgeBuild::default();
        s.environments.clear();
        s.managed_runtime_records = vec![
            serde_json::json!({"id":"managed-ge-proton11-7-slr4-20260805-r3","failure":"changed"}),
            serde_json::json!({"id":"managed-ge-proton11-7-slr4-20260805-r5","failure":null})];
        assert_eq!(installed_line(&s.bridge), "Installed: Linux VST Bridge (version unknown)");
        assert_eq!(setup_line(&s.bridge), None);
        assert_eq!(runtime_summary(&s).as_deref(), Some("r5"));
        s.managed_runtime_records.clear();
        assert!(version_details(&s, None).contains("Compatibility runtime: not installed"));
    }

    #[test]
    fn only_a_delivered_runtime_is_named_and_any_other_runner_stays_private() {
        assert_eq!(runtime_name("managed-ge-proton11-7-slr4-20260805-r5"), "r5");
        for private in ["pk-deck-wine", "/home/someone/.steam/proton", "proton-9.0-r2",
            "managed-ge-proton-rx", "managed-pk-deck-r"] {
            assert_eq!(runtime_name(private), "custom runtime", "{private}");
        }
        let mut s = snapshot();
        s.environments[0].runner = "pk-deck-wine".into();
        s.environments[1].runner = "managed-ge-proton11-7-slr4-20260805-r5".into();
        s.environments.truncate(2);
        let details = version_details(&s, None);
        assert!(details.contains("Compatibility runtime: custom runtime, r5"));
        assert!(!details.contains("pk") && !details.contains("deck"));
    }

    #[test]
    fn os_release_name_prefers_the_pretty_name() {
        assert_eq!(os_release_name("NAME=\"CachyOS Linux\"\nPRETTY_NAME=\"CachyOS\"\nID=cachyos\n").as_deref(),
            Some("CachyOS"));
        assert_eq!(os_release_name("NAME=\"SteamOS\"\nVERSION_ID=3.8\n").as_deref(), Some("SteamOS 3.8"));
        assert_eq!(os_release_name("ID=unknown\n"), None);
    }

    #[test]
    fn refused_load_says_how_long_ago_and_reaches_the_attention_list() {
        // The elapsed-time wording itself is tested with the screen model.
        let mut snapshot = snapshot();
        let index = snapshot.products.iter()
            .position(|p| p.availability.as_ref().is_some_and(|a| a.available)
                && !matches!(p.disposition.as_str(), "needs_attention" | "quarantined" | "another_configuration"))
            .expect("an available plug-in in the preview");
        assert_eq!(last_load_text(&snapshot.products[index]), None);
        let name = snapshot.products[index].name.clone();
        snapshot.products[index].details["last_load"] = serde_json::json!({"outcome": "refused",
            "observed_at": 1_000,
            "problem": {"code": "windows_side_stopped", "what_happened": "It stayed empty.",
                "likely_cause": "Its Windows side stopped.", "what_to_try": "Load it again."}});
        let text = last_load_text(&snapshot.products[index]).unwrap();
        assert!(text.starts_with("Load refused ") && text.ends_with("It stayed empty. Its Windows side stopped. Load it again."));
        assert!(attentions(&snapshot).iter().any(|item| item.title == name && item.detail == text));
        // A refused audio start (#403) says so rather than a refused load.
        snapshot.products[index].details["last_load"]["outcome"] = "activation_refused".into();
        let text = last_load_text(&snapshot.products[index]).unwrap();
        assert!(text.starts_with("Audio start refused ") && text.ends_with("Load it again."));
    }

    #[test]
    fn retained_active_row_never_becomes_a_current_product_claim_without_certain_capacity() {
        let mut snapshot = snapshot();
        snapshot.active_sessions = vec![serde_json::json!({
            "session":"exact-session", "class_id":"fixture-fragments", "state":"active", "recent":false
        })];
        let product = &snapshot.products[1];
        let confirmed = product_activity(&snapshot, product);
        assert_eq!(confirmed.certainty, ActivityCertainty::Confirmed);
        assert_eq!(confirmed.active, 1);
        assert_eq!(confirmed.detail_label(), "1 active instance(s)");
        assert_eq!(
            activity_certainty(&snapshot.system).cleanup_label(),
            "Cleanup confirmed"
        );

        snapshot.system.service = "capacity unavailable".into();
        let unavailable = product_activity(&snapshot, product);
        assert_eq!(unavailable.certainty, ActivityCertainty::Unavailable);
        assert_eq!(unavailable.active, 0);
        assert_eq!(unavailable.detail_label(), "Current activity unavailable");
        assert_eq!(
            activity_certainty(&snapshot.system).cleanup_label(),
            "Cleanup unknown"
        );
        snapshot.system.service = "active".into();
        snapshot.system.cleanup_unconfirmed = true;
        let uncertain = product_activity(&snapshot, product);
        assert_eq!(uncertain.certainty, ActivityCertainty::CleanupUncertain);
        assert_eq!(uncertain.active, 0);
        assert_eq!(
            uncertain.detail_label(),
            "Cleanup unresolved · current instances not confirmed"
        );
        assert_eq!(
            activity_certainty(&snapshot.system).cleanup_label(),
            "Cleanup unconfirmed"
        );
        snapshot.system.service = "capacity unavailable".into();
        assert_eq!(
            activity_certainty(&snapshot.system).cleanup_label(),
            "Cleanup unknown"
        );
        let unavailable_with_conservative_flag = product_activity(&snapshot, product);
        assert_eq!(
            unavailable_with_conservative_flag.certainty,
            ActivityCertainty::Unavailable
        );
        assert!(!attentions(&snapshot)
            .iter()
            .any(|item| item.title == "Cleanup is unconfirmed"));
    }

    #[test]
    fn shared_class_session_stays_class_level_on_both_product_surfaces() {
        let mut snapshot = snapshot();
        let mut second_build = snapshot.products[1].clone();
        second_build.module_sha256 = "different-build".into();
        second_build.environment = "different-environment".into();
        snapshot.products.push(second_build);
        snapshot.active_sessions = vec![serde_json::json!({
            "session":"exact-session", "class_id":"fixture-fragments", "state":"active", "recent":false
        })];
        for product in snapshot
            .products
            .iter()
            .filter(|p| p.class_id == "fixture-fragments")
        {
            let activity = product_activity(&snapshot, product);
            assert!(activity.shared_class);
            assert_eq!(activity.active, 1);
            assert_eq!(
                activity.detail_label(),
                "1 active in this plug-in class · exact build unavailable"
            );
        }
        assert!(session_product(&snapshot, "fixture-fragments").is_none());
        assert_eq!(
            session_display_name(&snapshot, "fixture-fragments"),
            "Shared plug-in class · exact build unavailable"
        );
        assert_eq!(snapshot.active_sessions[0]["session"], "exact-session");
    }

    #[test]
    fn exact_attention_target_and_class_level_session_claims() {
        let mut snapshot = snapshot();
        let p = &snapshot.products[2];
        let key = ProductKey::from(p);
        let item = attentions(&snapshot)
            .into_iter()
            .find(|item| item.title == p.name)
            .unwrap();
        assert_eq!(item.destination, Destination::Product(key));
        assert!(item.detail.contains("Rescan"));
        snapshot.active_sessions = vec![
            serde_json::json!({"class_id":"fixture-fragments","state":"active","recent":false}),
            serde_json::json!({"class_id":"fixture-fragments","state":"failed","recent":true,
                "terminal":"windows_host_exited","cleanup_confirmed":true,"transport_retired":true}),
        ];
        let state = product_activity(&snapshot, &snapshot.products[1]);
        assert_eq!(state.active, 1);
        assert_eq!(
            state.recent_failure,
            Some("Windows plug-in host exited unexpectedly")
        );
        assert!(!state.recent_cleanup_unconfirmed);
        snapshot.products.push(snapshot.products[1].clone());
        assert!(product_activity(&snapshot, &snapshot.products[1]).shared_class);
        assert!(session_product(&snapshot, "fixture-fragments").is_none());
        let class_items: Vec<_> = attentions(&snapshot)
            .into_iter()
            .filter(|item| item.destination == Destination::Activity)
            .collect();
        assert_eq!(class_items.len(), 1);
        assert!(class_items[0].detail.contains("exact build is unavailable"));
        snapshot.active_sessions[1]["session"] = serde_json::json!("exact-session");
        assert!(attentions(&snapshot)
            .iter()
            .any(|item| item.destination == Destination::Session("exact-session".into())));
    }
}
