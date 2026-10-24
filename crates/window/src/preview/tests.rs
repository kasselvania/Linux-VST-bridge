//! The preview draws exactly the view model's offers, presses them only
//! through `State`, and asks before every removal or deletion. These run
//! over every sample the view model offers, with nothing on screen.

use {
    super::*,
    crate::presentation::{self, FormStep, Node, Target},
    lvb_view_model::{forms, offers::Offer, samples, window::Page, Destination, Local},
    makepad_product_ui::{makepad_widgets::Cx, Importance, InputPath},
    std::collections::BTreeMap,
};

fn state_index(name: &str) -> usize {
    STATES.iter().position(|state| *state == name).unwrap()
}

/// Every page in every named state, the request samples over Home and the
/// plug-in page, and the update samples over Home and Setup.
fn every_selection() -> Vec<Selection> {
    let mut out = Vec::new();
    for page in Page::ALL {
        for state in 0..STATES.len() {
            out.push(Selection { state, page, request: None, update: None });
        }
    }
    for (page, state) in [(Page::Home, "populated"), (Page::Plugins, "plugin_ready")] {
        for request in 0..REQUESTS.len() {
            out.push(Selection { state: state_index(state), page, request: Some(request), update: None });
        }
    }
    for page in [Page::Home, Page::Setup] {
        for update in 0..samples::updates().len() {
            out.push(Selection { state: state_index("available"), page, request: None, update: Some(update) });
        }
    }
    out
}

fn name(selection: &Selection) -> String {
    format!("{}/{} request {:?} update {:?}", selection.page.key(), selection.state_name(), selection.request,
        selection.update)
}

fn revealed(selection: Selection) -> Preview {
    let mut preview = Preview::new(selection);
    preview.reveal = true;
    preview.rebuild();
    preview
}

/// Collection controls navigate to detail; exercise the controls on those
/// destinations as well as each sample's initial page.
fn revealed_windows(selection: Selection) -> Vec<Preview> {
    let base = revealed(selection);
    let products = base.sample.snapshot.products.clone();
    let mut windows = vec![base];
    for product in products {
        let mut detail = revealed(selection);
        detail.state.go(&Destination::Product(ProductKey::from(&product)));
        detail.rebuild();
        windows.push(detail);
    }
    windows
}

