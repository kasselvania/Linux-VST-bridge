//! The Plug-ins screens: the searchable list, one plug-in's detail, and the
//! read-only list shown while a plug-in's controls load. No discovery,
//! publication or launch authority: every control is a manager offer.
pub mod display;
use super::offers::{self, Importance, Offer, OfferRow};
use super::product::{self, daw_guidance, installation_progress, last_load_text, product_activity,
    product_daw, product_issue, recovery_pending, vendor_install_consequence};
use super::settings::{self, settings_actions};
use super::setup::SetupControls;
use super::{record, Detail, Disclosure, Fact, ProductKey, Status, StatusClass, Text};
use crate::model::{Action, AvailableAction, CompatibilityPhase, Product, Snapshot};

pub const HEADING: &str = "Plug-ins";
pub const SHOW_ALL: &str = "Show all plug-ins";
pub const SEARCH_HINT: &str = "Search name, vendor, type or version";
pub const CLEAR_FILTERS: &str = "Clear filters";
pub const CLEAR_SEARCH: &str = "Clear search";
pub const MANAGE: &str = "Manage plug-in";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RoleFilter {
    #[default]
    All,
    Instruments,
    Effects,
}

impl RoleFilter {
    pub const CHOICES: [Self; 3] = [Self::All, Self::Instruments, Self::Effects];

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All types",
            Self::Instruments => "Instruments",
            Self::Effects => "Effects",
        }
    }

    fn matches(self, product: &Product) -> bool {
        match self {
            Self::All => true,
            Self::Instruments => product.role == "instrument",
            Self::Effects => product.role == "effect",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusFilter {
    #[default]
    All,
    Available,
    NotYetAvailable,
    NeedsAttention,
}

impl StatusFilter {
    pub const CHOICES: [Self; 4] = [Self::All, Self::Available, Self::NotYetAvailable, Self::NeedsAttention];

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All statuses",
            Self::Available => "Available in DAW",
            Self::NotYetAvailable => "Not available yet",
            Self::NeedsAttention => "Needs attention",
        }
    }

    fn matches(self, status: Status) -> bool {
        match self {
            Self::All => true,
            Self::Available => status.class == StatusClass::Ready,
            Self::NotYetAvailable => status.class == StatusClass::NeedsYou,
            Self::NeedsAttention => status.class.needs_attention(),
        }
    }
}

/// The musician's current search, filters and selection on the Plug-ins page.
#[derive(Clone, Copy, Debug)]
pub struct Query<'a> {
    pub search: &'a str,
    pub role: RoleFilter,
    pub status: StatusFilter,
    /// Exact plug-ins opened from elsewhere; search and filters do not apply.
    pub focus: Option<&'a [ProductKey]>,
}

impl Query<'_> {
    /// Exactly one plug-in is open.
    pub fn single(&self) -> bool {
        self.focus.is_some_and(|keys| keys.len() == 1)
    }

    pub fn filtered(&self) -> bool {
        !self.search.is_empty() || self.role != RoleFilter::All || self.status != StatusFilter::All
    }
}

/// Plug-ins the query selects. Opened plug-ins keep the manager's order;
/// otherwise every search term must match and results group by vendor.
pub fn matching<'a>(snapshot: &'a Snapshot, query: &Query) -> Vec<&'a Product> {
    if let Some(keys) = query.focus {
        return snapshot.products.iter()
            .filter(|product| keys.contains(&ProductKey::from(*product)))
            .collect();
    }
    let search = query.search.to_lowercase();
    let mut products: Vec<_> = snapshot.products.iter()
        .filter(|p| {
            let haystack = format!("{} {} {} {}", p.name, product::vendor(p), product::role(p), p.version)
                .to_lowercase();
            search.split_whitespace().all(|term| haystack.contains(term))
                && query.role.matches(p)
                && query.status.matches(product::status(p))
        })
        .collect();
    products.sort_by_cached_key(|p| (
        product::vendor(p).to_lowercase(),
        p.name.to_lowercase(),
        p.version.clone(),
        p.environment.clone(),
        p.class_id.clone(),
        p.module_sha256.clone(),
    ));
    products
}

/// What the screen knows besides the manager's records.
#[derive(Clone, Copy, Debug)]
pub struct Context<'a> {
    /// The shared-resource refusal from current service status, if any.
    pub busy: Option<&'a str>,
    /// A request or the manager's readback is in flight.
    pub pending: bool,
    /// Hold every disclosure open (previews and tests).
    pub expanded: bool,
    pub activity: crate::diagnostics::Section<'a, &'a crate::model::Activity>,
}

/// The full Plug-ins list, or one opened plug-in's detail.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginList {
    /// Count and rescan hint, when listing rather than showing one plug-in.
    pub summary: Vec<String>,
    pub empty: Option<(&'static str, &'static str)>,
    pub groups: Vec<VendorGroup>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VendorGroup {
    pub heading: Option<String>,
    pub cards: Vec<PluginCard>,
}

impl PluginList {
    /// `single` is whether one plug-in was open when the page was laid out.
    pub fn build(snapshot: &Snapshot, query: &Query, single: bool, context: Context) -> Self {
        let products = matching(snapshot, query);
        let summary = if single { vec![] } else { vec![
            format!("{} of {} plug-ins · grouped by vendor", products.len(), snapshot.products.len()),
            "Available plug-ins appear as native VST3s. Rescan VST3 plug-ins in your DAW if one is missing.".into(),
        ] };
        let empty = products.is_empty().then_some(if snapshot.products.is_empty() {
            ("No plug-ins discovered yet",
                "Go to Setup to open a vendor application or add a Windows installer, then rescan installed products.")
        } else {
            ("No matching plug-ins", "Try another search or clear the filters.")
        });
        let mut groups: Vec<VendorGroup> = Vec::new();
        let mut previous_vendor = String::new();
        for product in products {
            let card = PluginCard::build(snapshot, product, context);
            let group = product::vendor(product).to_lowercase();
            match groups.last_mut() {
                Some(last) if single || group == previous_vendor => last.cards.push(card),
                _ => {
                    groups.push(VendorGroup {
                        heading: (!single).then(|| product::vendor(product).to_owned()),
                        cards: vec![card],
                    });
                    previous_vendor = group;
                }
            }
        }
        Self { summary, empty, groups }
    }
}

/// One plug-in with its status, guidance and every offered control.
#[derive(Clone, Debug, PartialEq)]
pub struct PluginCard {
    pub key: ProductKey,
    /// Position in the manager's product list, for records drawn by the caller.
    /// None when the product is not in that snapshot; no other record stands in.
    pub record: Option<usize>,
    pub name: String,
    pub status: Status,
    pub facts: Vec<Fact>,
    /// Distinct observed stages; DAW publication does not prove sound or recall.
    pub readiness: Vec<ReadinessStep>,
    /// Posture, session problems and retained explanations, in reading order.
    pub notes: Vec<Text>,
    /// The one everyday next step, when there is one.
    pub lead: Option<LeadOffer>,
    pub offers: OfferRow,
    /// Exact owner offers, grouped by detached window identity.
    pub standalone: Vec<OfferRow>,
    pub recovery: Vec<crate::recovery::Recovery>,
    pub display: display::DisplayView,
    pub uninstallers: Vec<Uninstall>,
    pub settings: Option<Disclosure>,
    pub help: Option<Disclosure>,
    pub availability: Option<Disclosure>,
    pub other_setups: Option<Disclosure>,
    pub previous_setups: Option<Disclosure>,
    pub shared_setup: Option<SharedSetup>,
    /// Technical details; the caller adds the retained technical records.
    pub technical: Disclosure,
}

/// One stage in the Console readiness ladder, composed from owner observations.
#[derive(Clone, Debug, PartialEq)]
pub struct ReadinessStep {
    pub title: String,
    pub detail: String,
    pub class: StatusClass,
}

