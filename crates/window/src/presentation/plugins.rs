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
                    page => self.product_page(card, page),
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
        let PluginCard { key, record, name, status: owner, facts, readiness: stages, notes, lead, offers, standalone, recovery, display: _, locations: _, uninstallers, settings,
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
        let locations = self.local("Sample locations", Importance::Secondary,
            Target::List(ListStep::ProductPage {key:key.clone(), page:vm::state::ProductPage::SampleLocations}), None);
        actions.push(self.one(locations));
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

    fn product_page(&mut self, card: &PluginCard, page: vm::state::ProductPage) -> Node {
        use vm::state::ProductPage;
        let title = match page { ProductPage::Display => "Display mode", ProductPage::Repair => "Fix it step by step", ProductPage::SampleLocations => "Sample locations", ProductPage::Overview => unreachable!() };
        let back = self.local("Back to plug-in", Importance::Quiet,
            Target::List(ListStep::ProductPage {key:card.key.clone(), page:ProductPage::Overview}), None);
        let name = if page == ProductPage::SampleLocations { &card.locations.setup } else { &card.name };
        let mut out = vec![Node::Header(vec![Node::PluginIdentity {name:name.clone(), detail:title.into()}, self.one(back)])];
        if page == ProductPage::SampleLocations {
            if !card.locations.members.is_empty() { out.push(words(Role::Detail, card.locations.members.join(", "))); }
            out.extend(self.library_locations(&card.locations));
            return Node::Stack(out);
        }
        if let Some(notice) = &card.display.notice { out.push(text(notice)); }
        if page == ProductPage::Display {
            out.push(words(Role::Body, "Prepare a setting, then choose whether to use it. Your current setup stays selected until you change it."));
            let mut modes = vec![];
            for mode in &card.display.modes {
                let mut children = vec![Node::Label(mode.label.clone()), words(Role::Detail, mode.posture.clone()), text(&mode.result)];
                children.extend(self.display_actions(&mode.prepare));
                let mut history = vec![];
                for trial in &mode.trials {
                    let mut trial_nodes = vec![Node::Label(trial.posture.clone()), text(&trial.evidence)];
                    trial_nodes.extend(self.display_actions(&trial.actions));
                    trial_nodes.push(self.fold(&trial.identity));
                    if trial.current { children.extend(trial_nodes); } else { history.extend(trial_nodes); }
                }
                if let Some(record) = &mode.result_record { history.push(self.fold(record)); }
                if !history.is_empty() {
                    let id = self.id("fold", &format!("{} display history", mode.label));
                    children.push(Node::Fold {id, title:"Earlier checks and details".into(), start_open:false, children:history});
                }
                modes.push(Node::Panel {kind:PanelKind::Card, children});
            }
            out.push(Node::Columns(modes));
        } else {
            out.push(words(Role::Body, "Try one change at a time. Review what each step changes before continuing."));
            let mut rows = vec![];
            for (index, step) in card.display.repairs.iter().enumerate() {
                let title = Node::Stack(vec![Node::Label(format!("{:02}  {}", index + 1, step.label)), words(Role::Detail, step.detail.clone())]);
                let actions = self.display_actions(&step.actions);
                rows.push(Node::Columns(vec![title, Node::Stack(actions)]));
            }
            out.push(Node::Panel {kind:PanelKind::Card, children:rows});
        }
        if let Some(provenance) = &card.display.provenance { out.push(self.fold(provenance)); }
        Node::Stack(out)
    }
    pub(super) fn library_locations(&mut self, view: &vm::locations::LocationsView) -> Vec<Node> {
        let mut folders = vec![Node::Label("Folders this setup can use".into()),
            Node::Panel {kind:PanelKind::Card, children:vec![
                Node::Header(vec![Node::Label("C:".into()), Node::Label("The setup’s own drive".into())]),
                words(Role::Detail, "Programs and settings. Managed for you."),
            ]}];
        if let Some(notice) = &view.notice { folders.push(text(notice)); }
        for folder in &view.folders {
            // A stacked path can wrap on Deck without compressing the controls.
            let mut row = vec![Node::Header(vec![Node::Label(folder.drive.clone()), Node::Label(folder.name.clone())]),
                words(Role::Code, folder.path.clone()), words(Role::Detail, folder.mode), text(&folder.availability)];
            row.extend(self.row(&folder.remove));
            folders.push(Node::Panel {kind:PanelKind::Card, children:row});
        }
        folders.extend(self.row(&view.add));
        vec![Node::Panel {kind:PanelKind::Card, children:folders},
            words(Role::Body, "Choose the assigned drive letter in your plug-in or vendor app to locate existing samples. The vendor still manages library registration and authorization."),
            words(Role::Detail, "Removing access keeps the folder and its files."),
            words(Role::Detail, vm::locations::UNAVAILABLE_GUIDANCE)]
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
