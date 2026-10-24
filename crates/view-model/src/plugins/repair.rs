//! Explicit installer selection for one plug-in's existing setup. A draft is
//! local UI state; only the setup owner's exact current offer can run anything.
use crate::{model::{AvailableAction, EnvironmentInstaller, Snapshot},
    offers::{self, Importance, Offer}, Control, Local, ProductKey};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallerDraft {
    pub product: ProductKey,
    pub runtime: String,
    pub revision: u64,
    pub selected: Option<String>,
}

fn target<'a>(snapshot: &'a Snapshot, key: &ProductKey) -> Result<&'a EnvironmentInstaller, &'static str> {
    if snapshot.products.iter().filter(|p| ProductKey::from(*p) == *key).count() != 1 {
        return Err("This plug-in changed. Refresh its details before choosing an installer.");
    }
    let mut matching = snapshot.environment_installers.iter().filter(|t| t.environment == key.environment);
    let row = matching.next().ok_or("This setup's installer controls are unavailable. Refresh before continuing.")?;
    if matching.next().is_some() { return Err("This setup could not be identified exactly. Refresh before continuing."); }
    Ok(row)
}

pub fn draft(snapshot: &Snapshot, key: &ProductKey) -> Result<InstallerDraft, &'static str> {
    let row = target(snapshot, key)?;
    let runtime = row.runtime.as_ref().ok_or("This setup's current revision is unavailable. Refresh before continuing.")?;
    Ok(InstallerDraft {product:key.clone(),runtime:runtime.runner.clone(),revision:runtime.revision,selected:None})
}

pub fn offered(snapshot: &Snapshot, selected: &InstallerDraft) -> Result<AvailableAction, &'static str> {
    let current = draft(snapshot, &selected.product)?;
    if current.runtime != selected.runtime || current.revision != selected.revision {
        return Err("This setup changed while you were choosing. Choose the installer again.");
    }
    let installer = selected.selected.as_ref().ok_or("Choose the installer for this plug-in.")?;
    let row = target(snapshot, &selected.product)?;
    let mut matching = row.choices.iter().filter(|offer| offer.action.environment_installer_start()
        .is_some_and(|(environment, id, policy)| environment == selected.product.environment
            && id == installer && policy.is_none()));
    let mut offer = matching.next().ok_or("This installer is no longer offered for this setup. Choose it again.")?.clone();
    if matching.next().is_some() { return Err("This installer choice is ambiguous. Refresh before continuing."); }
    offer.label = "Run this installer…".into();
    Ok(offer)
}

pub struct Repair {
    pub choose: Control,
    pub retained: Vec<(String, Control)>,
    pub selected: Option<Result<Offer, &'static str>>,
    pub selected_name: Option<String>,
}