pub fn readiness(product: &Product) -> Vec<ReadinessStep> {
    let mut steps = vec![];
    let installed = product.disposition != "uninstalled" && !product.environment.is_empty();
    steps.push(ReadinessStep { title: "Installed".into(),
        detail: if installed { "Retained in its managed setup" } else { "No current installation" }.into(),
        class: if installed { StatusClass::Ready } else { StatusClass::Unknown } });
    let (class, detail) = match &product.availability {
        Some(a) if a.available => (StatusClass::Ready, "The native Linux plug-in connection is published".into()),
        Some(a) => (StatusClass::NeedsYou, a.reason.clone().unwrap_or_else(|| "Not connected yet".into())),
        None => (StatusClass::Unknown, "Connection has not been checked".into()),
    };
    steps.push(ReadinessStep { title: "In your DAW’s folder".into(), detail, class });
    if let Some(workflow) = &product.compatibility {
        for area in &workflow.established {
            steps.push(ReadinessStep { title: product::check_area_label(area).into(),
                detail: "Observed with this setup".into(), class: StatusClass::Ready });
        }
        for area in &workflow.remaining {
            steps.push(ReadinessStep { title: product::check_area_label(area).into(),
                detail: "Not yet checked".into(), class: StatusClass::Unknown });
        }
        if workflow.established.is_empty() && workflow.remaining.is_empty() {
            steps.push(ReadinessStep { title: "Tried on this computer".into(),
                detail: workflow.summary.clone(), class: StatusClass::Unknown });
        }
    } else {
        steps.push(ReadinessStep { title: "Tried on this computer".into(),
            detail: "No retained test result for this setup".into(), class: StatusClass::Unknown });
    }
    steps
}

#[derive(Clone, Debug, PartialEq)]
pub struct LeadOffer {
    /// Said before the offer, such as which plug-ins become unavailable.
    pub preface: Option<String>,
    pub offer: Offer,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Uninstall {
    pub row: OfferRow,
    pub explanation: String,
}

/// The plug-in's shared setup controls, the same ones Setup shows for it.
#[derive(Clone, Debug, PartialEq)]
pub struct SharedSetup {
    pub title: &'static str,
    pub start_open: bool,
    pub controls: SetupControls,
}

impl PluginCard {
    pub fn build(snapshot: &Snapshot, p: &Product, context: Context) -> Self {
        let Context { busy, pending, expanded, activity } = context;
        let status = product::status(p);
        let related = related_actions(p, snapshot);
        let progress = installation_progress(snapshot, p);
        let display_actions = display::references(p);
        let primary = p.compatibility.as_ref().and_then(|workflow| workflow.primary.clone())
            .or_else(|| p.compatibility.is_none().then(||
                emphasized_action(p, &related, busy)).flatten())
            .filter(|offer| !display_actions.contains(&offer.action) && !standalone_action(&offer.action) && !matches!(offer.action, Action::PluginRestart {..}))
            .map(|mut offer| {
                if recovery_pending(p) && matches!(offer.action, Action::CompatibilityFinishResult { .. }) {
                    offer.label = "Finish plug-in recovery".into();
                }
                offer
            });
        let optional_result = primary.as_ref().filter(|offer| optional_test_result(p, &offer.action));
        let optional_preparation = primary.as_ref().filter(|offer|
            (status.class == StatusClass::Ready || progress.is_some()) && other_setup_option(&offer.action));
        let everyday_primary = primary.clone().filter(|_| optional_result.is_none()
            && optional_preparation.is_none());
        let management: Vec<_> = ordinary_actions(p, &related, primary.as_ref()).into_iter()
            .filter(|offer| !display_actions.contains(&offer.action) && !matches!(offer.action,Action::PluginRestart {..})).collect();

        let mut notes = Vec::new();
        if recovery_pending(p) {
            if let Some(workflow) = &p.compatibility {
                notes.push(Text::caution(&workflow.summary));
            }
        } else if status.class == StatusClass::Ready {
            notes.push(Text::body(daw_guidance(product_daw(snapshot, p))));
            if let Some(workflow) = p.compatibility.as_ref().filter(|workflow|
                workflow.phase == CompatibilityPhase::NeedsWork) {
                notes.push(Text::caution(&workflow.summary));
            }
        } else if matches!(p.disposition.as_str(), "removed_from_daw" | "uninstalled") {
            if let Some(reason) = p.availability.as_ref().and_then(|a| a.reason.as_ref()) {
                notes.push(Text::body(reason));
            }
        } else if let Some(workflow) = p.compatibility.as_ref().filter(|_| progress.is_none()) {
            notes.push(Text::body(if workflow.phase == CompatibilityPhase::ReadyForTest {
                "The native bridge is prepared. Make it available below, then choose it in your DAW."
            } else { workflow.summary.as_str() }));
        }
        let instances = product_activity(activity, p);
        notes.push(Text::detail(instances.detail_label()));
        notes.extend(instances.lines.iter().map(Text::body));
        for failure in &instances.failed_live {
            notes.push(Text::caution(format!("Live session: {failure}")));
        }
        if let Some(failure) = instances.recent_failure {
            notes.push(Text::caution(format!("Most recent session: {failure}")));
            notes.push(Text::detail(if instances.recent_cleanup_unconfirmed {
                "Cleanup remains unconfirmed."
            } else {
                "Cleanup and transport retirement confirmed."
            }));
        }
        if let Some(reason) = product_issue(p)
            .filter(|_| progress.is_none() && status.class.needs_attention()) {
            notes.push(Text::caution(reason));
        }
        if let Some(refused) = last_load_text(p) {
            notes.push(Text::caution(refused));
        }
        for limit in &p.limitations {
            let words = limit.replace('_', " ");
            if !notes.iter().any(|note| note.words == words) {
                notes.push(Text::caution(words));
            }
        }

        let lead = everyday_primary.as_ref().map(|action| LeadOffer {
            preface: vendor_install_consequence(snapshot, &action.action),
            offer: offers::offer(action, Importance::Primary, busy, pending),
        });
        let offered = management.iter().chain(p.compatibility.iter()
            .flat_map(|workflow| workflow.alternatives.iter())).chain(optional_preparation).filter(|offer| !display_actions.contains(&offer.action));
        let (previous, current): (Vec<_>, Vec<_>) = offered.cloned()
            .partition(|offer| matches!(offer.action,
                Action::OrdinaryRollback { .. } | Action::OrdinaryRestoreRecommended { .. }));
        let (availability, current): (Vec<_>, Vec<_>) = current.into_iter()
            .partition(|offer| matches!(offer.action, Action::ExperimentalDisable { .. } | Action::CandidateWithdraw { .. }));
        let (options, current): (Vec<_>, Vec<_>) = current.into_iter()
            .filter(|offer| !everyday_primary.as_ref().is_some_and(|shown| shown.action == offer.action))
            .partition(|offer| other_setup_option(&offer.action));
        let mut everyday = offers::row(&current, Importance::Secondary, busy, pending);
        everyday.preface = current.iter().find_map(|offer|
            vendor_install_consequence(snapshot, &offer.action));

        // Uninstall through the plug-in's own setup.
        let uninstallers = snapshot.uninstallers.iter().filter(|u| u.environment == p.environment
            && u.affected_classes.iter().any(|class| class.eq_ignore_ascii_case(&p.class_id)))
            .map(|uninstaller| Uninstall {
                row: offers::row(std::slice::from_ref(&uninstaller.action), Importance::Secondary, busy, pending),
                explanation: if uninstaller.coverage == "installed_with_this" {
                    format!("Takes it out of your DAW, then runs {}'s own uninstaller.", uninstaller.name)
                } else {
                    format!("Takes every plug-in of this setup out of your DAW, then runs {}'s own uninstaller.", uninstaller.name)
                },
            }).collect();

        let editor_checks: Vec<_> = settings_actions(p).into_iter()
            .filter(|offer| matches!(offer.action, Action::CandidateGraphicsAssess { .. }) && !display_actions.contains(&offer.action)).collect();
        let help = (p.compatibility.is_some() || !editor_checks.is_empty()).then(|| {
            let mut content = Vec::new();
            if let Some(offer) = optional_result {
                content.push(Detail::Offers(offers::row(std::slice::from_ref(offer), Importance::Quiet, busy, pending)));
            }
            if !editor_checks.is_empty() {
                content.push(Detail::Offers(offers::row(&editor_checks, Importance::Quiet, busy, pending)));
            }
            if let Some(workflow) = &p.compatibility {
                content.push(Text::body(if workflow.phase == CompatibilityPhase::AvailableForTest {
                    "Available in your DAW using this setup. You can record a result after trying it."
                } else { workflow.summary.as_str() }).into());
                content.extend(observed(&workflow.established));
                if !workflow.remaining.is_empty() {
                    content.push(Text::detail(format!("Not yet checked: {}", workflow.remaining.iter()
                        .map(|area| product::check_area_label(area)).collect::<Vec<_>>().join(", "))).into());
                }
            }
            content.push(Text::detail("Recorded checks describe only what was observed with that setup. They do not replace your vendor's authorization or your DAW's project save and reopen.").into());
            let configuration = &p.details["configuration"];
            let recorded = if configuration.is_null() { &p.details["graphics"] } else { configuration };
            let mut editor = settings::assessment("Configuration assessment", &recorded["assessment"]);
            editor.push(Text::detail("This supervised editor check does not establish uninterrupted audio, saved-project recall or a known fix.").into());
            content.push(Detail::Disclosure(Disclosure::new("Editor check details", editor).forced_open(expanded)));
            Disclosure::new("Help and testing", content).forced_open(expanded)
        });
        let availability = (!availability.is_empty()).then(|| Disclosure::new("DAW availability",
            vec![Detail::Offers(offers::row(&availability, Importance::Quiet, busy, pending))]).forced_open(expanded));
        let other_setups = (!options.is_empty()).then(|| {
            let mut content = vec![Text::body("Prepare an alternative or select a retained setup. Preparing one keeps your current DAW selection.").into()];
            content.extend(options.iter().map(|offer| Detail::Choice(Box::new(offers::setup_choice(p, offer, busy, pending)))));
            Disclosure { id_salt: Some("other-setup-options"), ..Disclosure::new(
                format!("Other setup options ({})", options.len()), content).forced_open(expanded) }
        });
        let previous_setups = (!previous.is_empty()).then(|| Disclosure {
            id_salt: Some("previous-setups"),
            ..Disclosure::new(format!("Previous setups ({})", previous.len()), previous.iter()
                .map(|offer| Detail::Choice(Box::new(offers::setup_choice(p, offer, busy, pending)))).collect())
                .forced_open(expanded)
        });
        let shared_setup = snapshot.environment_installers.iter()
            .find(|target| target.environment == p.environment)
            .map(|target| SharedSetup {
                title: "Manage shared setup",
                start_open: target.actions.iter().any(|offer|
                    matches!(offer.action, Action::EnvironmentInstallerStop { .. })),
                controls: SetupControls::build(target, pending),
            });

        let expert: Vec<_> = p.actions.iter().chain(related.iter())
            .filter(|action| {
                !matches!(action.action, Action::BufferingSet { .. } | Action::DeliverySet { .. }
                    | Action::CandidateGraphicsAssess { .. }
                    | Action::CandidateGraphicsPrepare { .. }
                    | Action::CandidateSettingsPrepare { .. })
                    && !standalone_action(&action.action) && !matches!(action.action,Action::PluginRestart {..}) && !display_actions.contains(&action.action)
                    && !management.iter().any(|shown| shown.action == action.action)
                    && !guided_action_shown(p, primary.as_ref(), &action.action)
            })
            .cloned()
            .collect();
        let mut technical = vec![Text::detail(instances.detail_label()).into()];
        if !expert.is_empty() {
            technical.push(Detail::Offers(offers::row(&expert, Importance::Quiet, busy, pending)));
        }
        if let Some(workflow) = &p.compatibility {
            technical.push(Text::body(&workflow.summary).into());
            technical.extend(observed(&workflow.established));
        }

        let standalone = standalone_controls(p, everyday_primary.as_ref(), busy, pending);
        let display = display::build(p, busy, pending);
        Self {
            key: ProductKey::from(p),
            record: snapshot.products.iter().position(|candidate| std::ptr::eq(candidate, p)),
            name: p.name.clone(),
            status: progress.map(|words| Status { words, class: StatusClass::Working }).unwrap_or(status),
            facts: vec![
                Fact { label: "Vendor", value: Some(product::vendor(p).into()), missing: "Vendor unavailable" },
                Fact { label: "Type", value: Some(product::role(p).into()), missing: "Type unavailable" },
                Fact { label: "Version", value: (!p.version.trim().is_empty()).then(|| p.version.clone()),
                    missing: "Version unavailable" },
            ],
            readiness: readiness(p),
            notes,
            recovery: crate::recovery::for_product(activity, p, busy, pending),
            standalone,
            display,
            lead,
            offers: everyday,
            uninstallers,
            settings: settings::section_omitting(p, busy, pending, expanded, &display_actions),
            help,
            availability,
            other_setups,
            previous_setups,
            shared_setup,
            technical: Disclosure::new(if p.compatibility.is_some() {
                "Technical details and expert actions"
            } else { "Technical details and advanced actions" }, technical).forced_open(expanded),
        }
    }
}

fn standalone_action(action: &Action) -> bool {
    matches!(action, Action::PluginOpen { .. } | Action::PluginFocus { .. } | Action::PluginClose { .. })
}

/// Group only exact owner-issued window controls. A pair never borrows another
/// window's focus/close target, even when their labels and product are equal.
fn standalone_controls(product: &Product, primary: Option<&AvailableAction>, busy: Option<&str>, pending: bool) -> Vec<OfferRow> {
    struct Group<'a> {
        identity: Option<(&'a str, &'a str)>,
        actions: Vec<AvailableAction>,
    }
    let mut groups: Vec<Group<'_>> = vec![];
    for offer in product.actions.iter().filter(|offer| standalone_action(&offer.action)
        && !primary.is_some_and(|shown| shown.action == offer.action)) {
        let identity = match &offer.action {
            Action::PluginFocus { operation, session } | Action::PluginClose { operation, session } =>
                Some((operation.as_str(), session.as_str())),
            _ => None,
        };
        if let Some(group) = groups.iter_mut().find(|group| group.identity == identity) {
            if !group.actions.iter().any(|old| old.action == offer.action) { group.actions.push(offer.clone()); }
        } else { groups.push(Group { identity, actions: vec![offer.clone()] }); }
    }
    groups.into_iter().map(|group| {
        let mut row = offers::row(&group.actions, Importance::Secondary, busy, pending);
        if primary.is_none() {
            for offer in &mut row.offers {
                if matches!(offer.intent, Action::PluginOpen { .. } | Action::PluginFocus { .. }) {
                    offer.importance = Importance::Primary;
                }
            }
        }
        row
    }).collect()
}