/// Every `Offer` in the view model's window, found in its `Debug` form, so
/// the list does not depend on how the preview walks the window.
fn view_model_offers(window: &Window) -> Vec<String> {
    let text = format!("{window:?}");
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(found) = text[from..].find("Offer { label: ") {
        let start = from + found;
        // `LeadOffer {` and the like name other structs.
        let standalone = start == 0 || !(bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_');
        let open = start + "Offer ".len();
        let (mut depth, mut i, mut in_string) = (0usize, open, false);
        while i < bytes.len() {
            let b = bytes[i];
            if in_string {
                if b == b'\\' { i += 1; } else if b == b'"' { in_string = false; }
            } else if b == b'"' {
                in_string = true;
            } else if b == b'{' {
                depth += 1;
            } else if b == b'}' {
                depth -= 1;
                if depth == 0 { break; }
            }
            i += 1;
        }
        if standalone {
            out.push(text[start..=i].to_string());
        }
        from = start + 1;
    }
    out.sort();
    out
}

fn drawn(preview: &Preview) -> Vec<(makepad_product_ui::ActionView, Offer)> {
    let mut lists: Vec<&[Node]> = vec![&preview.built.header, &preview.built.page];
    if let Some(form) = &preview.built.form {
        lists.push(form);
    }
    presentation::drawn_offers(&preview.built.targets, &lists)
}

/// Nodes that remain visible even if every disclosure was previously closed.
fn outside_folds(nodes: &[Node]) -> Vec<&Node> {
    let mut out = Vec::new();
    for node in nodes {
        out.push(node);
        match node {
            Node::Panel { children, .. } | Node::Row(children) | Node::Stack(children)
                | Node::Columns(children) | Node::Header(children) => out.extend(outside_folds(children)),
            _ => {}
        }
    }
    out
}

#[test]
fn the_preview_starts_from_the_window_the_samples_build() {
    for selection in every_selection() {
        let preview = Preview::new(selection);
        assert_eq!(preview.window(), preview.sample.window(Some(DISTRIBUTION)), "{}", name(&selection));
    }
}

#[test]
fn every_drawn_offer_preserves_the_owners_identity_availability_and_reason() {
    let mut checked = 0;
    let mut navigated = 0;
    let mut pressable_total = 0;
    for selection in every_selection() {
        let windows = revealed_windows(selection);
        navigated += windows.len() - 1;
        for preview in windows {
            let all = view_model_offers(&preview.window());
            let offers_drawn = drawn(&preview);
            for (view, offer) in &offers_drawn {
                assert!(all.contains(&format!("{offer:?}")), "{}: invented owner offer", name(&selection));
                assert_eq!(view.invoke(InputPath::Pointer).is_invoke(), offer.availability.reason().is_none(),
                    "{}: {} availability", name(&selection), offer.label);
                assert_eq!(view.label, offer.label, "{}", name(&selection));
                assert_eq!(view.reason(), offer.availability.reason(), "{}: {} prints its reason",
                    name(&selection), offer.label);
                pressable_total += usize::from(view.invoke(InputPath::Pointer).is_invoke());
            }
            checked += 1;
        }
    }
    assert!(checked > 243 && navigated > 100 && pressable_total > 200);
}

#[test]
fn home_rows_open_the_exact_product_and_do_not_mix_setup_siblings() {
    let mut preview = Preview::new(Selection { state: state_index("populated"), page: Page::Home,
        ..Selection::default() });
    let expected: Vec<_> = preview.sample.snapshot.products.iter().map(ProductKey::from).collect();
    let rows: Vec<_> = preview.built.page.iter().filter_map(|node| match node {
        Node::LibraryRow { action, .. } => Some(action.id.0.clone()), _ => None,
    }).collect();
    assert_eq!(rows.len(), expected.len());
    for (id, key) in rows.iter().zip(&expected) {
        let Target::Control(control) = &preview.built.targets[id] else { panic!("row is not navigation") };
        assert_eq!(control.does, Local::Go(Destination::Product(key.clone())));
    }
    let id = rows[0].clone();
    assert_eq!(preview.press(&id, InputPath::Pointer), Outcome::Navigated);
    assert_eq!(preview.state.plugins.focused_product(), expected.first());
    assert!(preview.built.page.iter().any(|node| matches!(node, Node::Stack(_))));
}

#[test]
fn an_unfinished_installation_remains_actionable_beside_working_products() {
    let mut preview = Preview::new(Selection { state: state_index("install_attention"),
        page: Page::Home, ..Selection::default() });
    // Keep the partially successful installation; remove the unrelated cleanup
    // fixture so this test exercises the mixed healthy/failed setup itself.
    preview.sample.snapshot.installer_setups.retain(|setup| setup.journey.is_some());
    preview.rebuild();
    let window = preview.window();
    let lvb_view_model::screen::Screen::Home(home) = window.screen else { panic!("Home") };
    assert!(!home.plugins.is_empty());
    let lvb_view_model::home::Next::Setup { title, primary: Some(primary), open, .. } = &home.next
        else { panic!("The unfinished installation must remain the next action") };
    assert!(drawn(&preview).iter().any(|(_, offer)| offer == primary),
        "A populated library must not hide the offered continuation");
    let callout = preview.built.page.iter().find_map(|node| match node {
        Node::Panel { children, .. } if children.iter().any(|node|
            matches!(node, Node::Heading(heading) if heading == title)) => Some(children),
        _ => None,
    }).expect("compact installation callout");
    assert!(callout.iter().any(|node| matches!(node, Node::Actions { stacked: false, actions, .. }
        if actions.len() == 2
            && matches!(preview.built.targets.get(&actions[0].id.0), Some(Target::Offer { offer, .. }) if offer == primary)
            && matches!(preview.built.targets.get(&actions[1].id.0), Some(Target::Control(control)) if control == open))),
        "Continuation and exact setup navigation share one wrapping action row");
    assert!(!format!("{callout:?}").contains("Example Keys"),
        "The healthy sibling belongs in its own library row, not the problem callout");
    let open_id = control_id(&preview, &open.does);
    assert_eq!(preview.press(&open_id, InputPath::Keyboard), Outcome::Navigated);
    assert_eq!(preview.state.page, Page::Setup);
}

#[test]
fn published_product_recovery_stays_visible_beside_healthy_siblings() {
    use lvb_view_model::model::{CompatibilityPhase, CompatibilityWorkflow};
    for phase in [CompatibilityPhase::AwaitingRetirement, CompatibilityPhase::PublicationNeedsAttention] {
        let mut preview = Preview::new(Selection { state: state_index("available"),
            page: Page::Home, ..Selection::default() });
        preview.sample.snapshot.installer_setups.clear();
        let products = &mut preview.sample.snapshot.products;
        products.truncate(2);
        products[0].name = "Affected instrument".into();
        products[0].active_revision = Some(1);
        products[1].name = "Healthy sibling".into();
        products[0].compatibility = Some(CompatibilityWorkflow {
            phase, summary: "Close the affected plug-in, then finish its recovery.".into(),
            established: vec![], remaining: vec![], current_inspection: None,
            current_candidate: None, primary: None, alternatives: vec![],
        });
        let key = ProductKey::from(&products[0]);
        assert!(products[0].active_revision.is_some());
        samples::presentation(&mut preview.sample.snapshot, "available");
        preview.rebuild();
        let callout = preview.built.page.iter().find_map(|node| match node {
            Node::Panel { children, .. } if children.iter().any(|node|
                matches!(node, Node::Heading(title) if title == "Recover Affected instrument")) => Some(children),
            _ => None,
        }).expect("recovery callout outside disclosures on populated Home");
        assert!(outside_folds(callout).iter().any(|node| matches!(node, Node::Text(_, text)
            if text == "Close the affected plug-in, then finish its recovery.")));
        assert!(!format!("{callout:?}").contains("Healthy sibling"));
        let id = outside_folds(callout).into_iter().find_map(|node| match node {
            Node::Actions { actions, .. } => actions.iter().find_map(|action|
                matches!(preview.built.targets.get(&action.id.0), Some(Target::Control(control))
                    if control.does == Local::Go(Destination::Product(key.clone())))
                    .then(|| action.id.0.clone())),
            _ => None,
        }).expect("the callout routes to the exact affected plug-in");
        assert_eq!(preview.press(&id, InputPath::Keyboard), Outcome::Navigated);
        assert_eq!(preview.state.plugins.focused_product(), Some(&key));
    }
    let ordinary = Preview::new(Selection::default());
    assert!(!outside_folds(&ordinary.built.page).iter().any(|node| matches!(node,
        Node::Heading(title) if title.starts_with("Next: set up"))));
}

#[test]
fn supplied_limitations_are_visible_once_beside_readiness() {
    let mut preview = Preview::new(Selection { state: state_index("plugin_ready"),
        page: Page::Plugins, ..Selection::default() });
    let limitation = "Saved-project recall has not been checked.";
    preview.sample.snapshot.products[0].limitations = vec![limitation.into()];
    let key = ProductKey::from(&preview.sample.snapshot.products[0]);
    preview.state.go(&Destination::Product(key));
    preview.rebuild();
    assert!(!preview.reveal && !preview.state.plugins.expanded);
    let readiness = outside_folds(&preview.built.page).into_iter().find_map(|node| match node {
        Node::Panel { children, .. } if children.iter().any(|node|
            matches!(node, Node::Label(label) if label == "Is it ready?")) => Some(children),
        _ => None,
    }).expect("readiness panel");
    assert_eq!(outside_folds(readiness).iter().filter(|node|
        matches!(node, Node::Text(_, text) if text == limitation)).count(), 1);
    assert_eq!(format!("{:?}", preview.built.page).matches(limitation).count(), 1,
        "technical records must not duplicate the visible limitation");
}

#[test]
fn active_installer_stop_is_visible_with_another_setup_focused_and_disclosures_closed() {
    for refusal in [None, Some("Waiting for this installer to finish closing")] {
        for selected_running in [false, true] {
            let mut preview = Preview::new(Selection { state: state_index("existing_setup"),
                page: Page::Setup, ..Selection::default() });
            let targets = &mut preview.sample.snapshot.environment_installers;
            let target = targets.iter_mut().find(|target| target.actions.iter().any(|offer|
                matches!(offer.action, Action::EnvironmentInstallerStop { .. }))).unwrap();
            let running = target.environment.clone();
            let stop = target.actions.iter_mut().find(|offer|
                matches!(offer.action, Action::EnvironmentInstallerStop { .. })).unwrap();
            stop.disabled_reason = refusal.map(str::to_owned);
            let expected = lvb_view_model::offers::offer(stop,
                lvb_view_model::offers::Importance::Secondary, None, false);
            let other = preview.sample.snapshot.installer_setups.iter().find(|setup|
                setup.environment.as_deref().is_some_and(|environment| environment != running)).unwrap();
            preview.state.go(&Destination::Attempt(lvb_view_model::AttemptKey {
                installer: other.installer.clone(), environment: other.environment.clone() }));
            preview.ui.choices.insert("existing".into(), if selected_running { running }
                else { other.environment.clone().unwrap() });
            preview.rebuild();
            assert!(!preview.reveal);
            assert!(preview.built.page.iter().any(|node| matches!(node,
                Node::Fold { id, start_open: false, .. } if id == "fold.existing-setups")));
            let visible_ids: Vec<_> = outside_folds(&preview.built.page).into_iter().filter_map(|node| match node {
                Node::Actions { actions, .. } => Some(actions.iter().map(|action| action.id.0.clone())),
                _ => None,
            }).flatten().collect();
            let matching: Vec<_> = drawn(&preview).into_iter().filter(|(_, offer)| offer.intent == expected.intent).collect();
            assert_eq!(matching.len(), 1, "Stop must not also be duplicated in the folded choice");
            let (view, offer) = &matching[0];
            assert!(visible_ids.contains(&view.id.0), "Stop must remain outside every disclosure");
            assert_eq!(offer, &expected);
            assert_eq!(view.reason(), refusal);
            assert_eq!(view.consequence.as_deref(), expected.consequence);
        }
    }
}

/// The form's control for `step`, if drawn.
fn form_control(preview: &Preview, step: FormStep) -> Option<makepad_product_ui::ActionView> {
    let form = preview.built.form.as_ref()?;
    fn walk(nodes: &[Node], out: &mut Vec<makepad_product_ui::ActionView>) {
        for node in nodes {
            match node {
                Node::Actions { actions, .. } => out.extend(actions.iter().cloned()),
                Node::Panel { children, .. } | Node::Fold { children, .. } | Node::Row(children) | Node::Stack(children) | Node::Columns(children) | Node::Header(children) => walk(children, out),
                _ => {}
            }
        }
    }
    let mut views = Vec::new();
    walk(form, &mut views);
    views.into_iter().find(|view| preview.built.targets.get(&view.id.0) == Some(&Target::Form(step.clone())))
}

#[test]
fn every_destructive_offer_opens_its_confirmation_first() {
    let mut exercised = 0;
    let mut confirmed = 0;
    for base in every_selection().into_iter().flat_map(revealed_windows) {
        let selection = base.selection;
        for (view, offer) in drawn(&base) {
            let destructive = forms::needs_confirmation(&offer.intent);
            assert_eq!(view.importance == Importance::Destructive, destructive,
                "{}: {} is drawn destructive exactly when it asks first", name(&selection), offer.label);
            if !destructive || !view.invoke(InputPath::Pointer).is_invoke() {
                continue;
            }
            for via in [InputPath::Pointer, InputPath::Keyboard, InputPath::Touch] {
                let mut preview = base.clone();
                preview.press(&view.id.0, via);
                assert!(preview.captured.is_empty(), "{}: {} was sent without asking", name(&selection), offer.label);
                assert_eq!(preview.state.forms.confirm.as_ref(), Some(&offer.intent), "{}", name(&selection));
                let confirm = form_control(&preview, FormStep::Confirm).expect("the confirmation's own button");
                let cancel = form_control(&preview, FormStep::CancelConfirm).expect("a way back");
                assert_eq!(confirm.importance, if matches!(offer.intent, Action::EnvironmentRuntimeSelect {..}) {
                    Importance::Primary
                } else { Importance::Destructive });
                // The safe answer has focus when the confirmation opens.
                assert_eq!(preview.built.form_focus.as_deref(), Some(cancel.id.0.as_str()));
                let mut cancelled = preview.clone();
                cancelled.press(&cancel.id.0, via);
                assert!(cancelled.captured.is_empty() && cancelled.built.form.is_none(), "{}", name(&selection));
                let mut dismissed = preview.clone();
                dismissed.dismiss_form("by Escape");
                assert!(dismissed.captured.is_empty() && dismissed.state.forms.confirm.is_none());
                if confirm.invoke(via).is_invoke() {
                    preview.press(&confirm.id.0, via);
                    assert_eq!(preview.captured.len(), 1, "{}", name(&selection));
                    let (sent, answered) = &preview.captured[0];
                    assert!(*answered && *sent == forms::confirmed(offer.intent.clone()), "{}: {sent:?}", name(&selection));
                    assert!(preview.state.forms.confirm.is_none(), "sending closes the confirmation");
                    confirmed += 1;
                } else {
                    // Held: the reason is printed and pressing it sends nothing.
                    assert!(confirm.reason().is_some_and(|reason| !reason.is_empty()));
                    preview.press(&confirm.id.0, via);
                    assert!(preview.captured.is_empty());
                }
            }
            exercised += 1;
        }
    }
    assert!(exercised >= 3, "only {exercised} destructive offers exercised");
    println!("{exercised} destructive offers ask first; {confirmed} confirmed presses sent their confirmed action");
}

#[test]
fn no_press_sends_a_removal_without_its_confirmation() {
    let mut presses = 0;
    for base in every_selection().into_iter().flat_map(revealed_windows) {
        let selection = base.selection;
        let ids: Vec<String> = base.drawn.iter().filter(|(_, reason)| reason.is_none()).map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            if matches!(base.built.targets.get(&id), Some(Target::Preview(_))) {
                continue;
            }
            let mut preview = base.clone();
            preview.press(&id, InputPath::Pointer);
            for (action, answered) in &preview.captured {
                assert!(!forms::needs_confirmation(action) || *answered, "{}: {id} sent {action:?}", name(&selection));
            }
            presses += 1;
        }
    }
    println!("{presses} presses checked");
}

