//! A plug-in's launch and audio settings and its editor checks, worded from the
//! manager-resolved records. Only the manager's offered, typed controls appear.
use super::offers::{self, Importance};
use super::{Detail, Disclosure, Text};
use crate::model::{Action, AvailableAction, Product};

/// Settings controls offered for this plug-in. Legacy graphics trials are
/// hidden once the shared configuration record exists.
pub fn settings_actions(product: &Product) -> Vec<AvailableAction> {
    let shared = !product.details["configuration"].is_null();
    product.actions.iter().filter(|offer| match offer.action {
        Action::CandidateGraphicsAssess { .. } | Action::CandidateSettingsPrepare { .. } => true,
        Action::CandidateGraphicsPrepare { .. } => !shared,
        _ => false,
    }).cloned().collect()
}

/// The Settings section, when the plug-in has settings records or controls.
pub fn section(product: &Product, busy: Option<&str>, pending: bool, expanded: bool) -> Option<Disclosure> {
    let configuration = &product.details["configuration"];
    let legacy = &product.details["graphics"];
    let actions: Vec<_> = settings_actions(product).into_iter()
        .filter(|offer| !matches!(offer.action, Action::CandidateGraphicsAssess { .. })).collect();
    let buffering: Vec<_> = product.actions.iter().filter(|offer|
        matches!(offer.action, Action::BufferingSet { .. } | Action::DeliverySet { .. })).cloned().collect();
    if configuration.is_null() && legacy.is_null() && actions.is_empty() && buffering.is_empty() {
        return None;
    }
    let settings = if configuration.is_null() { legacy } else { configuration };
    let mut content = Vec::new();
    if !configuration.is_null() {
        content.push(Text::strong("Launch settings").into());
        content.extend(choices(configuration, expanded));
        let mut technical: Vec<Detail> = Vec::new();
        for (key, label) in [("runtime", "Selected runtime"), ("environment", "Selected environment"),
            ("environment_revision", "Environment revision")] {
            technical.push(Text::detail(format!("{label}: {}", value(&configuration["launch"][key]))).into());
        }
        for (key, label) in [("module", "Module"), ("class_id", "Class"),
            ("host", "Windows host"), ("native", "Native bridge")] {
            technical.push(Text::detail(format!("{label}: {}", value(&configuration["launch"][key]))).into());
        }
        technical.push(Text::detail(format!("Candidate: {}", value(&configuration["candidate"]))).into());
        technical.push(Text::detail(format!("Publication: {}", value(&configuration["publication"]))).into());
        technical.push(Text::body(configuration["qualification"].as_str().unwrap_or("Qualification unavailable")).into());
        content.push(Detail::Disclosure(Disclosure::new("Technical setup details", technical).forced_open(expanded)));
    } else if !legacy.is_null() {
        content.push(Text::body(legacy["requested"].as_str().unwrap_or("Requested settings unavailable")).into());
        content.push(Text::detail(legacy["reason"].as_str().unwrap_or_default()).into());
    }
    if settings["publication"] == "another_configuration" {
        content.push(Text::strong("Prepared only. Your previous configuration is still selected.").into());
    }
    content.push(Text::detail("Prepare changed launch settings below, then select the prepared setup under Other setup options. Your current setup stays selected until then.").into());
    content.push(Detail::Offers(offers::row(&actions, Importance::Quiet, busy, pending)));
    if !configuration["buffering"].is_null() || !buffering.is_empty() {
        content.push(Detail::Divider);
        content.push(Text::strong("Bridge audio delivery").into());
        let audio = &configuration["buffering"];
        if let Some(frames) = audio["added_frames"].as_u64()
            .or_else(|| product.details["added_frames"].as_u64()) {
            content.push(Text::body(format!("{frames} added bridge frames")).into());
            if audio["delivery_mode"] == "same_callback" || product.details["delivery_mode"] == "same_callback" {
                content.push(Text::body("Same-callback delivery · test mode").into());
                if let Some(remembered) = audio["remembered_frames"].as_u64()
                    .or_else(|| product.details["buffered_frames"].as_u64()) {
                    content.push(Text::detail(format!("Buffered delivery retains {remembered} frames for restoration.")).into());
                }
            } else { content.push(Text::body("Buffered delivery").into()); }
            content.push(Text::detail(format!("Source: {}", value(&configuration["buffering"]["source"]))).into());
        }
        content.push(Text::detail("This is the bridge's added buffering. The DAW sets the live sample rate and processing block size; they are not measured here.").into());
        content.push(Detail::Offers(offers::row(&buffering, Importance::Quiet, busy, pending)));
    }
    if !settings["before"].is_null() {
        let mut before = if !configuration.is_null() {
            choices(&settings["before"], expanded)
        } else {
            vec![Text::body(settings["before"]["requested"].as_str()
                .unwrap_or("Retained previous configuration")).into()]
        };
        before.extend(assessment("Previous assessment", &settings["before"]["assessment"]));
        content.push(Detail::Disclosure(Disclosure::new("Before this trial", before)));
    }
    Some(Disclosure::new("Settings", content).forced_open(expanded))
}

fn value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Number(value) => value.to_string(),
        _ => "Unavailable".into(),
    }
}

