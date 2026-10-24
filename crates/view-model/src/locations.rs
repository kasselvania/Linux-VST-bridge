//! Sample-folder observations and exact setup offers. No filesystem policy or
//! vendor library registration is inferred by the frontend.
use crate::{model::{Action, AvailableAction, Observation, Snapshot},
    offers::{self, Importance, OfferRow}, Text};

pub const UNAVAILABLE_GUIDANCE: &str = "New sessions in this setup wait until you reconnect the original folder or remove its access. Existing sessions keep running.";

#[derive(Clone, Debug, PartialEq)]
pub struct LocationsView {
    pub setup: String,
    pub members: Vec<String>,
    pub notice: Option<Text>,
    pub folders: Vec<Folder>,
    pub add: OfferRow,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Folder {
    pub name: String,
    pub path: String,
    pub drive: String,
    pub mode: &'static str,
    pub availability: Text,
    pub remove: OfferRow,
}
impl LocationsView {
    pub fn held(mut self, held: bool) -> Self {
        self.add = self.add.held(held);
        for folder in &mut self.folders { folder.remove = folder.remove.clone().held(held); }
        self
    }
    pub fn build(snapshot: &Snapshot, environment: &str, busy: Option<&str>, pending: bool) -> Self {
        let setup = snapshot.installer_setups.iter().find(|setup| setup.environment.as_deref() == Some(environment))
            .map(|setup| crate::setup::display_name(setup, snapshot)).unwrap_or_else(|| "Selected setup".into());
        let members = snapshot.products.iter().filter(|product| product.environment == environment)
            .map(|product| product.name.clone()).collect();
        let mut view = Self { setup, members, notice: None, folders: vec![], add: OfferRow::default() };
        let Some(owner) = snapshot.environments.iter().find(|owner| owner.id == environment) else {
            view.notice = Some(Text::caution("Library locations are unavailable. Refresh this setup's controls."));
            return view;
        };
        let adds: Vec<_> = owner.actions.iter().filter(|offer| matches!(&offer.action,
            Action::AddLibraryLocation { environment: chosen, read_only: false, .. } if chosen == environment))
            .cloned().collect();
        view.add = offers::row(&adds, Importance::Secondary, busy, pending);
        match &owner.library_locations {
            Observation::Unavailable(reason) => view.notice = Some(Text::caution(reason)),
            Observation::Observed(locations) => {
                if locations.is_empty() { view.notice = Some(Text::detail("No library folders added to this setup yet.")); }
                view.folders = locations.iter().map(|location| {
                    let removes: Vec<_> = owner.actions.iter().filter(|offer| matches!(&offer.action,
                        Action::RemoveLibraryLocation { environment: chosen, location: id, .. }
                            if chosen == environment && id == &location.id)).cloned().collect();
                    Folder { name: location.name.clone(), path: location.path.display().to_string(),
                        drive: location.drive.clone(), mode: if location.read_only { "Read-only" } else { "Read and write" },
                        availability: match &location.availability {
                            Observation::Observed(()) => Text::body("Available"),
                            Observation::Unavailable(reason) => Text::caution(format!("Unavailable · {reason} {UNAVAILABLE_GUIDANCE}")),
                        }, remove: offers::row(&removes, Importance::Quiet, busy, pending) }
                }).collect();
            }
        }
        view
    }
}

/// Both access modes must be actual offers for the same exact setup revision.
/// The form retains the offered action, never manufactures a permitted mode.
pub fn folder_modes(snapshot: Option<&Snapshot>, selected: &Action) -> Vec<AvailableAction> {
    let Action::AddLibraryLocation { environment, expected_environment, .. } = selected else { return vec![]; };
    snapshot.into_iter().flat_map(|snapshot| &snapshot.environments)
        .filter(|setup| &setup.id == environment).flat_map(|setup| &setup.actions)
        .filter(|offer| matches!(&offer.action, Action::AddLibraryLocation {
            environment: offered, expected_environment: revision, ..
        } if offered == environment && revision == expected_environment)).cloned().collect()
}
pub fn folder_offer(snapshot: Option<&Snapshot>, selected: &Action) -> Result<AvailableAction, String> {
    let offer = folder_modes(snapshot, selected).into_iter().find(|offer| &offer.action == selected)
        .ok_or("This folder access offer changed. Refresh this setup before choosing a folder.")?;
    if let Some(reason) = &offer.disabled_reason { return Err(reason.clone()); }
    Ok(offer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Environment, LibraryLocation};
    fn setup(id:&str, read_only:bool) -> Environment {
        let action = |read_only| AvailableAction {label:"Add a library folder…".into(),
            action:Action::AddLibraryLocation {environment:id.into(),expected_environment:format!("revision-{id}"),read_only},
            disabled_reason:None};
        Environment {id:id.into(),family:"Fixture".into(),runner:"fixture".into(),revision:1,
            authorization:"Not observed".into(),last_scan:serde_json::Value::Null,
            library_locations:Observation::Observed(vec![LibraryLocation {id:format!("location-{id}"),name:format!("Folder {id}"),
                path:format!("/mnt/{id}/Samples").into(),drive:"L:".into(),read_only,
                availability:Observation::Unavailable("Drive is disconnected".into())}]),
            actions:vec![action(false),action(true),AvailableAction {label:"Remove folder".into(),
                action:Action::RemoveLibraryLocation {environment:id.into(),expected_environment:format!("revision-{id}"),location:format!("location-{id}")},disabled_reason:None}]}
    }
    #[test]
    fn locations_and_actions_are_scoped_to_exact_setup_and_keep_unavailable_mapping() {
        let mut snapshot = crate::samples::snapshot(crate::window::Page::Plugins,Some("plugin_ready"));
        snapshot.environments = vec![setup("chosen",true),setup("other",false)];
        let view = LocationsView::build(&snapshot,"chosen",None,false);
        assert_eq!(view.folders.len(),1);
        assert_eq!(view.folders[0].path,"/mnt/chosen/Samples");
        assert_eq!(view.folders[0].drive,"L:");
        assert_eq!(view.folders[0].mode,"Read-only");
        assert!(view.folders[0].availability.words.contains("disconnected"));
        assert!(view.folders[0].availability.words.contains(UNAVAILABLE_GUIDANCE));
        assert!(matches!(&view.folders[0].remove.offers[0].intent,
            Action::RemoveLibraryLocation {environment,location,..} if environment=="chosen" && location=="location-chosen"));
        assert!(matches!(&view.add.offers[0].intent,
            Action::AddLibraryLocation {environment,read_only:false,..} if environment=="chosen"));
        let selected = view.add.offers[0].intent.clone();
        assert_eq!(folder_modes(Some(&snapshot),&selected).len(),2);
        assert!(matches!(crate::forms::route(selected.clone(),None),crate::forms::Route::ChooseLibraryFolder(_)));
        // The folder's path and retained letter remain when availability changes.
        let Observation::Observed(locations) = &mut snapshot.environments[0].library_locations else {panic!()};
        locations[0].availability=Observation::Observed(());
        let remounted = LocationsView::build(&snapshot,"chosen",None,false);
        assert_eq!(remounted.folders[0].drive,view.folders[0].drive);
        assert_eq!(remounted.folders[0].path,view.folders[0].path);
        assert_eq!(remounted.folders[0].availability.words,"Available");
        snapshot.environments[0].actions.clear();
        assert!(folder_offer(Some(&snapshot),&selected).is_err(),"other setup cannot supply authority");
        let stale = Action::AddLibraryLocation {environment:"other".into(),expected_environment:"old-revision".into(),read_only:false};
        assert!(folder_modes(Some(&snapshot),&stale).is_empty());
    }
    #[test]
    fn removal_confirms_content_preservation_and_refuses_changed_exact_location() {
        let mut snapshot = crate::samples::snapshot(crate::window::Page::Plugins,Some("plugin_ready"));
        snapshot.environments=vec![setup("chosen",false)];
        let action=snapshot.environments[0].actions[2].action.clone();
        assert_eq!(crate::forms::route(action.clone(),None),crate::forms::Route::Confirm(action.clone()));
        let confirmation=crate::forms::removal_confirmation(Some(&snapshot),&action);
        assert!(confirmation.offer.is_ok());
        assert!(confirmation.body.iter().any(|line|line.contains("all of its files stay")));
        if let Action::RemoveLibraryLocation {location,..}=&mut snapshot.environments[0].actions[2].action {
            *location="replacement".into();
        }
        assert!(crate::forms::removal_confirmation(Some(&snapshot),&action).offer.is_err());
    }
    #[test]
    fn missing_observation_does_not_mean_no_folders() {
        let mut snapshot=crate::samples::snapshot(crate::window::Page::Plugins,Some("plugin_ready"));
        snapshot.environments=vec![setup("chosen",false)];
        snapshot.environments[0].library_locations=crate::model::library_locations_unobserved();
        let view=LocationsView::build(&snapshot,"chosen",None,false);
        assert!(view.notice.unwrap().words.contains("not been observed"));
        snapshot.environments[0].library_locations=Observation::Observed(vec![]);
        let view=LocationsView::build(&snapshot,"chosen",None,false);
        assert!(view.notice.unwrap().words.contains("No library folders"));
    }
}