#[test]
fn every_held_control_prints_its_reason_and_refuses_every_path() {
    let mut held = 0;
    for base in every_selection().into_iter().flat_map(revealed_windows) {
        let selection = base.selection;
        for (id, reason) in &base.drawn {
            let Some(reason) = reason else { continue };
            assert!(!reason.trim().is_empty(), "{}: {id} is held without a reason", name(&selection));
            let mut preview = base.clone();
            for via in [InputPath::Pointer, InputPath::Touch, InputPath::Keyboard, InputPath::Shortcut] {
                assert_eq!(preview.press(id, via), Outcome::Refused, "{}: {id}", name(&selection));
            }
            assert!(preview.captured.is_empty() && preview.state == base.state, "{}: {id}", name(&selection));
            held += 1;
        }
    }
    assert!(held > 50, "only {held} held controls");
    println!("{held} held controls print their reason");
}

fn control_id(preview: &Preview, local: &Local) -> String {
    preview.built.targets.iter().find_map(|(id, target)| match target {
        Target::Control(control) if control.does == *local && control.enabled => Some(id.clone()),
        _ => None,
    }).unwrap_or_else(|| panic!("no control for {local:?}"))
}

#[test]
fn navigation_follows_the_window_state() {
    let mut preview = Preview::new(Selection { state: state_index("populated"), ..Selection::default() });
    assert_eq!(preview.state.page, Page::Home);
    // A plug-in row opens that exact plug-in on the Plug-ins tab.
    let manage = preview.built.targets.iter().find_map(|(id, target)| match target {
        Target::Control(control) => match &control.does {
            Local::Go(Destination::Product(key)) => Some((id.clone(), key.clone())),
            _ => None,
        },
        _ => None,
    }).expect("a Manage plug-in control");
    assert_eq!(preview.press(&manage.0, InputPath::Keyboard), Outcome::Navigated);
    assert_eq!(preview.state.page, Page::Plugins);
    assert_eq!(preview.state.plugins.focused_product(), Some(&manage.1));
    assert!(matches!(preview.window().screen, lvb_view_model::screen::Screen::Plugins { single: true, .. }));
    // Tabs: Support is Diagnostics, Home's "Add a plug-in" is Setup.
    let tab = |preview: &Preview, label: &str| preview.built.targets.iter().find_map(|(id, target)|
        matches!(target, Target::Tab(tab) if tab.label() == label).then(|| id.clone())).unwrap();
    let support = tab(&preview, "Support");
    preview.press(&support, InputPath::Pointer);
    assert_eq!(preview.state.page, Page::Diagnostics);
    let home = tab(&preview, "Home");
    preview.press(&home, InputPath::Touch);
    assert_eq!(preview.state.page, Page::Home);
    let add = control_id(&preview, &Local::Go(Destination::Setup));
    preview.press(&add, InputPath::Pointer);
    assert_eq!((preview.state.page, presentation::Tab::of(preview.state.page)), (Page::Setup, presentation::Tab::Home));
    // Service refresh is in Support; effects are captured, never performed.
    let support = tab(&preview, "Support");
    preview.press(&support, InputPath::Pointer);
    let refresh = control_id(&preview, &Local::Refresh);
    preview.press(&refresh, InputPath::Shortcut);
    assert!(preview.log.last().unwrap().starts_with("Effect Refresh"));
    assert!(preview.captured.is_empty());
}