fn observed(established: &[String]) -> impl Iterator<Item = Detail> + '_ {
    established.iter().map(|area| Text::detail(format!("Observed: {area}")).into())
}

/// The read-only list shown while a plug-in's current controls load. It
/// carries no action authority; it only opens an exact plug-in.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadingList {
    /// Shown above the list when plug-ins were opened from elsewhere.
    pub focus_title: Option<&'static str>,
    pub notice: Text,
    /// Offered when loading the controls failed.
    pub retry: Option<&'static str>,
    pub count: String,
    pub empty: Option<&'static str>,
    pub rows: Vec<LoadingRow>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadingRow {
    pub key: ProductKey,
    pub name: String,
    pub facts: Vec<Fact>,
    /// Current availability; withheld when status could not be loaded.
    pub status: Option<&'static str>,
    pub status_class: StatusClass,
    pub notes: Vec<Text>,
    /// Opens this exact plug-in, when several are listed.
    pub manage: Option<&'static str>,
}

impl LoadingList {
    pub fn build(snapshot: &Snapshot, query: &Query, fresh: bool, details_attempted: bool) -> Self {
        let single = query.single();
        let (notice, retry) = if !fresh {
            (Text::caution("Current plug-in status could not be loaded. Try loading it again before continuing."), Some("Load plug-in controls again"))
        } else if details_attempted {
            (Text::caution("Plug-in controls could not be loaded. The last plug-in summary is shown below."),
                Some("Load plug-in controls again"))
        } else {
            (Text::body(if single {
                "Loading this plug-in's setup and settings…"
            } else {
                "Open a plug-in to manage its setup, updates and settings."
            }), None)
        };
        let products = matching(snapshot, query);
        let empty = products.is_empty().then_some(if query.focus.is_some() {
            "The selected plug-in is missing from the current list. Refresh before continuing."
        } else if snapshot.products.is_empty() {
            "No plug-ins discovered yet. Open Setup to add and scan an installer."
        } else {
            "No matching plug-ins. Clear the search to see all current products."
        });
        Self {
            focus_title: query.focus.map(|_| if single {
                "Showing the selected plug-in"
            } else {
                "Showing the selected plug-ins"
            }),
            notice,
            retry,
            count: format!("{} of {} current plug-ins", products.len(), snapshot.products.len()),
            empty,
            rows: products.into_iter().map(|product| {
                let mut notes = Vec::new();
                if let Some(refused) = last_load_text(product) {
                    notes.push(Text::caution(refused));
                }
                notes.push(match product::problem_text(&product.details["problem"]) {
                    Some(problem) => Text::caution(problem),
                    None => Text::detail("Setup actions and settings appear after the current controls load."),
                });
                LoadingRow {
                    key: ProductKey::from(product),
                    name: product.name.clone(),
                    facts: vec![
                        Fact { label: "Vendor", value: (!product.vendor.trim().is_empty())
                            .then(|| product::vendor(product).into()), missing: "Unknown vendor" },
                        Fact { label: "Type", value: Some(product::role(product).into()), missing: "Type unavailable" },
                        Fact { label: "Version", value: (!product.version.is_empty()).then(|| product.version.clone()),
                            missing: "Version unavailable" },
                    ],
                    status: fresh.then(|| installation_progress(snapshot, product)
                        .unwrap_or_else(|| product::status(product).words)),
                    status_class: if !fresh { StatusClass::Unknown }
                        else if installation_progress(snapshot, product).is_some() { StatusClass::Working }
                        else { product::status(product).class },
                    notes,
                    // A Setup discovery may focus several exact classes. Each
                    // row must still let the user select one current detail.
                    manage: (!single).then_some(MANAGE),
                }
            }).collect(),
        }
    }
}