pub fn build(snapshot: &Snapshot, key: &ProductKey, selected: Option<&InstallerDraft>, fresh: bool, pending: bool)
    -> Result<Repair, &'static str> {
    let context = draft(snapshot, key)?;
    let row = target(snapshot, key)?;
    let retained = row.choices.iter().filter_map(|offer| {
        let (environment, installer, policy) = offer.action.environment_installer_start()?;
        if environment != key.environment || policy.is_some() {return None;}
        let name = snapshot.installer_setups.iter().find(|s| s.installer == installer)
            .map(|s| s.name.as_str()).unwrap_or(offer.label.as_str());
        let mut chosen = context.clone(); chosen.selected = Some(installer.into());
        Some((name.into(), Control {label:"Use this installer",importance:Importance::Secondary,
            enabled:fresh && !pending,does:Local::SelectRepairInstaller(chosen)}))
    }).collect();
    let selected = selected.filter(|draft| &draft.product == key);
    let selected_name = selected.and_then(|draft| draft.selected.as_ref()).map(|id|
        snapshot.installer_setups.iter().find(|s| &s.installer == id)
            .map(|s| s.name.clone()).unwrap_or_else(|| "Selected Windows installer".into()));
    Ok(Repair {
        choose:Control {label:"Choose installer file…",importance:Importance::Secondary,
            enabled:fresh && !pending,does:Local::ChooseRepairInstaller(context)},
        retained,
        selected:selected.filter(|draft|draft.selected.is_some()).map(|draft|
            offered(snapshot, draft).map(|offer|offers::offer(&offer,Importance::Primary,None,pending).held(!fresh))),
        selected_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{forms, model::Action, state::{Effect, ProductPage, State}};
    fn fixture() -> (Snapshot, ProductKey) {
        let mut snapshot = crate::samples::base();
        crate::samples::repair_fixture(&mut snapshot);
        let key = ProductKey::from(&snapshot.products[0]);
        (snapshot,key)
    }
    #[test]
    fn repair_requires_an_explicit_exact_setup_installer_and_preserves_owner_refusal() {
        let (mut snapshot,key) = fixture();
        let mut choice = draft(&snapshot,&key).unwrap();
        assert!(choice.selected.is_none(), "latest setup installer is not the plug-in's original installer");
        snapshot.environment_installers[0].installer = Some("22".repeat(32));
        assert!(draft(&snapshot,&key).unwrap().selected.is_none());
        assert!(offered(&snapshot,&choice).is_err());
        choice.selected = Some("11".repeat(32));
        let exact = offered(&snapshot,&choice).unwrap();
        assert!(matches!(&exact.action,Action::EnvironmentInstallerStart {environment,installer}
            if environment == &key.environment && installer == &"11".repeat(32)));
        let view = build(&snapshot,&key,Some(&choice),true,true).unwrap();
        assert!(!view.choose.enabled);
        assert!(view.selected.unwrap().unwrap().pressed().is_none());
        snapshot.environment_installers[0].choices[0].disabled_reason = Some("Close the affected plug-in".into());
        assert_eq!(offered(&snapshot,&choice).unwrap().disabled_reason.as_deref(),Some("Close the affected plug-in"));
        let mut wrong = snapshot.environment_installers[0].choices[0].clone();
        wrong.action = Action::EnvironmentInstallerStart {environment:"another-setup".into(),installer:"11".repeat(32)};
        snapshot.environment_installers[0].choices[0] = wrong;
        assert!(offered(&snapshot,&choice).is_err(),"same installer cannot borrow another setup's offer");
        snapshot.environment_installers[0].runtime.as_mut().unwrap().revision += 1;
        assert!(offered(&snapshot,&choice).unwrap_err().contains("setup changed"));
    }
    #[test]
    fn repair_confirmation_names_affected_setup_and_rechecks_exact_offer() {
        let (mut snapshot,key) = fixture();
        let mut choice = draft(&snapshot,&key).unwrap(); choice.selected=Some("11".repeat(32));
        let action = offered(&snapshot,&choice).unwrap().action;
        assert!(matches!(forms::route(action.clone(),None),forms::Route::Confirm(_)));
        let form = forms::removal_confirmation(Some(&snapshot),&action);
        assert!(form.body.iter().any(|line|line.contains("First-party shared setup")));
        assert!(form.body.iter().any(|line|line.contains("Sibling plug-in")));
        assert!(form.body.iter().any(|line|line.contains("presets")));
        assert_eq!(form.offer.unwrap().action,action);
        snapshot.environment_installers[0].choices.retain(|offer|offer.action != action);
        assert!(forms::removal_confirmation(Some(&snapshot),&action).offer.is_err());
    }
    #[test]
    fn repair_navigation_and_file_choice_do_not_send_or_retarget_a_request() {
        let (snapshot,key) = fixture();
        let context = draft(&snapshot,&key).unwrap();
        let mut state = State::default(); state.go(&crate::Destination::Product(key.clone()));state.plugins.page=ProductPage::Repair;
        assert_eq!(state.apply(Local::ChooseRepairInstaller(context.clone())),Some(Effect::ChooseInstaller));
        assert_eq!(state.plugins.focused_product(),Some(&key));
        let mut chosen = context; chosen.selected=Some("11".repeat(32));
        assert_eq!(state.apply(Local::SelectRepairInstaller(chosen.clone())),None);
        let action = offered(&snapshot,&chosen).unwrap().action;
        let mut confirmed = state.clone();
        assert!(confirmed.press(action.clone()).is_none());
        assert!(confirmed.forms.confirm.is_some());
        confirmed.go(&crate::Destination::Setup);
        assert!(confirmed.focus.repair.is_none() && confirmed.forms.confirm.is_none());
        let mut setup = state.clone();
        setup.go(&crate::Destination::Setup);
        assert!(setup.focus.repair.is_none());
        let other = snapshot.environment_installers[0].choices[1].action.clone();
        assert!(forms::confirmation_with_repair(Some(&snapshot),&other,setup.focus.repair.as_ref()).offer.is_ok());
        state.sent(&action);
        assert_eq!(state.plugins.focused_product(),Some(&key));
        assert_eq!(state.plugins.page,ProductPage::Overview);
        assert!(state.focus.repair.is_none());
    }
}