#[test]
fn an_installation_asks_for_the_daw_then_sends_it_with_the_choice() {
    let mut preview = Preview::new(Selection { state: state_index("first_use"), page: Page::Setup, ..Selection::default() });
    preview.reveal = true;
    preview.rebuild();
    let Some((id, offer)) = drawn(&preview).into_iter().find(|(view, offer)| view.invoke(InputPath::Pointer).is_invoke()
        && forms::installation_with_daw(&offer.intent, lvb_view_model::model::DawChoice::Other).is_some())
        .map(|(view, offer)| (view.id.0, offer)) else {
        // first_use offers only the runtime download; look over every sample instead.
        return any_installation_asks_for_the_daw();
    };
    install_flow(preview, &id, &offer);
}

fn any_installation_asks_for_the_daw() {
    for selection in every_selection() {
        let mut preview = revealed(selection);
        preview.state.forms.daw = None;
        preview.rebuild();
        if let Some((view, offer)) = drawn(&preview).into_iter().find(|(view, offer)| view.invoke(InputPath::Pointer)
            .is_invoke() && forms::installation_with_daw(&offer.intent, lvb_view_model::model::DawChoice::Other).is_some()) {
            install_flow(preview, &view.id.0, &offer);
            return;
        }
    }
    panic!("no sample offers an installation");
}

