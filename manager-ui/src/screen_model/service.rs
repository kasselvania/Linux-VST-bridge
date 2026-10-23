//! The bridge service's own health, separate from plug-in availability.
use crate::model::System;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Health {
    Ready,
    InUse,
    Maintenance,
    NeedsAttention,
    Unavailable,
}

impl Health {
    pub fn title(self) -> &'static str {
        match self {
            Self::Ready => "Bridge service running",
            Self::InUse => "Plug-ins running",
            Self::Maintenance => "Installation or scan running",
            Self::NeedsAttention => "Bridge service needs attention",
            Self::Unavailable => "Bridge service status unavailable",
        }
    }

}

pub fn health(system: &System) -> Health {
    if !system.capacity_available() {
        Health::Unavailable
    } else if system.cleanup_unconfirmed
        || system.pending_transactions > 0
        || system.stale_transports > 0
    {
        Health::NeedsAttention
    } else if system.maintenance > 0 {
        Health::Maintenance
    } else if system.dsp > 0 {
        Health::InUse
    } else {
        Health::Ready
    }
}

/// The manager paused the bridge for its own automatic installation step.
///
/// The window reads this from the operation's raw record, and the two rules
/// below from the service's counters. They duplicate what the service should
/// report as one status; retire them when it does (#489).
pub fn owned_service_pause(operation: Option<&serde_json::Value>) -> bool {
    let Some(operation) = operation else { return false; };
    let pause = &operation["owned_service_pause"];
    pause["schema"] == 1
        && pause["automatic_operation"] == operation["operation"]
        && operation["operation"].as_str().is_some_and(|id| !id.is_empty())
        && pause["owner_operation"].as_str().is_some_and(|id| !id.is_empty())
        && matches!(operation["state"].as_str(), Some("waiting" | "running" | "vendor_running"))
        && matches!(operation["journey"]["phase"].as_str(), Some("preparing_environment"
            | "vendor_running" | "discovering" | "preparing_plugins"))
        && operation["journey"]["failure"].is_null()
}

/// Whether the bridge is paused for the manager's own installation and left
/// nothing else to resolve. `owned_pause` is `owned_service_pause` of the
/// current operation; `system` the current or last observed service status.
pub fn paused_for_installation(owned_pause: bool, system: Option<&System>) -> bool {
    owned_pause && system.is_none_or(|system| system.pending_transactions == 0
        && system.stale_transports == 0
        && (!system.capacity_available() || !system.cleanup_unconfirmed))
}

/// A service problem the musician must resolve before anything else: the
/// service cannot report, a setup change did not finish, or cleanup of an
/// earlier plug-in or installer is unconfirmed. An installation the manager
/// paused the bridge for is not one, unless it also left such a problem.
pub fn problem(system: &System, operation: Option<&serde_json::Value>) -> Option<&'static str> {
    let owned_pause = paused_for_installation(owned_service_pause(operation), Some(system));
    let problem = !system.capacity_available() || system.cleanup_unconfirmed
        || system.stale_transports > 0 || system.pending_transactions > 0;
    (problem && !owned_pause).then_some(if !system.capacity_available() {
        "The bridge service could not report its current status."
    } else if system.pending_transactions > 0 {
        "A previous setup change has not finished."
    } else {
        "Could not confirm that an earlier plug-in or installer closed."
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system() -> System {
        serde_json::from_str::<crate::model::Snapshot>(include_str!("../../examples/library-preview.json"))
            .unwrap().system
    }

    #[test]
    fn health_distinguishes_ready_busy_unavailable_and_cleanup_uncertainty() {
        let mut system = system();
        assert_eq!(health(&system), Health::Ready);
        system.dsp = 2;
        assert_eq!(health(&system), Health::InUse);
        system.maintenance = 1;
        assert_eq!(health(&system), Health::Maintenance);
        system.pending_transactions = 1;
        assert_eq!(health(&system), Health::NeedsAttention);
        system.pending_transactions = 0;
        system.cleanup_unconfirmed = true;
        assert_eq!(health(&system), Health::NeedsAttention);
        system.service = "capacity unavailable".into();
        assert_eq!(health(&system), Health::Unavailable);
    }

    #[test]
    fn an_owned_installation_pause_is_not_a_service_problem_unless_it_left_one() {
        let mut system = system();
        let paused = serde_json::json!({"operation": "op", "state": "running",
            "owned_service_pause": {"schema": 1, "automatic_operation": "op", "owner_operation": "owner"},
            "journey": {"phase": "preparing_plugins", "failure": null}});
        assert!(owned_service_pause(Some(&paused)));
        system.service = "capacity unavailable".into();
        assert_eq!(problem(&system, None), Some("The bridge service could not report its current status."));
        assert_eq!(problem(&system, Some(&paused)), None);
        system.service = "active".into();
        system.pending_transactions = 1;
        assert_eq!(problem(&system, Some(&paused)), Some("A previous setup change has not finished."));
        system.pending_transactions = 0;
        system.cleanup_unconfirmed = true;
        assert_eq!(problem(&system, Some(&paused)),
            Some("Could not confirm that an earlier plug-in or installer closed."));
    }
}
