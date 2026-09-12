//! UIO1 development observations. No registration/publication mutation, audio
//! calls, arbitrary executable launch, or new plug-in compatibility authority.
use crate::{profiles::*, publication::*, *};

#[derive(Debug, Serialize)]
pub struct Admission {
    pub schema: u32,
    pub profile_fingerprint: String,
    pub registration: Registration,
    pub accessibility_probe_permitted: bool,
    pub maximum_seconds: u32,
    pub maximum_records: u32,
}

pub fn admit(m: &Manager, class: &str) -> Result<Admission> {
    admit_selected(m, class, false)
}
/// The development experiment has its own exact compiled admission. It does
/// not grant ordinary UIO1 or activation authority to arbitrary candidates.
pub fn admit_uir1(m: &Manager) -> Result<Admission> {
    admit_selected(m, &crate::qualification::uir1_candidate()?.class.class_id, true)
}
fn admit_selected(m: &Manager, class: &str, uir1: bool) -> Result<Admission> {
    let _lock = m.lock("registry.lock")?;
    let db = m.registry()?;
    let e = db.classes.get(class).ok_or("uio1_class_absent")?;
    let reference = e.managed_revision.as_ref().ok_or("uio1_profile_absent")?;
    let revision = m.load_revision(class, reference)?;
    if uir1 {
        let exact = crate::qualification::uir1_candidate()?;
        require(revision.profile == exact && revision.qualification == Some(Qualification::Uir1Input),
            "uio1_exact_uir1_qualification_required")?;
        m.verify_retained_authority(&revision, &[exact])?;
    } else {
        let installed = installed_profiles()?;
        require(installed.contains(&revision.profile), "uio1_exact_installed_profile_required")?;
        require(revision.profile.claim == Claim::VerifiedExactFixture && revision.qualification.is_none(),
            "uio1_ordinary_profile_required")?;
    }
    require(
        e.publication == Publication::Published
            && !m.publication_pending(class)?
            && physical(&m.link(class))? == Some(revision.target.clone())
            && e.registration == revision.registration,
        "uio1_physical_binding_changed",
    )?;
    m.verify_completed_publication(&revision, reference)?;
    e.registration.verify(&m.root)?;
    // Manifest and native bytes are checked, not inferred from the profile name.
    Artifact {
        path: e
            .registration
            .host
            .path
            .with_file_name("host-source-manifest.json"),
        sha256: e.registration.host_source_sha256.clone(),
    }
    .verify()?;
    Ok(Admission {
        schema: 1,
        profile_fingerprint: revision.profile.fingerprint()?,
        registration: e.registration.clone(),
        accessibility_probe_permitted: revision.profile.capabilities.accessibility
            == Accessibility::WindowsDefault,
        maximum_seconds: 180,
        maximum_records: 16384,
    })
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Adapter {
    Xtest,
    PortalEis,
    Human,
}
/// XWayland windows can use XTEST only after an exact XID is resolved. The
/// compositor session being Wayland does not itself authorize global injection.
pub fn adapter(
    x11_target: bool,
    human: bool,
    portal_pointer_granted: bool,
    eis_connected: bool,
) -> Result<Adapter> {
    if human {
        return Ok(Adapter::Human);
    }
    if x11_target {
        return Ok(Adapter::Xtest);
    }
    require(
        portal_pointer_granted && eis_connected,
        "uio1_portal_consent_and_eis_required",
    )?;
    Ok(Adapter::PortalEis)
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Bracket {
    pub linux_before_ns: u64,
    pub linux_after_ns: u64,
    pub windows_qpc: u64,
    pub frequency: u64,
}
impl Bracket {
    /// Explicit interpolation bracket, not a claim of synchronized clocks.
    /// Only observations within ten seconds of a retained bracket are projected;
    /// 100 ppm adds conservative drift allowance to the measured round trip.
    pub fn project(self, tick: u64) -> Result<[u64; 2]> {
        require(
            self.frequency > 0
                && self.linux_after_ns >= self.linux_before_ns
                && self.linux_after_ns - self.linux_before_ns <= 100_000_000,
            "uio1_clock_bracket_invalid",
        )?;
        let delta = (i128::from(tick) - i128::from(self.windows_qpc)) * 1_000_000_000
            / i128::from(self.frequency);
        require(delta.abs() <= 10_000_000_000, "uio1_clock_bracket_stale")?;
        let drift = delta.abs() / 10_000;
        let lo = i128::from(self.linux_before_ns) + delta - drift;
        let hi = i128::from(self.linux_after_ns) + delta + drift;
        require(
            lo >= 0 && hi <= i128::from(u64::MAX),
            "uio1_clock_projection_overflow",
        )?;
        Ok([lo as u64, hi as u64])
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    HostParameter,
    ParameterDrag,
    PageClick,
    Idle,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WitnessKind {
    InputIssued,
    X11Received,
    Win32Retrieved,
    DispatchEntered,
    DispatchReturned,
    MouseHookEntered,
    MouseHookReturned,
    HeartbeatAcknowledged,
    GestureBegin,
    ParameterValue,
    GestureEnd,
    HostValue,
    FirstPixelChange,
    StableVisualSignature,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Witness {
    pub kind: WitnessKind,
    pub linux_interval_ns: [u64; 2],
    pub target_alias: u32,
    pub message: Option<u32>,
    pub parameter: Option<u32>,
    pub value: Option<f64>,
    #[serde(default)]
    pub focus_alias: Option<u32>,
    #[serde(default)]
    pub active_alias: Option<u32>,
    #[serde(default)]
    pub capture_alias: Option<u32>,
    #[serde(default)]
    pub client_coordinates: Option<[i32; 2]>,
    #[serde(default)]
    pub result: Option<i64>,
}

/// Intermediate report with already-aliased identities and explicit clocks.
/// Windows QPC is projected here, not subtracted from Linux timestamps by an
/// exporter. Linux polling witnesses must retain their observation interval.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "domain", rename_all = "snake_case", deny_unknown_fields)]
pub enum TimePoint {
    Linux { interval: [u64; 2] },
    WindowsQpc { ticks: u64 },
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClockedWitness {
    pub action: u32,
    pub time: TimePoint,
    pub witness: Witness,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub report: Waterfall,
    pub witnesses: Vec<ClockedWitness>,
}
impl Projection {
    pub fn project(mut self) -> Result<Waterfall> {
        self.report.validate()?;
        require(
            self.witnesses.len() <= 32768
                && self.report.actions.iter().all(|a| a.witnesses.is_empty()),
            "uio1_projection_bound",
        )?;
        for mut w in self.witnesses {
            w.witness.linux_interval_ns = match w.time {
                TimePoint::Linux { interval } => interval,
                TimePoint::WindowsQpc { ticks } => {
                    let b = self
                        .report
                        .brackets
                        .iter()
                        .min_by_key(|b| b.windows_qpc.abs_diff(ticks))
                        .ok_or("uio1_clock_bracket_absent")?;
                    b.project(ticks)?
                }
            };
            self.report
                .actions
                .iter_mut()
                .find(|a| a.id == w.action)
                .ok_or("uio1_action_absent")?
                .witnesses
                .push(w.witness);
        }
        for a in &mut self.report.actions {
            a.witnesses.sort_by_key(|w| w.linux_interval_ns);
        }
        self.report.validate()?;
        Ok(self.report)
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub id: u32,
    pub kind: ActionKind,
    pub witnesses: Vec<Witness>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Waterfall {
    pub schema: u32,
    pub profile_fingerprint: String,
    pub adapter: Adapter,
    pub diagnostic_enabled: bool,
    pub capacity: u32,
    pub dropped: u64,
    pub actions: Vec<Action>,
    pub brackets: Vec<Bracket>,
}
impl Waterfall {
    pub fn validate(&self) -> Result<()> {
        require(
            self.schema == 1
                && valid_hex(&self.profile_fingerprint, 64)
                && self.capacity == 16384
                && self.actions.len() <= 4
                && self.brackets.len() <= 64,
            "uio1_report_contract",
        )?;
        let mut ids = std::collections::BTreeSet::new();
        for a in &self.actions {
            require(
                a.id > 0 && ids.insert(a.id) && a.witnesses.len() <= 16384,
                "uio1_action_bound",
            )?;
            for w in &a.witnesses {
                require(
                    w.target_alias <= 128
                        && [w.focus_alias, w.active_alias, w.capture_alias]
                            .iter()
                            .all(|a| a.is_none_or(|a| a <= 128))
                        && w.linux_interval_ns[0] <= w.linux_interval_ns[1]
                        && w.value
                            .is_none_or(|v| v.is_finite() && (0.0..=1.0).contains(&v)),
                    "uio1_witness_contract",
                )?;
            }
        }
        for b in &self.brackets {
            b.project(b.windows_qpc)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adapters_require_actual_targets_or_portal_grants() {
        assert_eq!(adapter(true, false, false, false).unwrap(), Adapter::Xtest);
        assert!(adapter(false, false, false, false).is_err());
        assert!(adapter(false, false, true, false).is_err());
        assert_eq!(
            adapter(false, false, true, true).unwrap(),
            Adapter::PortalEis
        );
        assert_eq!(adapter(false, true, false, false).unwrap(), Adapter::Human);
    }
    #[test]
    fn projection_retains_round_trip_and_drift() {
        let b = Bracket {
            linux_before_ns: 10_000_000_000,
            linux_after_ns: 10_001_000_000,
            windows_qpc: 500,
            frequency: 1000,
        };
        assert_eq!(b.project(500).unwrap(), [10_000_000_000, 10_001_000_000]);
        assert_eq!(b.project(1500).unwrap(), [10_999_900_000, 11_001_100_000]);
        assert!(b.project(20000).is_err());
        assert!(Bracket { frequency: 0, ..b }.project(500).is_err());
        assert!(Bracket {
            linux_after_ns: 0,
            ..b
        }
        .project(500)
        .is_err());
    }
    #[test]
    fn public_report_cannot_carry_text_paths_or_hwnds() {
        let w = r#"{"kind":"win32_retrieved","linux_interval_ns":[1,2],"target_alias":1,"message":513,"parameter":null,"value":null}"#;
        assert!(serde_json::from_str::<Witness>(w).is_ok());
        for field in ["text", "path", "hwnd", "account", "payload"] {
            assert!(serde_json::from_str::<Witness>(
                &w.replace("}", &format!(",\"{field}\":\"secret\"}}"))
            )
            .is_err());
        }
    }
    #[test]
    fn reporter_does_not_conflate_windows_and_linux_clocks() {
        let report = Waterfall {
            schema: 1,
            profile_fingerprint: "a".repeat(64),
            adapter: Adapter::Human,
            diagnostic_enabled: true,
            capacity: 16384,
            dropped: 0,
            actions: vec![Action {
                id: 1,
                kind: ActionKind::PageClick,
                witnesses: vec![],
            }],
            brackets: vec![Bracket {
                linux_before_ns: 5_000_000_000,
                linux_after_ns: 5_002_000_000,
                windows_qpc: 100,
                frequency: 1000,
            }],
        };
        let witness:Witness=serde_json::from_str(r#"{"kind":"win32_retrieved","linux_interval_ns":[0,0],"target_alias":1,"message":513,"parameter":null,"value":null}"#).unwrap();
        let p = Projection {
            report,
            witnesses: vec![ClockedWitness {
                action: 1,
                time: TimePoint::WindowsQpc { ticks: 1100 },
                witness,
            }],
        }
        .project()
        .unwrap();
        assert_eq!(
            p.actions[0].witnesses[0].linux_interval_ns,
            [5_999_900_000, 6_002_100_000]
        );
        assert!(serde_json::from_str::<TimePoint>(
            r#"{"domain":"linux","interval":[0,1],"account":"secret"}"#
        )
        .is_err());
    }
}