fn install_flow(mut preview: Preview, id: &str, offer: &Offer) {
    preview.state.forms.daw = None;
    preview.rebuild();
    preview.press(id, InputPath::Pointer);
    assert!(preview.captured.is_empty() && preview.state.forms.install.is_some(), "{offer:?}");
    let start = form_control(&preview, FormStep::StartInstall).unwrap();
    assert!(!start.invoke(InputPath::Pointer).is_invoke(), "nothing starts before a DAW is chosen");
    let reaper = form_control(&preview, FormStep::ChooseDaw(lvb_view_model::model::DawChoice::Reaper)).unwrap();
    preview.press(&reaper.id.0, InputPath::Keyboard);
    let start = form_control(&preview, FormStep::StartInstall).unwrap();
    if start.invoke(InputPath::Pointer).is_invoke() {
        preview.press(&start.id.0, InputPath::Keyboard);
        let expected = forms::installation_with_daw(&offer.intent, lvb_view_model::model::DawChoice::Reaper);
        assert_eq!(preview.captured.first().map(|(action, _)| action.clone()), expected);
        assert!(preview.state.forms.install.is_none());
    } else {
        assert!(start.reason().is_some());
    }
}

#[test]
fn building_any_sample_takes_well_under_a_frame() {
    let mut slowest = (0.0f64, String::new());
    let mut total = 0.0;
    let selections = every_selection();
    for selection in &selections {
        let started = Cx::monotonic_now();
        let mut preview = Preview::new(*selection);
        let loaded = (Cx::monotonic_now() - started) * 1000.0;
        let started = Cx::monotonic_now();
        preview.rebuild();
        let rebuilt = (Cx::monotonic_now() - started) * 1000.0;
        total += rebuilt;
        if loaded.max(rebuilt) > slowest.0 {
            slowest = (loaded.max(rebuilt), name(selection));
        }
    }
    println!("{} samples: slowest load or rebuild {:.2} ms ({}), mean rebuild {:.3} ms", selections.len(),
        slowest.0, slowest.1, total / selections.len() as f64);
    // Debug builds are slower; a release build is far below this.
    assert!(slowest.0 < 100.0, "{} took {:.1} ms", slowest.1, slowest.0);
}