/// The retained technical records under a plug-in's technical details: its
/// preparation, published revision and history. Supplied limitations are
/// musician-facing notes on the card, not hidden technical records.
pub fn technical_records(product: &Product) -> Vec<Detail> {
    let mut details = Vec::new();
    if let Some(preparation) = product.details.get("preparation") {
        details.extend(preparation_records(preparation));
    }
    if let Some(revision) = product.active_revision {
        details.push(Text::body(format!("Published revision: {revision} · Recommended: {}",
            product.recommended_revision.map(|value| value.to_string()).unwrap_or_else(|| "none".into()))).into());
    }
    for history in &product.history {
        details.push(Text::body(format!("{} · {}{}", history.description, history.claim,
            if history.active { " · active" } else { "" })).into());
        if let Some(reason) = &history.rollback_unavailable { details.push(Text::body(reason).into()); }
    }
    details.push(Detail::Disclosure(Disclosure::new("Technical product details", vec![
        Text::body(format!("Status: {}\nClass: {}\nModule SHA-256: {}\nEnvironment: {}\nRunner: {}",
            product.disposition, product.class_id, product.module_sha256, product.environment, product.runner)).into(),
        record(&product.details),
    ])));
    details
}

/// How a plug-in was inspected and prepared, and what each check recorded.
fn preparation_records(v: &serde_json::Value) -> Vec<Detail> {
    let mut details: Vec<Detail> = Vec::new();
    if v["publication"] == "ordinary" { details.push(Text::strong("Ordinarily published").into()); }
    if let Some(history) = v["candidates"].as_array() {
        details.push(Detail::Disclosure(Disclosure { start_open: history.len() > 1, ..Disclosure::new(
            "Candidate generations and their own results", history.iter().map(|candidate| {
                let mut group = vec![Text::body(format!("Candidate {} · {} · {}", candidate["id"].as_str().unwrap_or("?"),
                    candidate["disposition"].as_str().unwrap_or("?"), candidate["publication"].as_str().unwrap_or("?"))).into()];
                if candidate["publication_acceptance_sealed"] == true
                    && candidate["unmet_requirements"].as_array().is_some_and(|rows| !rows.is_empty()) {
                    group.push(Text::alert("This publication retains its original acceptance. New evidence needs review; use Withdraw to disable it.").into());
                }
                group.push(record(candidate));
                Detail::Group(group)
            }).collect()) }));
    }
    if let Some(reason) = v["recovery"].as_str() { details.push(Text::alert(reason).into()); }
    if let Some(reason) = v["build_prerequisite"].as_str() { details.push(Text::body(reason).into()); }
    details.push(Detail::Disclosure(Disclosure::new("Inspection generations", vec![record(&v["inspections"])])));
    details.push(Text::body(format!("Inspection: {} · Preparation: {} · Publication: {}",
        v["inspection"].as_str().unwrap_or("unknown"), v["preparation"].as_str().unwrap_or("unknown"),
        v["publication"].as_str().unwrap_or("unknown"))).into());
    details.push(Text::detail("Preliminary inspection is not a guarantee of DAW audio, editor, automation, or state recall.").into());
    if v["origin"] == "retained_sv1" {
        details.push(Text::detail("Adopted exact SV1 work; originally prepared through the engineering CLI.").into());
    }
    if let Some(reason) = v["controller"]["reason"].as_str() { details.push(Text::detail(reason).into()); }
    if !v["preliminary"].is_null() {
        let f = &v["preliminary"];
        details.push(Detail::Disclosure(Disclosure::new("Preliminary compatibility details", vec![
            Text::body(format!("Parameters: {} · Positive inspection cleanup: {}", f["parameter_count"], f["cleanup_confirmed"])).into(),
            Text::body(if f["editor_interface"].is_null() { "Editor interface: no assessment recorded".into() } else {
                format!("Editor created: {} · HWND interface result: {} · Not attached",
                    f["editor_interface"]["created"], f["editor_interface"]["hwnd_result"]) }).into(),
            Text::code(serde_json::to_string_pretty(f).unwrap_or_default()).into(),
        ])));
    }
    if let Some(operation) = v.get("operation").filter(|operation| !operation.is_null()) {
        details.push(Text::body(format!("This product's operation: {}", operation["state"].as_str().unwrap_or("unknown"))).into());
        if let Some(stage) = operation["preparation_failure"]["attempted_stage"].as_str() {
            details.push(Text::body(format!("Stopped during {}", stage.replace('_', " "))).into());
        }
        if let Some(reason) = operation["reason"].as_str() { details.push(Text::alert(reason).into()); }
        if let Some(recovery) = operation["preparation_failure"]["recovery"].as_str() {
            details.push(Text::detail(recovery).into());
        }
    }
    let mut tests: Vec<Detail> = Vec::new();
    for area in ["daw_load", "midi", "audio", "editor", "parameters", "automation", "state_recall",
        "processing_restart", "retirement"] {
        let latest = v["evidence"].as_array().and_then(|rows| rows.iter().rev().find(|row| row["area"] == area));
        tests.push(Text::body(format!("{}: {}", area.replace('_', " "),
            latest.and_then(|row| row["status"].as_str()).unwrap_or("not tested"))).into());
        if let Some(row) = latest {
            tests.push(Text::detail(format!("{} · {}", row["witness"].as_str().unwrap_or("unavailable"),
                row["detail"].as_str().unwrap_or(""))).into());
        }
    }
    if let Some(rows) = v["unmet_requirements"].as_array() {
        tests.extend(rows.iter().filter_map(|row| row.as_str()).map(|text| Text::detail(text).into()));
    }
    details.push(Detail::Disclosure(Disclosure::new("Recorded tests and remaining qualification", tests)));
    details
}

fn optional_test_result(product: &Product, action: &Action) -> bool {
    matches!(action, Action::CompatibilityResult { .. })
        || (matches!(action, Action::CompatibilityFinishResult { .. }) && !recovery_pending(product))
}

