//! Recovery observations explain an exact instance. Only a matching current
//! product offer authorizes Restart; pushed observations never supply authority.
use crate::{diagnostics::Section, model::{self, Action, Observation as O, Product},
    offers::{self, Importance, OfferRow}, Detail, Disclosure, Text};

pub const LOSS_WARNING: &str = "Restore the last confirmed state; changes made after it may be lost.";

#[derive(Clone, Debug, PartialEq)]
pub struct Recovery {
    pub name: String,
    pub context: String,
    pub status: crate::Status,
    pub processing: String,
    pub detail: Vec<Text>,
    pub actions: OfferRow,
    pub identity: Disclosure,
}

pub fn for_product(read: Section<'_, &model::Activity>, product: &Product,
    busy: Option<&str>, pending: bool) -> Vec<Recovery> {
    let Section::Loaded(activity) = read else { return vec![]; };
    let O::Observed(instances) = &activity.live.instances else { return vec![]; };
    instances.iter().filter(|instance| instance.product.observed().is_some_and(|key|key.matches(product)))
        .filter_map(|instance| for_instance(instance, Some(product), busy, pending)).collect()
}

pub fn for_instance(instance: &model::LiveInstance, product: Option<&Product>,
    busy: Option<&str>, pending: bool) -> Option<Recovery> {
    let recovery = instance.recovery.as_ref()?;
    use model::RecoveryPhase as P;
    use crate::StatusClass as C;
    let (words, class) = match recovery.phase {
        P::Connected => return None,
        P::Failed => ("Plug-in instance failed", C::Problem),
        P::Restarting => ("Restarting", C::Working),
        P::Restored => ("Restarted", C::Ready),
        P::Refused => ("Restart could not complete", C::Problem),
        P::Uncertain => ("Restart outcome unconfirmed", C::NeedsYou),
        P::Unknown => ("Recovery status unavailable", C::Unknown),
    };
    let matched = product.filter(|product| instance.product.observed().is_some_and(|key|key.matches(product)));
    let mut detail = vec![Text::body(&recovery.detail)];
    match &recovery.snapshot {
        O::Observed(snapshot) => detail.push(Text::detail(format!("Last confirmed state: revision {}", snapshot.revision))),
        O::Unavailable(reason) => detail.push(Text::detail(format!("Confirmed state unavailable: {reason}"))),
    }
    if recovery.uncaptured_changes_possible { detail.push(Text::caution(LOSS_WARNING)); }
    let current = recovery.restart.as_ref().filter(|reference| match (&reference.action, &recovery.snapshot) {
        (Action::PluginRestart {instance,generation,snapshot}, O::Observed(confirmed)) =>
            instance == &recovery.instance && generation == &recovery.generation && snapshot == confirmed,
        _ => false,
    }).and_then(|reference| product.filter(|product|
        instance.product.observed().is_some_and(|key|key.matches(product)))
        .and_then(|product|product.actions.iter().find(|offer|offer.action == reference.action)));
    let actions = current.map(|offer| offers::row(std::slice::from_ref(offer), Importance::Primary, busy, pending))
        .unwrap_or_else(|| OfferRow {notes: recovery.restart.as_ref().map(|_|vec![
            "Restart is not offered by the current plug-in controls. Refresh status before trying again.".into()]).unwrap_or_default(),
            ..OfferRow::default()});
    Some(Recovery {
        name: matched.map(|product| product.name.clone()).unwrap_or_else(|| "Plug-in instance".into()),
        context: crate::product::host_text(instance),
        status: crate::Status {words, class},
        processing: crate::product::processing_text(instance),
        detail, actions, identity:Disclosure::new("Confirmed recovery state", vec![Detail::Text(Text::detail(
        format!("Instance: {} · host generation {}", recovery.instance, recovery.generation))),
        crate::record(&serde_json::to_value(&recovery.snapshot).unwrap_or_default())])})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{forms, offers::Availability, samples};

    #[test]
    fn contextual_phase_never_comes_from_prose_pending_clicks_or_processing() {
        let mut snapshot = samples::base();
        let activity = samples::recovery_fixture(&mut snapshot);
        let mut instance = activity.live.instances.observed().unwrap()[0].clone();
        for (phase, title) in [
            (model::RecoveryPhase::Failed, "Plug-in instance failed"),
            (model::RecoveryPhase::Restarting, "Restarting"),
            (model::RecoveryPhase::Restored, "Restarted"),
            (model::RecoveryPhase::Unknown, "Recovery status unavailable"),
        ] {
            instance.recovery.as_mut().unwrap().phase = phase;
            instance.recovery.as_mut().unwrap().detail = "Playing again".into();
            let view = for_instance(&instance, Some(&snapshot.products[0]), None, true).unwrap();
            assert_eq!(view.status.words, title);
            assert_eq!(view.name, snapshot.products[0].name);
            assert!(!view.processing.contains("Playing again"));
        }
        instance.recovery.as_mut().unwrap().phase = model::RecoveryPhase::Connected;
        assert!(for_instance(&instance, Some(&snapshot.products[0]), None, false).is_none());
        instance.recovery.as_mut().unwrap().phase = model::RecoveryPhase::Restored;
        instance.terminal = O::Observed(None);
        instance.processing = model::ProcessingObservation::Advancing {epoch:1,completed_end:512,observed_age_ms:1};
        let view = for_instance(&instance, None, None, false).unwrap();
        assert_eq!(view.name, "Plug-in instance");
        assert!(view.actions.offers.is_empty());
        assert!(view.processing.contains("audible output is not measured"));
    }

    #[test]
    fn observation_only_rejoins_the_exact_current_product_offer() {
        let mut snapshot = samples::base();
        let activity = samples::recovery_fixture(&mut snapshot);
        let product = &snapshot.products[0];
        let instance = &activity.live.instances.observed().unwrap()[0];
        let expected = instance.recovery.as_ref().unwrap().restart.as_ref().unwrap().action.clone();
        let view = for_instance(instance, Some(product), None, false).unwrap();
        assert_eq!(view.actions.offers.len(), 1);
        assert_eq!(view.actions.offers[0].intent, expected);
        assert!(view.actions.offers[0].availability.enabled());
        assert!(view.detail.iter().any(|line|line.words==LOSS_WARNING));
        assert!(for_instance(instance,None,None,false).unwrap().actions.offers.is_empty());
        let mut changed = product.clone();
        changed.actions.retain(|offer| !matches!(offer.action,Action::PluginRestart {..}));
        assert!(for_instance(instance,Some(&changed),None,false).unwrap().actions.offers.is_empty());
        for field in 0..3 {
            let mut changed = product.clone();
            match field {0=>changed.environment.push('x'),1=>changed.module_sha256.push('x'),_=>changed.class_id.push('x')}
            assert!(for_instance(instance,Some(&changed),None,false).unwrap().actions.offers.is_empty());
        }
        for field in 0..3 {
            let mut changed = product.clone();
            let offer = changed.actions.iter_mut().find(|offer|offer.action==expected).unwrap();
            let Action::PluginRestart {generation,snapshot,instance: original} = &mut offer.action else {panic!()};
            match field {0=>*generation+=1,1=>snapshot.sha256="ef".repeat(32),_=>original.push('x')}
            assert!(for_instance(instance,Some(&changed),None,false).unwrap().actions.offers.is_empty());
        }
        let mut malformed = instance.clone();
        malformed.recovery.as_mut().unwrap().generation += 1;
        assert!(for_instance(&malformed,Some(product),None,false).unwrap().actions.offers.is_empty());
        let mut refused = product.clone();
        refused.actions.iter_mut().find(|offer|offer.action==expected).unwrap().disabled_reason=Some("Restart already in progress".into());
        let view=for_instance(instance,Some(&refused),None,true).unwrap();
        assert_eq!(view.actions.offers[0].availability,Availability::Refused("Restart already in progress".into()));
        assert!(view.actions.notes.iter().any(|line|line=="Restart already in progress"));
        assert_eq!(for_instance(instance,Some(product),None,true).unwrap().actions.offers[0].availability,Availability::Waiting);
        for read in [Section::NotRequested,Section::Loading,Section::Failed("Lost connection")] {
            assert!(for_product(read,product,None,false).is_empty());
        }
    }

    fn instance_or_original(activity:&model::Activity)->&model::LiveInstance { &activity.live.instances.observed().unwrap()[0] }

    #[test]
    fn restart_confirmation_keeps_the_exact_snapshot_and_rechecks_the_offer() {
        let mut snapshot=samples::base();
        let activity=samples::recovery_fixture(&mut snapshot);
        let action=instance_or_original(&activity).recovery.as_ref().unwrap().restart.as_ref().unwrap().action.clone();
        assert_eq!(forms::route(action.clone(),None),forms::Route::Confirm(action.clone()));
        let confirmation=forms::removal_confirmation(Some(&snapshot),&action);
        assert!(confirmation.body.iter().any(|line|line==LOSS_WARNING));
        assert_eq!(confirmation.offer.unwrap().action, action);
        assert_eq!(forms::confirmed(action.clone()),action);
        let current=snapshot.products[0].actions.iter_mut().find(|offer|offer.action==action).unwrap();
        let Action::PluginRestart {generation,..}=&mut current.action else {panic!()};
        *generation+=1;
        assert!(forms::removal_confirmation(Some(&snapshot),&action).offer.is_err());
    }

    #[test]
    fn activity_never_uses_pushed_actions_without_current_authority() {
        let mut snapshot=samples::base();
        let activity=samples::recovery_fixture(&mut snapshot);
        let view=crate::activity::Activity::from_current(&snapshot,Section::Loaded(&activity),false,false,None,false);
        assert!(view.live.rows[0].recovery.as_ref().unwrap().actions.offers.is_empty());
        let view=crate::activity::Activity::from_current(&snapshot,Section::Loaded(&activity),false,false,Some(&snapshot),false);
        assert_eq!(view.live.rows[0].recovery.as_ref().unwrap().actions.offers.len(),1);
        let view=crate::activity::Activity::from_current(&snapshot,Section::Failed("Disconnected"),false,false,Some(&snapshot),false);
        assert!(view.live.rows.is_empty());
    }
}