#[test]
fn ids_are_unique_and_every_target_is_drawn() {
    for selection in every_selection() {
        let preview = revealed(selection);
        let mut seen = BTreeMap::new();
        fn walk(nodes: &[Node], seen: &mut BTreeMap<String, usize>) {
            for node in nodes {
                match node {
                    Node::LibraryRow { action, .. } => {
                        *seen.entry(action.id.0.clone()).or_default() += 1;
                    }
                    Node::Actions { id, actions, .. } => {
                        *seen.entry(id.clone()).or_default() += 1;
                        for action in actions { *seen.entry(action.id.0.clone()).or_default() += 1; }
                    }
                    Node::Tabs { id, tabs, .. } => {
                        *seen.entry(id.clone()).or_default() += 1;
                        for tab in tabs { *seen.entry(tab.id.0.clone()).or_default() += 1; }
                    }
                    Node::Chip { id, .. } | Node::Facts { id, .. } | Node::Notice { id, .. } | Node::Field { id, .. } =>
                        *seen.entry(id.clone()).or_default() += 1,
                    Node::Fold { id, children, .. } => {
                        *seen.entry(id.clone()).or_default() += 1;
                        walk(children, seen);
                    }
                    Node::Panel { children, .. } | Node::Row(children) | Node::Stack(children) | Node::Columns(children) | Node::Header(children) => walk(children, seen),
                    _ => {}
                }
            }
        }
        for list in [Some(&preview.built.preview), Some(&preview.built.header), Some(&preview.built.page),
            preview.built.form.as_ref()].into_iter().flatten() {
            walk(list, &mut seen);
        }
        let repeated: Vec<_> = seen.iter().filter(|(_, n)| **n > 1).collect();
        assert!(repeated.is_empty(), "{}: {repeated:?}", name(&selection));
        for id in preview.built.targets.keys() {
            assert!(preview.drawn.contains_key(id), "{}: {id} has a meaning but no control", name(&selection));
        }
    }
}