fn other_setup_option(action: &Action) -> bool {
    matches!(action,Action::CompatibilityCheck { .. } | Action::CompatibilityPublishTest { .. }
        |Action::PluginInspect { .. } |Action::PluginReinspect { .. } |Action::PluginPrepare { .. }
        |Action::ExperimentalReplace { .. } |Action::ExperimentalEnable { .. }
        |Action::CandidateSettingsPrepare { .. } |Action::CandidateGraphicsPrepare { .. })
}

/// Show only manager-offered actions. Visibility supplies no new authority;
/// disabled reasons, freshness and exact request validation still apply.
fn ordinary_actions(product:&Product, related:&[AvailableAction],
    primary:Option<&AvailableAction>) -> Vec<AvailableAction> {
    let installed_publication=product.active_revision.is_some()
        || matches!(product.disposition.as_str(),"ready"|"experimental"|"another_configuration");
    let mut actions=Vec::new();
    for offer in product.actions.iter().chain(related) {
        let show=match offer.action {
            Action::PluginReinspect{..}|Action::PluginPrepare{..} => installed_publication,
            Action::ExperimentalReplace{..}|Action::OrdinaryRollback{..}
                |Action::PluginRemoveFromDaw{..}|Action::PluginAddBackToDaw{..}
                |Action::OrdinaryRestoreRecommended{..}|Action::ExperimentalDisable{..}
                |Action::CandidateWithdraw{..}|Action::QuarantinedModuleRetry{..}
                |Action::PluginFinishSetup{..}|Action::PluginSetupFocus{..}|Action::PluginSetupStop{..}
                |Action::EnvironmentRescan{..}|Action::VendorApplicationFocus{..}
                |Action::VendorSoftwareInstall{..}|Action::VendorApplicationOpen{..}|Action::RendererOpen{..}
                |Action::RendererFocus{..} => true,
            _ => false,
        };
        if show && !guided_action_shown(product, primary, &offer.action)
            && !actions.iter().any(|shown:&AvailableAction|shown.action==offer.action) {
            actions.push(offer.clone());
        }
    }
    actions
}

fn guided_action_shown(product: &Product, primary: Option<&AvailableAction>, action: &Action) -> bool {
    primary.into_iter().chain(product.compatibility.iter()
        .flat_map(|workflow| workflow.alternatives.iter())).any(|shown| {
        shown.action == *action || matches!((action, &shown.action),
            (Action::ExperimentalReplace {candidate, expected_current},
                Action::CompatibilityPublishTest {candidate: shown_candidate,
                    expected_current: Some(shown_current)})
            if candidate == shown_candidate && expected_current == shown_current)
    })
}

fn routine_rank(action: &Action) -> Option<u8> {
    match action {
        Action::PluginFinishSetup {..}|Action::PluginSetupFocus {..} => Some(0),
        Action::QuarantinedModuleRetry { .. } | Action::PluginAddBackToDaw { .. } => Some(1),
        Action::PluginReinspect { .. } => Some(1),
        Action::PluginInspect { .. } => Some(2),
        Action::PluginPrepare { .. } => Some(3),
        Action::ExperimentalEnable { .. } => Some(4),
        Action::OrdinaryRestoreRecommended { .. } => Some(5),
        Action::EnvironmentRescan { .. } => Some(6),
        Action::VendorApplicationFocus { .. } | Action::RendererFocus { .. } => Some(7),
        Action::VendorSoftwareInstall { .. } | Action::VendorApplicationOpen { .. } | Action::RendererOpen { .. } => Some(8),
        _ => None,
    }
}

/// The one enabled routine offer to lead with, when the manager has no
/// guided next step for this plug-in.
fn emphasized_action(
    product: &Product,
    related: &[AvailableAction],
    _busy: Option<&str>,
) -> Option<AvailableAction> {
    let published = matches!(product.disposition.as_str(), "ready" | "experimental");
    product
        .actions
        .iter()
        .chain(related)
        .filter(|offer| {
            offer.disabled_reason.is_none()
        })
        .filter_map(|offer| {
            routine_rank(&offer.action).map(|rank| {
                // A published plug-in does not lead with housekeeping when its
                // manager-offered vendor application is available.
                let rank = if published {
                    match offer.action {
                        Action::VendorApplicationFocus { .. } | Action::RendererFocus { .. } => 5,
                        Action::VendorSoftwareInstall { .. } | Action::VendorApplicationOpen { .. } | Action::RendererOpen { .. } => 6,
                        Action::EnvironmentRescan { .. } => 8,
                        _ => rank,
                    }
                } else {
                    rank
                };
                (rank, offer)
            })
        })
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, offer)| offer.clone())
}

