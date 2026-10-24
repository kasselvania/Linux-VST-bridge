//! Console plugins composition. Owner data and offers come from the shared view model.
use super::*;

impl Builder<'_> {
    pub(super) fn plugins(&mut self, not_current: Option<&Text>, list: &PluginList, single: bool, snapshot: &Snapshot,
        state: &State) -> Vec<Node> {
        let mut out = if single { vec![] } else { self.list_controls(state) };
        if let Some(not_current) = not_current {
            out.push(text(not_current));
        }
        let PluginList { summary, empty, groups } = list;
        if !single { for line in summary { out.push(words(Role::Detail, line.clone())); } }
        if let Some((title, hint)) = empty {
            out.push(Node::Heading((*title).into()));
            out.push(words(Role::Body, *hint));
        }
        for VendorGroup { heading, cards } in groups {
            if !single { if let Some(heading) = heading { out.push(Node::Label(heading.clone())); } }
            for card in cards {
                if single { out.push(match state.plugins.page {
                    vm::state::ProductPage::Overview => self.card(card, snapshot),
                    page => self.product_page(card, page, snapshot, state),
                }); }
                else {
                    let action = self.local(&card.name, Importance::Secondary,
                        Target::List(ListStep::Open(card.key.clone())), None);
                    out.push(Node::LibraryRow { action, detail: vm::fact_line(&card.facts),
                        status: Some(status(&card.status)) });
                }
            }
        }
        if single && state.plugins.page == vm::state::ProductPage::Overview {
            let view = self.local(plugins::SHOW_ALL, Importance::Quiet, Target::List(ListStep::ShowAll), None);
            out.push(self.one(view));
        }
        out
    }

    pub(super) fn list_controls(&mut self, state: &State) -> Vec<Node> {
        let mut out = vec![Node::Header(vec![Node::Title(plugins::HEADING.into()),
            Node::Field { secret: false, id: "field.search".into(), value: state.plugins.search.clone(),
                hint: plugins::SEARCH_HINT.into() }])];
        if !state.plugins.search.is_empty() {
            let view = self.local(plugins::CLEAR_SEARCH, Importance::Quiet, Target::List(ListStep::ClearSearch), None);
            out.push(self.one(view));
        }
        let roles: Vec<ActionView> = plugins::RoleFilter::CHOICES.iter().map(|role| self.local(role.label(),
            if *role == state.plugins.role { Importance::Secondary } else { Importance::Quiet }, Target::List(ListStep::Role(*role)), None)).collect();
        out.push(self.actions("roles", roles));
        let statuses: Vec<ActionView> = plugins::StatusFilter::CHOICES.iter().map(|filter| self.local(filter.label(),
            if *filter == state.plugins.status { Importance::Secondary } else { Importance::Quiet }, Target::List(ListStep::Status(*filter)), None)).collect();
        out.push(self.actions("statuses", statuses));
        if state.plugins.query().filtered() {
            let view = self.local(plugins::CLEAR_FILTERS, Importance::Quiet, Target::List(ListStep::ClearFilters), None);
            out.push(self.one(view));
        }
        out
    }

    pub(super) fn card(&mut self, card: &PluginCard, snapshot: &Snapshot) -> Node {
        let PluginCard { key, record, name, status: owner, facts, readiness: stages, notes, lead, offers, standalone, recovery, display: _, uninstallers, settings,
            help, availability, other_setups, previous_setups, shared_setup, technical } = card;
        let chip = self.id("chip", &format!("card {name}"));
        let mut children = vec![Node::Header(vec![
            Node::PluginIdentity { name: name.clone(), detail: vm::fact_line(facts) },
            Node::Chip { id: chip, status: status(owner) },
        ])];
        let mut readiness = vec![Node::Label("Is it ready?".into())];
        for stage in stages {
            let class = match stage.class {
                vm::StatusClass::Ready => StatusClass::Ready,
                vm::StatusClass::Working => StatusClass::Working,
                vm::StatusClass::NeedsYou => StatusClass::NeedsYou,
                vm::StatusClass::Problem => StatusClass::Problem,
                vm::StatusClass::Unknown => StatusClass::Unknown,
            };
            readiness.push(Node::Readiness { status: StatusView::new(class, stage.title.clone()),
                detail: stage.detail.clone() });
        }
        for note in notes { readiness.push(text(note)); }
        let mut actions = vec![Node::Label("What you can do".into())];
        for instance in recovery { actions.push(self.instance_recovery(instance)); }
        if let Some(LeadOffer { preface, offer }) = lead {
            if let Some(preface) = preface {
                actions.push(words(Role::Body, preface.clone()));
            }
            actions.push(self.main_offer(offer, After::Nothing));
        }
        let repair = self.local("Fix it step by step", Importance::Primary,
            Target::List(ListStep::ProductPage {key:key.clone(), page:vm::state::ProductPage::Repair}), None);
        let modes = self.local("Display mode", Importance::Secondary,
            Target::List(ListStep::ProductPage {key:key.clone(), page:vm::state::ProductPage::Display}), None);
        actions.push(self.actions("product-pages", vec![repair, modes]));
        if !standalone.is_empty() {
            actions.push(Node::Label("Outside your DAW".into()));
            actions.push(words(Role::Detail, "These plug-in windows do not open an audio device."));
            for row in standalone { actions.extend(self.row(row)); }
        }
        actions.extend(self.row(offers));
        for Uninstall { row, explanation } in uninstallers {
            actions.extend(self.row(row));
            actions.push(words(Role::Detail, explanation.clone()));
        }
        children.push(Node::Columns(vec![
            Node::Panel { kind: PanelKind::Card, children: readiness },
            Node::Panel { kind: PanelKind::Card, children: actions },
        ]));
        for section in [settings, help, availability, other_setups, previous_setups].into_iter().flatten() {
            children.push(self.fold(section));
        }
        if let Some(SharedSetup { title, start_open, controls }) = shared_setup {
            let inner = self.setup_controls(controls);
            let id = self.id("fold", title);
            children.push(Node::Fold { id, title: (*title).into(), start_open: *start_open, children: inner });
        }
        // The caller adds the plug-in's retained technical records.
        let records = record.and_then(|index| snapshot.products.get(index))
            .map(|product| self.details(&plugins::technical_records(product))).unwrap_or_default();
        children.push(self.fold_with(technical, records));
        Node::Stack(children)
    }

    fn product_page(&mut self, card: &PluginCard, page: vm::state::ProductPage, snapshot: &Snapshot, state: &State) -> Node {
        use vm::state::ProductPage;
        let title = match page { ProductPage::Display => "Display mode", ProductPage::Repair => "Fix it step by step", ProductPage::Overview => unreachable!() };
        let back = self.local("Back to plug-in", Importance::Quiet,
            Target::List(ListStep::ProductPage {key:card.key.clone(), page:ProductPage::Overview}), None);
        let mut out = vec![Node::Header(vec![Node::PluginIdentity {name:card.name.clone(), detail:title.into()}, self.one(back)])];
        if let Some(notice) = &card.display.notice { out.push(text(notice)); }
        if page == ProductPage::Display {
            out.push(words(Role::Body, "Choose the display mode and Windows accessibility setting, then use them for a test."));
            if let Some(settings) = &card.display.settings {
                out.push(words(Role::Body, if settings.has_previous_settings {
                    "Your previous settings are kept."
                } else {"These settings make the plug-in available to test."}));
            }
            let selected = card.display.settings.as_ref().map(|settings| self.display_selection(settings));
            let mut modes = vec![];
            for mode in &card.display.modes {
                let mut children = vec![Node::Label(mode.label.clone()), words(Role::Detail, mode.posture.clone()), text(&mode.result)];
                if let (Some(settings), Some((graphics, _))) = (&card.display.settings, &selected) {
                    let chosen = *graphics == mode.backend;
                    let choose = self.local(if chosen {"Chosen"} else {"Choose this mode"},
                        if chosen {Importance::Primary} else {Importance::Secondary},
                        Target::Choose {group:format!("display-mode:{}", settings.basis),
                            key:serde_json::to_string(&mode.backend).unwrap()}, None);
                    children.push(self.one(choose));
                }
                let mut prepared = vec![];
                let mut history = vec![];
                for trial in &mode.trials {
                    let trial_nodes = self.display_trial(trial);
                    if trial.current { children.extend(trial_nodes); }
                    else if trial.prepared { prepared.extend(trial_nodes); }
                    else { history.extend(trial_nodes); }
                }
                if !prepared.is_empty() {
                    let id = self.id("fold", &format!("{} prepared settings", mode.label));
                    children.push(Node::Fold {id, title:"Choose a prepared setting…".into(), start_open:false, children:prepared});
                }
                if let Some(record) = &mode.result_record { history.push(self.fold(record)); }
                if !history.is_empty() {
                    let id = self.id("fold", &format!("{} display history", mode.label));
                    children.push(Node::Fold {id, title:"Earlier checks and details".into(), start_open:false, children:history});
                }
                modes.push(Node::Panel {kind:PanelKind::Card, children});
            }
            out.push(Node::Columns(modes));
            if let (Some(settings), Some((graphics, accessibility))) = (&card.display.settings, selected) {
                let mut controls = vec![Node::Label("Windows accessibility support".into()),
                    words(Role::Detail, format!("Currently: {}{}.",
                        plugins::display::accessibility_words(settings.effective_accessibility),
                        if settings.current.accessibility == vm::model::AccessibilityChoice::ProfileDefault {
                            " (automatic setting)"
                        } else {""})),
                    words(Role::Detail, "Turning it off can help some plug-in windows. This changes only the plug-in process; Linux input settings are unchanged.")];
                let options = [
                    (vm::model::AccessibilityChoice::WindowsDefault, "On · Windows defaults"),
                    (vm::model::AccessibilityChoice::DisabledForHost, "Off for this plug-in"),
                ].into_iter().map(|(choice, label)| self.local(label,
                    if choice == accessibility {Importance::Primary} else {Importance::Secondary},
                    Target::Choose {group:format!("display-accessibility:{}", settings.basis),
                        key:serde_json::to_string(&choice).unwrap()}, None)).collect();
                controls.push(self.actions("display-accessibility", options));
                let wanted = vm::model::LocalSettings {graphics,accessibility};
                if let Some(choice) = settings.choices.iter().find(|choice| choice.settings == wanted) {
                    controls.extend(self.display_actions(&choice.action));
                } else {
                    controls.push(words(Role::Detail, if wanted == settings.current {
                        "These are the current settings. Choose a change to test."
                    } else {"This exact settings choice is not currently offered. Refresh the plug-in controls."}));
                }
                out.push(Node::Panel {kind:PanelKind::Card, children:controls});
            }
        } else {
            out.push(words(Role::Body, "Try one change at a time. Review what each step changes before continuing."));
            let mut rows = vec![];
            for (index, step) in card.display.repairs.iter().enumerate() {
                let (label, detail, actions) = match step.kind {
                    vm::model::RepairStepKind::TryDisplayMode => {
                        let choose = self.local("Choose…", Importance::Secondary,
                            Target::List(ListStep::ProductPage {key:card.key.clone(), page:ProductPage::Display}), None);
                        ("Choose another display mode", "Compare the available modes and their last editor checks.", vec![self.one(choose)])
                    }
                    vm::model::RepairStepKind::RepairInstallation =>
                        (step.label.as_str(), step.detail.as_str(), self.installer_repair(snapshot, &card.key, state)),
                    _ => (step.label.as_str(), step.detail.as_str(), self.display_actions(&step.actions)),
                };
                let title = Node::Stack(vec![Node::Label(format!("{:02}  {}", index + 1, label)), words(Role::Detail, detail)]);
                rows.push(Node::Columns(vec![title, Node::Stack(actions)]));
            }
            if !card.display.repairs.iter().any(|step|step.kind == vm::model::RepairStepKind::RepairInstallation) {
                rows.push(Node::Stack(vec![Node::Label("Repair installation".into()),
                    Node::Stack(self.installer_repair(snapshot, &card.key, state))]));
            }
            out.push(Node::Panel {kind:PanelKind::Card, children:rows});
            // A diagnostic report is separate from recording or finishing the
            // exact test result above. Opening Support sends neither operation.
            let support = self.control(&Control::go("Make a support report…", offers::Importance::Secondary, vm::Destination::Diagnostics));
            out.push(self.one(support));
        }
        if let Some(provenance) = &card.display.provenance { out.push(self.fold(provenance)); }
        Node::Stack(out)
    }
    fn installer_repair(&mut self, snapshot: &Snapshot, key: &vm::ProductKey, state: &State) -> Vec<Node> {
        let repair = match plugins::repair::build(snapshot, key, state.focus.repair.as_ref(),
            self.why.fresh && !self.why.superseded, self.why.pending) {
            Ok(repair) => repair,
            Err(reason) => return vec![words(Role::Detail, reason)],
        };
        let mut out = vec![words(Role::Body, "Choose the installer for this plug-in. The bridge will run it in this same setup."),
            words(Role::Detail, "The maker's installer decides what changes, including presets and shared components.")];
        let choose = self.control(&repair.choose);
        out.push(self.one(choose));
        if !repair.retained.is_empty() {
            let mut retained = vec![];
            for (name, control) in repair.retained {
                let choose = self.control(&control);
                retained.push(Node::Stack(vec![words(Role::Strong, name), self.one(choose)]));
            }
            let id = self.id("fold", &format!("repair-installers:{key:?}"));
            out.push(Node::Fold {id,title:"Choose an installer already added…".into(),start_open:false,children:retained});
        }
        if let Some(name) = repair.selected_name {out.push(words(Role::Strong, format!("Selected: {name}")));}
        if let Some(selected) = repair.selected {
            match selected {
                Ok(offer) => out.push(self.main_offer(&offer,After::Nothing)),
                Err(reason) => out.push(words(Role::Caution, reason)),
            }
        }
        out
    }
    fn display_selection(&self, settings: &plugins::display::DisplaySettings)
        -> (Option<vm::model::GraphicsBackend>, vm::model::AccessibilityChoice) {
        let graphics = self.ui.choices.get(&format!("display-mode:{}", settings.basis))
            .and_then(|value| serde_json::from_str(value).ok()).unwrap_or(settings.current.graphics);
        let effective = match settings.effective_accessibility {
            vm::model::EffectiveAccessibility::WindowsDefault => vm::model::AccessibilityChoice::WindowsDefault,
            vm::model::EffectiveAccessibility::DisabledForHost => vm::model::AccessibilityChoice::DisabledForHost,
        };
        let accessibility = self.ui.choices.get(&format!("display-accessibility:{}", settings.basis))
            .and_then(|value| serde_json::from_str(value).ok())
            .filter(|choice| *choice != vm::model::AccessibilityChoice::ProfileDefault).unwrap_or(effective);
        (graphics, accessibility)
    }
    fn display_trial(&mut self, trial: &plugins::display::DisplayTrial) -> Vec<Node> {
        let mut out = vec![];
        if !trial.current {
            out.push(Node::Label(trial.posture.clone()));
            out.push(text(&trial.result));
        }
        if let Some(evidence) = &trial.evidence { out.push(text(evidence)); }
        out.extend(self.display_actions(&trial.actions));
        let technical = self.display_actions(&trial.technical);
        out.push(self.fold_with(&trial.identity, technical));
        out
    }
    fn display_actions(&mut self, row: &OfferRow) -> Vec<Node> {
        if row.offers.is_empty() { row.notes.iter().map(|note| words(Role::Detail, note.clone())).collect() }
        else { self.row(row) }
    }

    pub(super) fn loading(&mut self, list: &LoadingList, state: &State) -> Vec<Node> {
        let LoadingList { focus_title, notice, retry, count, empty, rows } = list;
        let focused = focus_title.is_some();
        let mut out = if focused { vec![] } else { self.list_controls(state) };
        if focused {
            for row in rows.iter().filter(|row| row.manage.is_none()) {
                out.push(Node::PluginIdentity { name: row.name.clone(), detail: vm::fact_line(&row.facts) });
            }
        }
        out.push(text(notice));
        if let Some(retry) = retry {
            let view = self.control(&Control { label: retry, importance: offers::Importance::Secondary,
                enabled: true, does: Local::RetryDetails });
            out.push(self.one(view));
        }
        if !focused { out.push(words(Role::Detail, count.clone())); }
        if let Some(empty) = empty {
            out.push(words(Role::Body, *empty));
        }
        for LoadingRow { key, name, facts, status: said, status_class, notes, manage, .. } in rows {
            if manage.is_none() {
                if let Some(said) = said { out.push(words(Role::Strong, *said)); }
                for note in notes { out.push(text(note)); }
            } else {
                let action = self.local(name, Importance::Secondary, Target::List(ListStep::Open(key.clone())), None);
                out.push(Node::LibraryRow { action, detail: vm::fact_line(facts),
                    status: said.map(|said| StatusView::new(class(*status_class), said)) });
            }
        }
        if focused {
            let view = self.local(plugins::SHOW_ALL, Importance::Quiet, Target::List(ListStep::ShowAll), None);
            out.push(self.one(view));
        }
        out
    }

}