#[test]
fn removing_or_reordering_setup_rows_never_retargets_an_old_control() {
    let mut preview = revealed(Selection { state: state_index("setup_many"), page: Page::Setup,
        ..Selection::default() });
    let old = preview.built.targets.clone();
    assert!(preview.sample.snapshot.installer_setups.len() > 2);
    preview.sample.snapshot.installer_setups.reverse();
    preview.sample.snapshot.installer_setups.pop();
    preview.rebuild();
    let mut removed = 0;
    for (id, target) in old {
        if let Some(current) = preview.built.targets.get(&id) {
            assert_eq!(current, &target, "a retained id must keep its exact meaning");
        } else {
            assert_eq!(preview.press(&id, InputPath::Pointer), Outcome::Refused);
            removed += 1;
        }
    }
    assert!(removed > 0, "the removed setup must retire its controls");
    assert!(preview.captured.is_empty());
    assert!(preview.state.forms.rename.is_none());
}

#[test]
fn rename_refusal_explains_the_owner_validation() {
    let mut preview = revealed(Selection { state: state_index("setup_many"), page: Page::Setup,
        ..Selection::default() });
    let rename = drawn(&preview).into_iter().find(|(_, offer)|
        matches!(offer.intent, lvb_view_model::model::Action::InstallerRename { .. })).unwrap();
    preview.press(&rename.0.id.0, InputPath::Pointer);
    for label in ["é".repeat(49), "trailing ".into(), ".".into(), "..".into()] {
        preview.field("field.rename", &label);
        let save = form_control(&preview, FormStep::SaveRename).unwrap();
        assert!(save.reason().unwrap().contains("96 UTF-8 bytes"));
        assert_eq!(preview.press(&save.id.0, InputPath::Keyboard), Outcome::Refused);
    }
    preview.field("field.rename", &"é".repeat(48));
    assert!(form_control(&preview, FormStep::SaveRename).unwrap().invoke(InputPath::Keyboard).is_invoke());
}

#[test]
fn a_closed_forms_confirm_cannot_confirm_a_new_form() {
    for selection in every_selection() {
        let base = revealed(selection);
        let Some((view, _)) = drawn(&base).into_iter().find(|(view, offer)|
            forms::needs_confirmation(&offer.intent) && view.invoke(InputPath::Pointer).is_invoke()) else { continue };
        let mut preview = base;
        preview.press(&view.id.0, InputPath::Pointer);
        let old_confirm = form_control(&preview, FormStep::Confirm).unwrap();
        preview.dismiss_form("by Escape");
        preview.press(&view.id.0, InputPath::Pointer);
        let current = form_control(&preview, FormStep::Confirm).unwrap();
        assert_ne!(old_confirm.id, current.id);
        assert_eq!(preview.press(&old_confirm.id.0, InputPath::Pointer), Outcome::Refused);
        assert!(preview.captured.is_empty());
        assert!(preview.state.forms.confirm.is_some());
        return;
    }
    panic!("no enabled destructive sample");
}

#[test]
fn display_and_repair_are_local_pages_of_the_same_exact_product() {
    use lvb_view_model::state::ProductPage;
    let mut preview = Preview::new(Selection {state:state_index("plugin_ready"),page:Page::Plugins,..Selection::default()});
    preview.display_fixture();
    let key = preview.state.plugins.focused_product().unwrap().clone();
    fn navigate(preview:&mut Preview, page:ProductPage) {
        let id = preview.built.targets.iter().find_map(|(id,target)| matches!(target,
            Target::List(ListStep::ProductPage {page:shown,..}) if *shown==page).then(||id.clone())).unwrap();
        assert_eq!(preview.press(&id,InputPath::Pointer),Outcome::Navigated);
    }
    fn offered(preview:&Preview, predicate:impl Fn(&Action)->bool)->Option<String> {
        preview.built.targets.iter().find_map(|(id,target)| matches!(target,
            Target::Offer {offer,..} if predicate(&offer.intent)).then(||id.clone()))
    }
    assert!(offered(&preview,|a|matches!(a,Action::PluginOpen {..})).is_some());
    assert!(offered(&preview,|a|matches!(a,Action::CandidateSettingsPrepare {..})).is_none(),
        "ordinary detail must not append the display/repair pages");
    navigate(&mut preview,ProductPage::Display);
    assert_eq!(preview.state.plugins.focused_product(),Some(&key));
    assert!(offered(&preview,|a|matches!(a,Action::PluginOpen {..})).is_none());
    let record = offered(&preview,|a|matches!(a,Action::CompatibilityResult {..})).unwrap();
    preview.press(&record,InputPath::Pointer);
    assert!(preview.state.forms.record.is_some() && preview.captured.is_empty());
    preview.dismiss_form("fixture Escape");
    let choose = offered(&preview,|a|matches!(a,Action::CompatibilityPublishTest {..})).unwrap();
    let Some(Target::Offer {offer,..}) = preview.built.targets.get(&choose).cloned() else {panic!()};
    let mut chosen = preview.clone(); chosen.press(&choose,InputPath::Keyboard);
    assert_eq!(chosen.captured.last().unwrap().0,offer.intent);
    navigate(&mut preview,ProductPage::Overview);
    assert_eq!(preview.press(&choose,InputPath::Keyboard),Outcome::Refused,
        "a control from the departed subpage cannot still run");
    navigate(&mut preview,ProductPage::Repair);
    assert_eq!(preview.state.plugins.focused_product(),Some(&key));
    assert!(offered(&preview,|a|matches!(a,Action::PluginOpen {..})).is_some(),
        "repair keeps its exact action, not a reference to another page");
    let blocked = preview.built.targets.iter().find_map(|(id,target)| matches!(target,
        Target::Offer {offer,..} if offer.availability.reason()==Some("Close this exact plug-in window before preparing its accessibility trial."))
        .then(||id.clone())).unwrap();
    assert_eq!(preview.press(&blocked,InputPath::Touch),Outcome::Refused);
    assert!(preview.captured.is_empty());
    navigate(&mut preview,ProductPage::Overview);
    assert_eq!(preview.state.plugins.focused_product(),Some(&key));
    assert!(preview.captured.is_empty());
}