/// Offers from this plug-in's own setup: its vendor application and rescan.
fn related_actions(product: &Product, snapshot: &Snapshot) -> Vec<AvailableAction> {
    if product.environment.is_empty() {
        return vec![];
    }
    let mut actions = vec![];
    // Association is supplied by the manager's exact environment identity, never vendor/name heuristics.
    for app in &snapshot.vendor_applications {
        if app.details["environment"].as_str() != Some(product.environment.as_str()) {
            continue;
        }
        actions.extend(app.actions.iter().filter(|offer| offer.disabled_reason.is_none() && matches!(offer.action,
            Action::VendorSoftwareInstall { .. } | Action::VendorApplicationOpen { .. } | Action::VendorApplicationFocus { .. }
                | Action::RendererOpen { .. } | Action::RendererFocus { .. }))
            .map(|offer| {
                let mut named = offer.clone();
                if !app.name.trim().is_empty() && !matches!(offer.action, Action::VendorSoftwareInstall {..}) {
                    named.label = format!("{} {}", if matches!(offer.action,
                        Action::VendorApplicationOpen { .. } | Action::RendererOpen { .. }) {
                        "Open" } else { "Show" }, app.name);
                }
                named
            }));
    }
    for environment in &snapshot.environments {
        if environment.id == product.environment {
            actions.extend(
                environment
                    .actions
                    .iter()
                    .filter(|a| {
                        matches!(&a.action,
                Action::EnvironmentRescan { environment } if environment == &product.environment)
                    })
                    .cloned(),
            );
        }
    }
    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CompatibilityWorkflow, NativeAvailability};

    fn snapshot() -> Snapshot {
        crate::samples::base()
    }

    fn query(search: &str) -> Query<'_> {
        Query { search, role: RoleFilter::All, status: StatusFilter::All, focus: None }
    }

    #[test]
    fn search_combines_terms_and_filters_without_losing_unknown_states() {
        let mut snapshot = snapshot();
        snapshot.products[0].disposition = "future_state".into();
        snapshot.products[0].availability=None;
        assert_eq!(matching(&snapshot, &query("")).len(), 3);
        assert_eq!(matching(&snapshot, &query(""))[0].vendor, "Arturia");
        let mut search = query("  INSTRUMENT   8.13 native ");
        assert_eq!(matching(&snapshot, &search)[0].name, "Kontakt 8");
        search.status = StatusFilter::NeedsAttention;
        assert_eq!(matching(&snapshot, &search).len(), 1);
        search.role = RoleFilter::Effects;
        assert!(matching(&snapshot, &search).is_empty());
    }

    #[test]
    fn opened_plug_ins_ignore_search_and_keep_the_manager_order() {
        let snapshot = snapshot();
        let keys = [ProductKey::from(&snapshot.products[2]), ProductKey::from(&snapshot.products[0])];
        let mut opened = query("nothing matches this");
        opened.focus = Some(&keys);
        let names: Vec<_> = matching(&snapshot, &opened).iter().map(|p| p.name.clone()).collect();
        assert_eq!(names, [snapshot.products[0].name.clone(), snapshot.products[2].name.clone()]);
        assert!(!opened.single());
    }

    #[test]
    fn related_actions_hide_disabled_app_rows_and_preserve_owning_card_refusals() {
        let mut s = snapshot();
        assert_eq!(related_actions(&s.products[0], &s).len(), 2);
        s.vendor_applications[1].actions[0].disabled_reason =
            Some("Owned session is running".into());
        let owning_card = serde_json::to_value(&s.vendor_applications[1]).unwrap();
        let actions = related_actions(&s.products[0], &s);
        assert_eq!(actions.len(), 1); // The exact setup's rescan stays visible.
        assert!(matches!(actions[0].action, Action::EnvironmentRescan {..}));
        // Its own setup still explains the app's status and offers recovery;
        // only a duplicate disabled Open row on another card disappears.
        assert_eq!(serde_json::to_value(&s.vendor_applications[1]).unwrap(), owning_card);
        assert_eq!(s.vendor_applications[1].actions[0].disabled_reason.as_deref(),
            Some("Owned session is running"));
        s.products[0].environment = "different-installation-same-vendor".into();
        assert!(related_actions(&s.products[0], &s).is_empty());
        s.vendor_applications[1].details = serde_json::json!({});
        s.products[0].environment = "fixture-ni".into();
        assert_eq!(related_actions(&s.products[0], &s).len(), 1); // only the exact rescan
    }

    #[test]
    fn one_enabled_routine_action_is_emphasized_without_changing_offers() {
        let mut s = snapshot();
        let related = related_actions(&s.products[2], &s);
        let product = &mut s.products[2];
        let retry = AvailableAction {
            label: "Retry exact scan".into(),
            action: Action::PluginReinspect {
                selection: "exact".into(),
                audio_layout: None,
            },
            disabled_reason: Some("Current manager work blocks this scan".into()),
        };
        product.actions.push(retry.clone());
        let enabled_related = emphasized_action(product, &related, Some("Busy")).unwrap();
        assert_ne!(enabled_related.action, retry.action);
        assert_eq!(enabled_related.disabled_reason, None);
        assert_eq!(product.actions.len(), 1);
        assert_eq!(product.actions[0].action, retry.action);
        product.actions[0].disabled_reason = None;
        let selected = emphasized_action(product, &related, Some("Unrelated setup is busy")).unwrap();
        assert_eq!(selected.action, retry.action);
        assert_eq!(product.actions.len(), 1);
        assert_eq!(selected.disabled_reason, None);
    }

    #[test]
    fn published_product_prefers_offered_vendor_application_over_rescan() {
        let s = snapshot();
        let product = &s.products[1];
        assert_eq!(product.disposition, "ready");
        let related = related_actions(product, &s);
        let chosen = emphasized_action(product, &related, None).unwrap();
        assert!(matches!(
            chosen.action,
            Action::VendorApplicationOpen { .. } | Action::VendorApplicationFocus { .. }
        ));
        assert!(related
            .iter()
            .any(|offer| matches!(offer.action, Action::EnvironmentRescan { .. })));
    }

    #[test]
    fn recovery_finish_leads_under_its_own_name_and_keeps_its_refusal() {
        let mut s = snapshot();
        s.products.truncate(1);
        s.products[0].compatibility = Some(CompatibilityWorkflow {
            phase: CompatibilityPhase::AwaitingRetirement,
            summary: "Problem recorded. Close the DAW, then finish this result.".into(),
            established: vec![], remaining: vec![], current_inspection: None, current_candidate: None,
            primary: Some(AvailableAction { label: "Finish recording test result".into(),
                action: Action::CompatibilityFinishResult { operation: "dd".repeat(16) },
                disabled_reason: Some("Cleanup unconfirmed".into()) }),
            alternatives: vec![],
        });
        let card = PluginCard::build(&s, &s.products[0],
            Context { activity: crate::diagnostics::Section::NotRequested, busy: Some("Cleanup unconfirmed"), pending: false, expanded: false });
        let lead = card.lead.expect("recovery leads");
        assert_eq!(lead.offer.label, "Finish plug-in recovery");
        assert_eq!(lead.offer.availability, offers::Availability::Refused("Cleanup unconfirmed".into()));
        assert_eq!(lead.offer.intent, Action::CompatibilityFinishResult { operation: "dd".repeat(16) });
        assert_eq!(card.notes[0], Text::caution("Problem recorded. Close the DAW, then finish this result."));
        assert!(!card.notes.iter().any(|note| note.words == daw_guidance(None)));
    }

    #[test]
    fn removed_plug_in_reads_as_removed_and_leads_with_add_back() {
        let mut s = snapshot();
        s.products.truncate(1);
        s.environments.clear();
        s.vendor_applications.clear();
        let removed = "Removed from your DAW. Projects that use it open with it missing until you add it back.";
        let class = s.products[0].class_id.clone();
        let product = &mut s.products[0];
        product.compatibility = None;
        product.disposition = "removed_from_daw".into();
        product.availability = Some(NativeAvailability { available: false, reason: Some(removed.into()) });
        product.actions = vec![AvailableAction { label: "Add back to my DAW".into(),
            action: Action::PluginAddBackToDaw { class_id: class,
                expected_removed: crate::model::PublicationIdentity { id: "ac".repeat(16), sha256: "bd".repeat(32) } },
            disabled_reason: None }];
        let card = PluginCard::build(&s, &s.products[0], Context { activity: crate::diagnostics::Section::NotRequested, busy: None, pending: false, expanded: false });
        assert_eq!(card.status, Status { words: "Removed from your DAW", class: StatusClass::NeedsYou });
        assert_eq!(card.notes, [Text::body(removed), Text::detail("Live activity has not been observed")]);
        assert_eq!(card.lead.unwrap().offer.label, "Add back to my DAW");
        assert!(card.offers.offers.is_empty());
    }

    use crate::window::Page;
    use crate::samples::{self, STATES};
    use offers::Availability;

    fn sample(page: Page, state: &str) -> Snapshot {
        samples::snapshot(page, Some(state))
    }

    fn context(snapshot: &Snapshot, pending: bool, expanded: bool) -> Context<'_> {
        Context { activity: crate::diagnostics::Section::NotRequested, busy: snapshot.system.inactive_reason(), pending, expanded }
    }

    fn card(snapshot: &Snapshot, index: usize, pending: bool, expanded: bool) -> PluginCard {
        PluginCard::build(snapshot, &snapshot.products[index], context(snapshot, pending, expanded))
    }

    /// Every offer on a card, with the notes printed beside it.
    fn offers_with_notes(card: &PluginCard) -> Vec<(&Offer, Vec<&str>)> {
        fn details<'a>(content: &'a [Detail], out: &mut Vec<(&'a Offer, Vec<&'a str>)>) {
            for detail in content {
                match detail {
                    Detail::Offers(row) | Detail::Held { row, .. } => out.extend(row.offers.iter()
                        .map(|offer| (offer, row.notes.iter().map(String::as_str).collect()))),
                    Detail::Choice(choice) => out.push((&choice.offer,
                        choice.notes.iter().map(String::as_str).collect())),
                    Detail::Disclosure(inner) => details(&inner.content, out),
                    Detail::Group(inner) => details(inner, out),
                    Detail::Text(_) | Detail::Divider => {}
                }
            }
        }
        let mut out = Vec::new();
        if let Some(lead) = &card.lead {
            out.push((&lead.offer, lead.offer.availability.reason().into_iter().collect()));
        }
        for row in std::iter::once(&card.offers).chain(card.uninstallers.iter().map(|u| &u.row)) {
            out.extend(row.offers.iter().map(|offer| (offer, row.notes.iter().map(String::as_str).collect())));
        }
        for section in [&card.settings, &card.help, &card.availability, &card.other_setups,
            &card.previous_setups].into_iter().flatten().chain([&card.technical]) {
            details(&section.content, &mut out);
        }
        out
    }

    /// Every action the manager offers anywhere in these records.
    fn manager_offers(snapshot: &Snapshot) -> Vec<&Action> {
        let mut offered: Vec<&Action> = Vec::new();
        for product in &snapshot.products {
            offered.extend(product.actions.iter().map(|offer| &offer.action));
            if let Some(workflow) = &product.compatibility {
                offered.extend(workflow.primary.iter().chain(&workflow.alternatives).map(|offer| &offer.action));
            }
        }
        offered.extend(snapshot.vendor_applications.iter().flat_map(|app| &app.actions).map(|offer| &offer.action));
        offered.extend(snapshot.environments.iter().flat_map(|e| &e.actions).map(|offer| &offer.action));
        offered.extend(snapshot.uninstallers.iter().map(|u| &u.action.action));
        offered
    }

    #[test]
    fn sample_states_offer_only_manager_actions_with_every_refusal_visible() {
        let mut checked = 0;
        for state in STATES {
            for page in [Page::Plugins, Page::Setup] {
                let snapshot = sample(page, state);
                let offered = manager_offers(&snapshot);
                for pending in [false, true] {
                    for index in 0..snapshot.products.len() {
                        let card = card(&snapshot, index, pending, true);
                        for (offer, notes) in offers_with_notes(&card) {
                            assert!(offered.contains(&&offer.intent),
                                "{state}: {} sends an action the manager did not offer", offer.label);
                            if let Some(reason) = offer.availability.reason() {
                                assert!(notes.contains(&reason), "{state}: {} hides its refusal {reason}", offer.label);
                            }
                            if pending {
                                assert!(!offer.availability.enabled(), "{state}: {} pressable while pending", offer.label);
                            }
                            checked += 1;
                        }
                    }
                }
            }
        }
        assert!(checked > 500, "only {checked} offers checked");
    }

    #[test]
    fn sample_state_lists_group_by_vendor_and_open_exactly_one_plug_in() {
        for state in STATES {
            let snapshot = sample(Page::Plugins, state);
            let all = query("");
            let list = PluginList::build(&snapshot, &all, false, context(&snapshot, false, false));
            let cards: Vec<_> = list.groups.iter().flat_map(|group| &group.cards).collect();
            assert_eq!(cards.len(), snapshot.products.len(), "{state}");
            assert!(list.groups.iter().all(|group| group.heading.is_some()), "{state}");
            assert_eq!(list.summary[0], format!("{0} of {0} plug-ins · grouped by vendor", snapshot.products.len()));
            assert_eq!(list.empty.is_some(), snapshot.products.is_empty(), "{state}");
            for card in cards {
                let record = card.record.expect("a listed card has its own record");
                assert_eq!(ProductKey::from(&snapshot.products[record]), card.key, "{state}");
                let keys = [card.key.clone()];
                let opened = Query { focus: Some(&keys), ..all };
                let detail = PluginList::build(&snapshot, &opened, true, context(&snapshot, false, false));
                assert!(detail.summary.is_empty() && detail.groups.len() == 1, "{state}");
                assert!(detail.groups[0].heading.is_none(), "{state}");
                assert_eq!(detail.groups[0].cards.len(), 1, "{state}");
                assert_eq!(detail.groups[0].cards[0].key, card.key);
                let loading = LoadingList::build(&snapshot, &opened, true, false);
                assert_eq!(loading.focus_title, Some("Showing the selected plug-in"));
                assert!(loading.rows.iter().all(|row| row.manage.is_none()), "{state}");
            }
            let loading = LoadingList::build(&snapshot, &all, true, false);
            assert!(loading.rows.iter().all(|row| row.manage == Some(MANAGE)), "{state}");
            assert_eq!(loading.count, format!("{0} of {0} current plug-ins", snapshot.products.len()));
        }
    }

    #[test]
    fn prepared_plug_in_leads_with_making_it_available() {
        let snapshot = sample(Page::Plugins, "plugin_action");
        let card = card(&snapshot, 0, false, false);
        let lead = card.lead.expect("a prepared plug-in has one next step");
        assert_eq!(lead.offer.label, "Make available in your DAW");
        assert_eq!(lead.offer.importance, Importance::Primary);
        assert_eq!(lead.offer.availability, Availability::Enabled);
        assert!(matches!(lead.offer.intent, Action::CompatibilityPublishTest { expected_current: None, .. }));
        assert!(lead.offer.consequence.unwrap().starts_with("Makes the native VST3 bridge available"));
        assert!(card.notes.contains(&Text::body(
            "The native bridge is prepared. Make it available below, then choose it in your DAW.")));
        let waiting = self::card(&snapshot, 0, true, false).lead.unwrap().offer.availability;
        assert_eq!(waiting, Availability::Waiting);
    }

    #[test]
    fn available_plug_in_gives_daw_guidance_and_no_lead() {
        for state in ["plugin_ready", "plugin_details", "available"] {
            let snapshot = sample(Page::Plugins, state);
            let card = card(&snapshot, 0, false, false);
            assert_eq!(card.status, Status { words: "Available in your DAW", class: StatusClass::Ready }, "{state}");
            assert_eq!(card.notes[0], Text::body(daw_guidance(None)), "{state}");
            assert_eq!(crate::fact_line(&card.facts), "Native Instruments · Instrument · 8.13.1");
        }
        let snapshot = sample(Page::Plugins, "plugin_ready");
        assert!(card(&snapshot, 0, false, false).lead.is_none());
        let snapshot = sample(Page::Plugins, "available");
        let available = Query { status: StatusFilter::Available, ..query("") };
        assert_eq!(matching(&snapshot, &available).len(), snapshot.products.len());
        let attention = Query { status: StatusFilter::NeedsAttention, ..query("") };
        assert!(matching(&snapshot, &attention).is_empty());
    }

    #[test]
    fn available_product_supplies_limitations_as_visible_notes_only_once() {
        let mut snapshot = sample(Page::Plugins, "plugin_ready");
        snapshot.products[0].limitations = vec!["Saved-project recall has not been checked.".into()];
        let card = card(&snapshot, 0, false, false);
        assert_eq!(card.status.class, StatusClass::Ready);
        assert_eq!(card.notes.iter().filter(|note|
            note.words == "Saved-project recall has not been checked.").count(), 1);
        assert!(!format!("{:?}", technical_records(&snapshot.products[0]))
            .contains("Saved-project recall has not been checked."));
        let product = &mut snapshot.products[0];
        product.disposition = "needs_attention".into();
        product.availability = None;
        product.compatibility = None;
        product.details = serde_json::json!({});
        product.limitations.push("The editor has not been checked.".into());
        let card = self::card(&snapshot, 0, false, false);
        assert!(card.status.class.needs_attention());
        for limitation in &snapshot.products[0].limitations {
            assert_eq!(card.notes.iter().filter(|note| note.words == *limitation).count(), 1,
                "the first limitation may already be the product's issue; retain every distinct limitation once");
        }
    }

    #[test]
    fn publication_does_not_invent_a_passed_sound_or_recall_check() {
        let mut snapshot = sample(Page::Plugins, "plugin_ready");
        let product = &mut snapshot.products[0];
        product.compatibility = None;
        product.availability = Some(crate::model::NativeAvailability { available: true, reason: None });
        let steps = readiness(product);
        assert_eq!(steps[1].class, StatusClass::Ready);
        assert_eq!(steps[2].class, StatusClass::Unknown);
        assert!(steps[2].detail.contains("No retained test result"));
        product.availability = None;
        assert_eq!(readiness(product)[1].class, StatusClass::Unknown);
    }

    #[test]
    fn retained_setups_are_counted_named_and_keep_their_refusal() {
        for (state, expanded) in [("plugin_history", false), ("plugin_history_open", true)] {
            let snapshot = sample(Page::Plugins, state);
            let card = card(&snapshot, 0, false, expanded);
            assert!(card.lead.is_none(), "an available plug-in's re-check is optional");
            let other = card.other_setups.as_ref().unwrap();
            assert_eq!(other.title, "Other setup options (1)");
            assert_eq!(other.id_salt, Some("other-setup-options"));
            assert!(matches!(&other.content[1], Detail::Choice(choice) if choice.offer.label == "Check this plug-in again"));
            let previous = card.previous_setups.as_ref().unwrap();
            assert_eq!(previous.title, "Previous setups (12)");
            assert_eq!(previous.force_open, expanded);
            let Detail::Choice(first) = &previous.content[0] else { panic!("a setup choice") };
            assert_eq!(first.offer.availability, Availability::Refused("Previous files are unavailable".into()));
            assert_eq!(first.notes, ["Previous files are unavailable"]);
            assert!(first.offer.label.starts_with("Restore previous setup"));
            assert_eq!(first.exact_action, serde_json::to_string(&first.offer.intent).unwrap());
            let settings = card.settings.as_ref().expect("audio delivery is a setting");
            assert_eq!(settings.force_open, expanded);
            assert!(settings.content.contains(&Text::body("512 added bridge frames").into()));
            assert_eq!(card.technical.force_open, expanded);
        }
    }

    #[test]
    fn guided_states_separate_optional_results_from_the_next_step() {
        let snapshot = sample(Page::Plugins, "guided");
        let beam = card(&snapshot, 0, false, false);
        assert!(beam.lead.is_none(), "recording a result is optional");
        let help = beam.help.as_ref().unwrap();
        assert!(matches!(&help.content[0], Detail::Offers(row)
            if row.offers[0].label == "Record what worked…"));
        assert!(help.content.contains(&Text::body(
            "Available in your DAW using this setup. You can record a result after trying it.").into()));
        let bismuth = card(&snapshot, 1, false, false);
        assert_eq!(bismuth.status.class, StatusClass::NeedsYou);
        assert_eq!(bismuth.lead.unwrap().offer.label, "Prepare for your DAW");

        let snapshot = sample(Page::Plugins, "guided_pending");
        let beam = card(&snapshot, 0, false, false);
        let lead = beam.lead.unwrap();
        assert_eq!(lead.offer.label, "Finish plug-in recovery");
        assert_eq!(lead.offer.availability, Availability::Refused("Previous instance cleanup is unconfirmed".into()));
        assert_eq!(beam.notes[0].role, crate::TextRole::Caution);
    }

    #[test]
    fn raw_session_history_does_not_supply_live_product_claims() {
        let snapshot = sample(Page::Plugins, "shared");
        let fragments: Vec<_> = snapshot.products.iter().enumerate()
            .filter(|(_, p)| p.class_id == "fixture-fragments").map(|(index, _)| index).collect();
        assert_eq!(fragments.len(), 2);
        for index in fragments {
            let card = card(&snapshot, index, false, false);
            assert_eq!(card.technical.content[0], Text::detail("Live activity has not been observed").into());
            assert!(!card.notes.iter().any(|note| note.words.contains("active instance")));
        }
        let kontakt = snapshot.products.iter().position(|p| p.class_id == "fixture-kontakt").unwrap();
        let card = card(&snapshot, kontakt, false, false);
        assert!(!card.notes.iter().any(|note| note.words.contains("most recent session") || note.words.contains("Cleanup and transport retirement confirmed")));
    }

    #[test]
    fn first_use_and_stale_lists_say_what_is_missing() {
        let snapshot = sample(Page::Plugins, "first_use");
        let all = query("");
        let list = PluginList::build(&snapshot, &all, false, context(&snapshot, false, false));
        assert_eq!(list.empty.unwrap().0, "No plug-ins discovered yet");
        assert!(list.groups.is_empty());
        assert_eq!(LoadingList::build(&snapshot, &all, true, false).empty,
            Some("No plug-ins discovered yet. Open Setup to add and scan an installer."));
        let snapshot = sample(Page::Plugins, "stale");
        let stale = LoadingList::build(&snapshot, &all, false, false);
        assert_eq!(stale.notice, Text::caution(
            "Current plug-in status could not be loaded. Try loading it again before continuing."));
        assert!(stale.rows.iter().all(|row| row.status.is_none()));
        let failed = LoadingList::build(&snapshot, &all, true, true);
        assert_eq!(failed.retry, Some("Load plug-in controls again"));
    }

    #[test]
    fn technical_records_keep_every_retained_preparation_fact() {
        let snapshot = sample(Page::Plugins, "plugin_technical");
        let records = technical_records(&snapshot.products[0]);
        let mut lines = Vec::new();
        fn walk<'a>(details: &'a [Detail], out: &mut Vec<&'a str>) {
            for detail in details {
                match detail {
                    Detail::Text(text) => out.push(&text.words),
                    Detail::Disclosure(inner) => { out.push(&inner.title); walk(&inner.content, out); }
                    Detail::Group(inner) => walk(inner, out),
                    _ => {}
                }
            }
        }
        walk(&records, &mut lines);
        assert!(card(&snapshot, 0, false, false).notes.contains(&Text::caution("editor not yet assessed")));
        assert!(!lines.contains(&"editor not yet assessed"), "the visible limitation is not duplicated in technical records");
        for expected in ["Ordinarily published", "Candidate generations and their own results",
            "This publication retains its original acceptance. New evidence needs review; use Withdraw to disable it.",
            "The previous preparation stopped; prepare again.", "Inspection: inspected · Preparation: prepared · Publication: ordinary",
            "Adopted exact SV1 work; originally prepared through the engineering CLI.", "Parameters: 412 · Positive inspection cleanup: true",
            "This product's operation: failed", "Stopped during native build", "audio: passed", "operator · Played a chord",
            "editor: failed", "midi: not tested", "Published revision: 3 · Recommended: 2",
            "Version 8.13 · default graphics · ordinary", "Installed files changed since this setup", "Technical product details"] {
            assert!(lines.contains(&expected), "{expected} missing: {lines:?}");
        }
        let candidates = records.iter().find_map(|detail| match detail {
            Detail::Disclosure(inner) if inner.title == "Candidate generations and their own results" => Some(inner),
            _ => None }).unwrap();
        assert!(candidates.start_open, "several generations start open");
        assert!(candidates.content.iter().all(|detail| matches!(detail, Detail::Group(_))));
    }
    #[test]
    fn standalone_controls_bind_each_exact_window_and_preserve_refusals() {
        let mut snapshot = snapshot();
        snapshot.products.truncate(1);
        snapshot.environment_installers.clear(); snapshot.vendor_applications.clear(); snapshot.environments.clear();
        let p = &mut snapshot.products[0]; p.compatibility = None;
        let open = Action::PluginOpen { product: crate::model::ProductIdentity::from(&*p),
            expected_current: crate::model::PublicationIdentity { id:"ab".repeat(16), sha256:"cd".repeat(32) } };
        let mut actions = vec![AvailableAction { label:"Open on its own".into(), action:open.clone(),
            disabled_reason:Some("This setup is being changed".into()) }];
        for value in ["ab", "cd"] {
            let operation = value.repeat(16); let session = value.repeat(16);
            for (label, action) in [("Show the plug-in window", Action::PluginFocus {operation:operation.clone(),session:session.clone()}),
                ("Close this plug-in window", Action::PluginClose {operation,session})] {
                actions.push(AvailableAction {label:label.into(),action,disabled_reason:None});
            }
        }
        p.actions = actions.clone();
        let view = card(&snapshot, 0, false, false);
        assert_eq!(view.standalone.len(), 3);
        assert_eq!(view.standalone[0].offers[0].intent, open);
        assert_eq!(view.standalone[0].offers[0].availability, Availability::Refused("This setup is being changed".into()));
        assert!(view.standalone[0].notes.iter().any(|text| text == "This setup is being changed"));
        assert_eq!(view.standalone.iter().flat_map(|row| &row.offers).map(|offer| offer.intent.clone()).collect::<Vec<_>>(),
            actions.iter().map(|offer| offer.action.clone()).collect::<Vec<_>>());
        for (row, expected) in view.standalone[1..].iter().zip(actions[1..].chunks(2)) {
            assert_eq!(row.offers[0].pressed(), Some(expected[0].action.clone()));
            assert_eq!(row.offers[1].pressed(), Some(expected[1].action.clone()));
            assert_eq!(row.offers[0].importance, Importance::Primary);
            assert_eq!(row.offers[1].importance, Importance::Secondary);
        }
        assert!(view.offers.offers.is_empty() && view.lead.is_none());
        assert!(!format!("{:?}", view.technical).contains("PluginFocus"), "ordinary controls are not duplicated in expert actions");
        let pending = card(&snapshot, 0, true, false);
        assert_eq!(pending.standalone[0].offers[0].availability, view.standalone[0].offers[0].availability);
        assert!(pending.standalone[1..].iter().flat_map(|row| &row.offers)
            .all(|offer| offer.availability == Availability::Waiting && offer.pressed().is_none()));
        snapshot.products[0].actions.clear();
        assert!(card(&snapshot, 0, false, false).standalone.is_empty(), "observed windows never manufacture controls");
    }

}