fn choices(configuration: &serde_json::Value, expanded: bool) -> Vec<Detail> {
    let Some(choices) = configuration["choices"].as_array().filter(|choices| !choices.is_empty()) else {
        return vec![Text::detail("Requested choices unavailable.").into()];
    };
    let mut details = Vec::new();
    for choice in choices {
        details.push(Text::body(format!("{}: {}", value(&choice["name"]), value(&choice["value"]))).into());
        details.push(Detail::Disclosure(Disclosure::new(format!("{} setting details", value(&choice["name"])), vec![
            Text::detail(format!("Source: {} · Scope: {}", value(&choice["source"]), value(&choice["scope"]))).into(),
            Text::detail(format!("Application: {}", value(&choice["change"]))).into(),
        ]).forced_open(expanded)));
    }
    details
}

/// One recorded editor assessment, with what it does not establish.
pub fn assessment(label: &str, result: &serde_json::Value) -> Vec<Detail> {
    let mut lines = vec![Text::strong(label)];
    if result.is_null() {
        lines.push(Text::detail("Not assessed with these settings."));
        return lines.into_iter().map(Detail::Text).collect();
    }
    lines.push(Text::body(format!("Editor check: {}", result["editor"]["status"].as_str().unwrap_or("unknown"))));
    if matches!(result["editor_retirement"].as_str(), Some("timed_out" | "failed")) {
        lines.push(Text::alert(
            "The editor did not close. Graphics findings were retained; the editor is not confirmed working."));
    }
    if let Some(reason) = result["recommendation"]["reason"].as_str() { lines.push(Text::body(reason)); }
    if result["applied"]["prepared"].is_string() {
        let setup = result["applied"]["setup"].as_u64()
            .map(|n| format!(" as setup {n}")).unwrap_or_default();
        lines.push(Text::body(format!("Prepared: Wine's built-in Direct3D 11 is set up for this plug-in's next launch{setup}. Its Try offer below tests it in the DAW.")));
    } else if let Some(action) = result["recommendation"]["action"].as_str() {
        lines.push(Text::body(format!("Next: {action}")));
    }
    lines.push(Text::detail("Effective editor renderer: not established by this assessment."));
    if let Some(probes) = result["runtime_probes"].as_array() {
        // What the probe found loaded, whatever graphics setting was requested.
        match probes.iter().find(|probe| probe["api"] == "d3d11_default")
            .and_then(|probe| probe["implementation"].as_str()) {
            Some("wine_builtin") if result["context"]["requested_backend"].is_null() =>
                lines.push(Text::detail("Direct3D 11 on this launch: Wine's built-in. The Wine D3D11 setting selects this same implementation.")),
            Some("wine_builtin") => lines.push(Text::detail("Direct3D 11 on this launch: Wine's built-in.")),
            Some(_) => lines.push(Text::detail("Direct3D 11 on this launch: the runner's own provider, not Wine's built-in.")),
            None => {}
        }
        lines.push(Text::detail("Independent runtime probes"));
        for probe in probes {
            let renderer = probe["renderer"].as_str().unwrap_or("renderer unknown");
            // Wine's built-in Direct3D reports a fixed stand-in card.
            let note = if probe["implementation"] == "wine_builtin" && probe["renderer"].is_string() {
                " (Wine's stand-in adapter name, not the graphics device)" } else { "" };
            lines.push(Text::detail(format!("{}: {} · {renderer}{note}", probe["api"].as_str().unwrap_or("API"),
                probe["status"].as_str().unwrap_or("unknown"))));
        }
    }
    lines.push(Text::detail("Recorded run; current driver/device freshness is unverified."));
    lines.into_iter().map(Detail::Text).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen_model::TextRole;

    fn words(details: &[Detail], out: &mut Vec<String>) {
        for detail in details {
            match detail {
                Detail::Text(text) => out.push(text.words.clone()),
                Detail::Disclosure(disclosure) => { out.push(disclosure.title.clone()); words(&disclosure.content, out); }
                Detail::Offers(row) => out.extend(row.offers.iter().map(|offer| offer.label.clone())
                    .chain(row.notes.iter().cloned())),
                Detail::Choice(choice) => out.push(choice.offer.label.clone()),
                Detail::Divider => {}
            }
        }
    }

    #[test]
    fn same_callback_delivery_keeps_its_remembered_buffer_and_no_other_frames() {
        let mut product: Product = serde_json::from_str::<crate::model::Snapshot>(
            include_str!("../../examples/library-preview.json")).unwrap().products.remove(0);
        product.details = serde_json::json!({"configuration": {"buffering": {
            "added_frames":0,"remembered_frames":512,"delivery_mode":"same_callback",
            "source":"Explicit class preference"}}});
        let settings = section(&product, None, false, true).expect("buffering record");
        assert!(settings.force_open);
        let mut lines = vec![];
        words(&settings.content, &mut lines);
        assert!(lines.iter().any(|line| line == "0 added bridge frames"));
        assert!(lines.iter().any(|line| line.contains("retains 512 frames for restoration")));
        assert!(!lines.iter().any(|line| line.contains("512 added bridge frames")));
        assert!(settings.content.contains(&Detail::Divider));
    }

    #[test]
    fn unclosed_editor_is_an_alert_and_never_a_working_claim() {
        let result = serde_json::json!({"editor":{"status":"opened"},"editor_retirement":"timed_out"});
        let lines = assessment("Configuration assessment", &result);
        assert!(lines.iter().any(|line| matches!(line, Detail::Text(text)
            if text.role == TextRole::Alert && text.words.starts_with("The editor did not close"))));
        assert_eq!(assessment("Previous assessment", &serde_json::Value::Null).len(), 2);
    }
}