#[test]
fn exact_instance_restart_confirms_loss_and_never_replays_a_cancelled_or_stale_choice() {
    let mut base=Preview::new(Selection {state:state_index("plugin_ready"),page:Page::Plugins,..Selection::default()});
    base.recovery_fixture();
    fn restart(preview:&Preview)->String {
        preview.built.targets.iter().find_map(|(id,target)|matches!(target,
            Target::Offer {offer,..} if matches!(offer.intent,Action::PluginRestart {..})).then(||id.clone())).unwrap()
    }
    for page in [Page::Plugins,Page::Activity] {
        let mut preview=base.clone();
        preview.state.page=page;
        preview.rebuild();
        let id=restart(&preview);
        preview.press(&id,InputPath::Touch);
        let action=preview.state.forms.confirm.clone().unwrap();
        assert!(preview.captured.is_empty());
        let rendered=format!("{:?}",preview.built.form);
        assert!(rendered.contains(lvb_view_model::recovery::LOSS_WARNING));
        let confirm=form_control(&preview,FormStep::Confirm).unwrap();
        let cancel=form_control(&preview,FormStep::CancelConfirm).unwrap();
        assert_eq!(preview.built.form_focus.as_deref(),Some(cancel.id.0.as_str()));
        let mut cancelled=preview.clone();
        cancelled.press(&cancel.id.0,InputPath::Pointer);
        assert!(cancelled.captured.is_empty() && cancelled.built.form.is_none());
        assert_eq!(cancelled.press(&confirm.id.0,InputPath::Keyboard),Outcome::Refused);
        let mut escaped=preview.clone();
        escaped.dismiss_form("Escape");
        assert!(escaped.captured.is_empty() && escaped.built.form.is_none());
        let mut stale=preview.clone();
        let offer=stale.sample.snapshot.products[0].actions.iter_mut().find(|offer|offer.action==action).unwrap();
        let Action::PluginRestart {snapshot,..}=&mut offer.action else {panic!()};
        snapshot.revision+=1;
        stale.rebuild();
        let held=form_control(&stale,FormStep::Confirm).unwrap();
        assert!(held.reason().is_some());
        assert_eq!(stale.press(&held.id.0,InputPath::Touch),Outcome::Refused);
        assert!(stale.captured.is_empty());
        preview.press(&confirm.id.0,InputPath::Keyboard);
        assert_eq!(preview.captured,vec![(action,true)]);
        assert_eq!(preview.press(&confirm.id.0,InputPath::Keyboard),Outcome::Refused);
        assert_eq!(preview.captured.len(),1);
    }
}

#[test]
fn presets_is_a_product_page_without_a_fake_file_browser() {
    use lvb_view_model::state::ProductPage;
    let mut preview=Preview::new(Selection {state:state_index("plugin_ready"),page:Page::Plugins,..Selection::default()});
    preview.display_fixture();
    let key=preview.state.plugins.focused_product().unwrap().clone();
    let id=preview.built.targets.iter().find_map(|(id,target)|matches!(target,
        Target::List(ListStep::ProductPage {page:ProductPage::Presets,..})).then(||id.clone())).unwrap();
    assert_eq!(preview.press(&id,InputPath::Touch),Outcome::Navigated);
    assert_eq!(preview.state.plugins.focused_product(),Some(&key));
    assert_eq!(preview.state.plugins.page,ProductPage::Presets);
    assert!(!preview.built.targets.values().any(|target|matches!(target,Target::Offer {offer,..}
        if matches!(offer.intent,Action::PresetFiles {..}))),"an unknown location cannot invent file actions");
    assert!(preview.captured.is_empty());
}
