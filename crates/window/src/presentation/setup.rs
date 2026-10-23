//! Console setup composition. Owner data and offers come from the shared view model.
use super::*;

impl Builder<'_> {
    pub(super) fn setup(&mut self, setup: &Setup, state: &State, update: Option<&updates::Panel>) -> Vec<Node> {
        let Setup { heading, add, add_label, daw, progress, existing, showing_one, cards, empty, missing,
            apps, deleted, footer, history } = setup;
        let focused = showing_one.is_some() && cards.len() == 1;
        let mut add = self.control(add);
        add.importance = Importance::Primary;
        let mut out = if focused { vec![] } else {
            vec![Node::Header(vec![Node::Title((*heading).into()), self.one(add.clone())]), text(progress)]
        };
        for card in cards { out.push(self.setup_card(card, focused)); }
        if let Some((_, show_all)) = showing_one {
            let show_all = self.control(show_all);
            out.push(self.one(show_all));
        }
        // A focused installation starts with its own state and continuation.
        // New-install choices stay available without pushing that state down.
        let options: Vec<ActionView> = daw.choices.iter().map(|(choice, name)|
            self.local(name, if Some(*choice) == state.forms.daw { Importance::Secondary } else { Importance::Quiet },
                Target::Daw(*choice), None)).collect();
        let mut install_choices = vec![words(Role::Body, daw.placeholder), self.actions("daw", options),
            words(Role::Strong, daw.shown(state.forms.daw))];
        if focused { install_choices.push(self.one(add)); }
        out.push(Node::Fold { id: "fold.new-installation".into(), title: (*add_label).into(),
            start_open: !focused && cards.is_empty() && state.forms.daw.is_none(), children: install_choices });
        if let Some(existing) = existing {
            // These live controls must survive a previously closed disclosure
            // and choosing or opening a different setup.
            for setup in &existing.setups {
                if setup.content.stop.is_some() {
                    let mut children = vec![words(Role::Strong, setup.label.clone())];
                    children.extend(self.row(&setup.content.controls.running));
                    out.push(Node::Panel { kind: PanelKind::Card, children });
                }
            }
            let content = self.existing(existing);
            out.push(Node::Fold { id: "fold.existing-setups".into(), title: existing.title.into(),
                start_open: false, children: vec![content] });
        }
        for line in empty {
            out.push(words(Role::Body, *line));
        }
        if let Some(missing) = missing {
            out.push(words(Role::Caution, *missing));
        }
        for group in apps {
            out.push(self.app_group(group));
        }
        if let Some(deleted) = deleted {
            out.push(self.deleted(deleted));
        }
        if let Some(panel) = update {
            out.push(self.update_panel(panel));
        }
        out.push(words(Role::Detail, *footer));
        let history = self.control(history);
        out.push(self.one(history));
        out
    }

    pub(super) fn setup_card(&mut self, card: &SetupCard, focused: bool) -> Node {
        let SetupCard { installer: _, name, title, status: said, journey, primary, discovered, secondary, latest,
            replace, draft, details, removal, actionable: _ } = card;
        let mut children = if focused { vec![text(said)] }
            else { vec![Node::Heading(name.clone()), words(Role::Strong, *title), text(said)] };
        if let Some(journey) = journey {
            children.extend(self.results(journey));
        }
        if let Some(primary) = primary {
            children.push(self.main_offer(primary, After::Nothing));
        }
        if let Some(discovered) = discovered {
            let view = self.control(discovered);
            children.push(self.one(view));
        }
        children.extend(self.row(secondary));
        if let Some((line, row)) = latest {
            children.push(words(Role::Body, line.clone()));
            children.extend(self.row(row));
        }
        match replace {
            Replace::Allowed(choices) => {
                let ReplaceChoices { dependency, another } = choices.as_ref();
                if let Some(RequiredApp { title, help, explanation, choose }) = dependency {
                    children.push(words(Role::Strong, *title));
                    for line in help {
                        children.push(words(Role::Body, line.clone()));
                    }
                    children.push(words(Role::Body, *explanation));
                    let choose = self.control(choose);
                    children.push(self.one(choose));
                }
                let another = self.control(another);
                children.push(self.one(another));
            }
            Replace::Waiting(reason) => children.push(words(Role::Detail, *reason)),
        }
        if let Some(ReplacementDraft { selected, keep }) = draft {
            if let Some(SelectedInstaller { title, same_setup, offer, focus_after }) = selected {
                children.push(Node::Rule);
                children.push(words(Role::Strong, title.clone()));
                if let Some((note, consequence, affected)) = same_setup {
                    children.push(words(Role::Body, *note));
                    children.push(words(Role::Body, consequence.clone()));
                    if let Some(affected) = affected {
                        children.push(words(Role::Strong, affected.clone()));
                    }
                }
                match offer {
                    Ok(offer) => children.push(self.main_offer(offer,
                        After::CloseReplacement { focus_after: focus_after.clone() })),
                    Err(reason) => children.push(words(Role::Caution, *reason)),
                }
            }
            let keep = self.control(keep);
            children.push(self.one(keep));
        }
        children.push(self.fold(details));
        if let Some(removal) = removal {
            children.push(self.fold(removal));
        }
        let panel = Node::Panel { kind: PanelKind::Card, children };
        if focused { Node::Stack(vec![Node::Label((*title).into()), Node::Title(name.clone()), panel]) }
        else { panel }
    }

    pub fn setup_controls(&mut self, controls: &SetupControls) -> Vec<Node> {
        let SetupControls { environment, help, programs_title, programs_hint, programs, running, consequence,
            affected, identity, runtime, last_installer, continuation, installers } = controls;
        let mut out = Vec::new();
        for line in help {
            out.push(words(Role::Body, line.clone()));
        }
        out.push(words(Role::Strong, *programs_title));
        out.push(words(Role::Detail, *programs_hint));
        out.extend(self.row(programs));
        out.extend(self.row(running));
        out.push(words(Role::Body, consequence.clone()));
        out.push(text(affected));
        out.push(self.fold(identity));
        if let Some(RuntimeChoice { title, current, consequence, affected_title, affected, placeholder, choices,
            none }) = runtime {
            let mut inner = vec![words(Role::Body, current.clone()), words(Role::Body, consequence.clone()),
                words(Role::Strong, *affected_title)];
            for owner in affected {
                inner.push(words(Role::Body, owner.clone()));
            }
            inner.extend(self.choice(&format!("runtime.{environment}"), placeholder, choices, None,
                |builder, row| builder.row(row)));
            if let Some(none) = none {
                inner.push(words(Role::Detail, *none));
            }
            let id = self.id("fold", title);
            out.push(Node::Fold { id, title: (*title).into(), start_open: false, children: inner });
        }
        if let Some(LastInstaller { state: said, installation, warning, details }) = last_installer {
            out.push(words(Role::Strong, said.clone()));
            if let Some(installation) = installation {
                out.push(words(Role::Body, installation.clone()));
            }
            if let Some(warning) = warning {
                out.push(words(Role::Caution, *warning));
            }
            out.push(self.fold(details));
        }
        out.extend(self.row(continuation));
        let InstallerChoice { preselected, placeholder, choices, none } = installers;
        out.extend(self.choice(&format!("installer.{environment}"), placeholder, choices, preselected.as_deref(),
            |builder, (row, advanced)| {
                let mut inner = builder.row(row);
                if let Some(advanced) = advanced {
                    inner.push(builder.fold(advanced));
                }
                inner
            }));
        if let Some(none) = none {
            out.push(words(Role::Detail, *none));
        }
        out
    }

    pub(super) fn existing(&mut self, existing: &ExistingSetups) -> Node {
        let ExistingSetups { title, placeholder, setups, actionable: _ } = existing;
        let mut children = vec![Node::Heading((*title).into())];
        children.extend(self.choice("existing", placeholder, setups, None, |builder, setup| {
            let ExistingSetup { controls, programs, stop } = setup;
            let mut folded = controls.clone();
            if stop.is_some() { folded.running = OfferRow::default(); }
            let mut inner = builder.setup_controls(&folded);
            for UserProgram { name, version, path, offers } in programs {
                inner.push(words(Role::Strong, name.clone()));
                inner.push(words(Role::Body, version.clone()));
                if let Some(path) = path {
                    inner.push(words(Role::Code, path.clone()));
                }
                inner.extend(builder.row(offers));
            }
            inner
        }));
        Node::Panel { kind: PanelKind::Card, children }
    }

    pub(super) fn app_group(&mut self, group: &AppGroup) -> Node {
        let AppGroup { environment, title, plugins, apps, picker, actionable: _ } = group;
        let mut children = vec![Node::Heading(title.clone())];
        if let Some(plugins) = plugins {
            children.push(words(Role::Body, plugins.clone()));
        }
        for AppCard { id: _, name, progress, hint, notes, offers } in apps {
            children.push(words(Role::Strong, name.clone()));
            if let Some((headline, results)) = progress {
                children.push(words(Role::Strong, *headline));
                children.extend(self.results(results));
            }
            if let Some(hint) = hint {
                children.push(words(Role::Body, *hint));
            }
            for note in notes {
                children.push(words(Role::Body, note.clone()));
            }
            children.extend(self.row(offers));
        }
        if let Some(ProgramPicker { title, hint, placeholder, choices, unchosen, forget_missing: _ }) = picker {
            children.push(words(Role::Strong, *title));
            children.push(words(Role::Body, *hint));
            let group_id = format!("program.{environment}");
            let chosen = self.ui.choices.get(&group_id).filter(|key| choices.iter().any(|c| &c.key == *key)).is_some();
            children.extend(self.choice(&group_id, placeholder, choices, None, |builder, open| match open {
                Some(ProgramOpen { consequence, offer }) => {
                    let mut inner = Vec::new();
                    if let Some(consequence) = consequence {
                        inner.push(words(Role::Body, consequence.clone()));
                    }
                    inner.push(builder.main_offer(offer, After::Nothing));
                    inner
                }
                None => vec![],
            }));
            if !chosen && !self.reveal {
                // Shown, not pressable, until a program is chosen.
                let view = self.local(unchosen, Importance::Primary, Target::Choose {
                    group: group_id.clone(), key: String::new() }, Some((*hint).into()));
                children.push(self.one(view));
            }
        }
        Node::Panel { kind: PanelKind::Card, children }
    }

    pub(super) fn deleted(&mut self, deleted: &DeletedSetups) -> Node {
        let DeletedSetups { title, start_open, setups, actionable: _ } = deleted;
        let mut children = Vec::new();
        for DeletedSetupView { environment: _, name, state: said, plugins, offers } in setups {
            children.push(words(Role::Strong, name.clone()));
            children.push(words(Role::Body, *said));
            if let Some(plugins) = plugins {
                children.push(words(Role::Detail, plugins.clone()));
            }
            children.extend(self.row(offers));
        }
        let id = self.id("fold", "deleted setups");
        Node::Fold { id, title: title.clone(), start_open: *start_open, children }
    }

}
