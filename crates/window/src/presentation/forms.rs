//! Modal forms and explicit confirmation.
use super::*;

impl Builder<'_> {
    // ----- the forms a pressed offer opened -----

    /// The open form, if any, with the control it focuses first.
    pub fn form(&mut self, state: &State, snapshot: &Snapshot, actions: Option<&Snapshot>)
        -> Option<(Vec<Node>, Option<String>)> {
        let availability = if self.why.superseded {
            Availability::Refused(vm::window::superseded().title.into())
        } else if self.why.form_stale {
            Availability::Refused("This form is no longer current.".into())
        } else if self.why.pending || self.why.in_flight {
            Availability::Waiting
        } else if !self.why.fresh {
            Availability::Held
        } else {
            Availability::Enabled
        };
        let forms = &state.forms;
        let (mut nodes, focus) = if let Some(form) = &forms.confirm {
            self.confirmation(form, forms.forget_installer, snapshot, &availability, state.focus.repair.as_ref())
        } else if let Some(form) = &forms.install {
            self.install_form(form, forms.daw, actions, &availability)
        } else if let Some(form) = &forms.rename {
            self.rename_form(form, &availability)
        } else if let Some(form) = &forms.record {
            self.record_form(form, snapshot, &availability)
        } else {
            return None;
        };
        if self.why.form_stale {
            // Keep the return path visible even when an owner refusal or an
            // incomplete field supplies the submit button's own reason.
            nodes.insert(1, words(Role::Body, "The manager's status changed since this form was opened. Close this dialog and choose the action again from current status."));
        }
        Some((nodes, focus))
    }

    pub(super) fn confirmation(&mut self, form: &Action, forget: bool, snapshot: &Snapshot, availability: &Availability,
        repair: Option<&vm::plugins::repair::InstallerDraft>)
        -> (Vec<Node>, Option<String>) {
        let forms::Confirmation { title, body, confirm, cancel, offer, forget: choice } =
            forms::confirmation_with_repair(Some(snapshot), form, repair);
        let mut out = vec![Node::Title(title.into())];
        for line in body {
            out.push(words(Role::Body, line));
        }
        if let Some((choice, meaning)) = choice {
            let label = format!("{choice}: {}", if forget { "on" } else { "off" });
            let view = self.local_keyed(choice, &label, Importance::Secondary, Target::Form(FormStep::ToggleForget),
                None);
            out.push(self.one(view));
            out.push(words(Role::Detail, meaning));
        }
        let reason = match &offer {
            Err(reason) => Some(reason.clone()),
            Ok(_) => availability.reason().map(str::to_owned),
        };
        let importance = if form.environment_installer_start().is_some()
            || matches!(form, Action::EnvironmentRuntimeSelect { .. } | Action::CandidateSettingsTest { .. }) { Importance::Primary }
            else { Importance::Destructive };
        let confirm = self.local(confirm, importance, Target::Form(FormStep::Confirm), reason);
        let cancel = self.local(cancel, Importance::Secondary, Target::Form(FormStep::CancelConfirm), None);
        let focus = cancel.id.0.clone();
        out.push(self.actions("confirm", vec![cancel, confirm]));
        (out, Some(focus))
    }

    pub(super) fn install_form(&mut self, form: &Action, daw: Option<DawChoice>, actions: Option<&Snapshot>, availability: &Availability)
        -> (Vec<Node>, Option<String>) {
        let forms::InstallForm { title, question, daw: picker, explanation, note, submit, refusal, cancel } =
            forms::InstallForm::build(actions, form);
        let mut out = vec![Node::Title(title.into()), words(Role::Body, question)];
        let options: Vec<ActionView> = picker.choices.iter().map(|(choice, name)|
            self.local(name, Importance::Secondary, Target::Form(FormStep::ChooseDaw(*choice)), None)).collect();
        let first = options.first().map(|option| option.id.0.clone());
        out.push(self.actions("install-daw", options));
        out.push(words(Role::Strong, format!("DAW: {}", picker.shown(daw))));
        out.push(words(Role::Body, explanation));
        out.push(words(Role::Detail, note));
        let reason = refusal.clone()
            .or_else(|| daw.is_none().then(|| picker.placeholder.to_string()))
            .or_else(|| availability.reason().map(str::to_owned));
        let submit = self.local(submit, Importance::Primary, Target::Form(FormStep::StartInstall), reason);
        let cancel = self.local(cancel, Importance::Secondary, Target::Form(FormStep::CancelInstall), None);
        out.push(self.actions("install", vec![submit, cancel]));
        (out, first)
    }

    pub(super) fn rename_form(&mut self, form: &Action, availability: &Availability) -> (Vec<Node>, Option<String>) {
        let Action::InstallerRename { installer, label } = form else { return (vec![], None) };
        let forms::RenameForm { title, explanation, identity, save, cancel } = forms::RenameForm::build(installer);
        let mut out = vec![Node::Title(title.into()), words(Role::Body, explanation), words(Role::Code, identity)];
        out.push(Node::Field { secret: false, id: "field.rename".into(), value: label.clone(), hint: "Installer name".into() });
        // The view model's check has no words of its own; the preview says what it checks.
        let reason = if !forms::plausible_installer_label(label) {
            Some("Use a name of at most 96 UTF-8 bytes, with no surrounding spaces, control or direction-formatting characters, /, \\, or :. The name cannot be '.' or '..'.".to_string())
        } else {
            availability.reason().map(str::to_owned)
        };
        let save = self.local(save, Importance::Primary, Target::Form(FormStep::SaveRename), reason);
        let cancel = self.local(cancel, Importance::Secondary, Target::Form(FormStep::CancelRename), None);
        out.push(self.actions("rename", vec![save, cancel]));
        (out, None)
    }

    pub(super) fn record_form(&mut self, form: &Action, snapshot: &Snapshot, availability: &Availability) -> (Vec<Node>, Option<String>) {
        let words_ = forms::result_form();
        let expert = forms::expert_form();
        let mut out = Vec::new();
        let cancel_label;
        let (submit_label, valid, refusal): (&str, bool, Option<String>) = match form {
            Action::CompatibilityResult { candidate, expected_current, result, passed, failed_area, note } => {
                cancel_label = words_.cancel;
                out.push(Node::Title(words_.title.into()));
                out.push(words(Role::Body, words_.intro));
                let worked = matches!(result, TestResultKind::Worked);
                let kinds = vec![
                    self.local_keyed(words_.worked, &format!("{}{}", words_.worked, if worked { " (chosen)" } else { "" }),
                        Importance::Secondary, Target::Form(FormStep::ResultWorked), None),
                    self.local_keyed(words_.problem, &format!("{}{}", words_.problem, if worked { "" } else { " (chosen)" }),
                        Importance::Secondary, Target::Form(FormStep::ResultProblem), None),
                ];
                out.push(self.actions("result-kind", kinds));
                if let TestResultKind::Problem { category } = result {
                    out.push(Node::Label(format!("Problem: {}", forms::problem_category_label(*category))));
                    let categories: Vec<ActionView> = forms::PROBLEM_CATEGORIES.iter().map(|choice| self.local(
                        forms::problem_category_label(*choice), Importance::Secondary,
                        Target::Form(FormStep::Category(*choice)), None)).collect();
                    out.push(self.actions("categories", categories));
                    if *category == ProblemCategory::Other {
                        out.push(words(Role::Body, words_.area_question));
                        out.push(words(Role::Strong, failed_area.map(forms::test_area_label)
                            .unwrap_or(words_.area_placeholder)));
                        let areas: Vec<ActionView> = forms::test_areas().iter().map(|area| self.local(
                            forms::test_area_label(*area), Importance::Secondary,
                            Target::Form(FormStep::FailedArea(*area)), None)).collect();
                        out.push(self.actions("failed-area", areas));
                    }
                }
                out.push(Node::Label(words_.checks.into()));
                let checks: Vec<ActionView> = forms::test_areas().iter().map(|area| {
                    let on = passed.contains(area);
                    self.local_keyed(forms::test_area_label(*area), &format!("{}: {}", forms::test_area_label(*area),
                        if on { "confirmed" } else { "not confirmed" }), Importance::Secondary,
                        Target::Form(FormStep::Passed(*area)), None)
                }).collect();
                out.push(self.actions("checks", checks));
                out.push(words(Role::Body, words_.note));
                out.push(Node::Field { secret: false, id: "field.note".into(), value: note.clone(), hint: String::new() });
                let offer = forms::compatibility_result_offer(Some(snapshot), candidate, expected_current);
                let valid = forms::result_complete(result, passed, *failed_area, note);
                let refusal = forms::compatibility_result_refusal(Some(&offer), result).map(str::to_owned);
                if !valid {
                    out.push(words(Role::Detail, words_.incomplete));
                }
                if matches!(result, TestResultKind::Worked) && refusal.is_some()
                    && forms::compatibility_result_refusal(Some(&offer),
                        &TestResultKind::Problem {category:ProblemCategory::Other}).is_none() {
                    out.push(words(Role::Detail, words_.problem_only));
                }
                let reason = refusal.clone().or_else(|| (!valid).then(|| words_.incomplete.to_string()));
                (words_.submit, valid && refusal.is_none(), reason)
            }
            Action::CandidateObserve { area, status: observed, note, .. } => {
                cancel_label = expert.cancel;
                out.push(Node::Title(expert.title.into()));
                out.push(words(Role::Body, expert.observe_intro));
                out.push(words(Role::Strong, format!("Area: {}", area.replace('_', " "))));
                let areas: Vec<ActionView> = expert.areas.iter().map(|a| self.local(&a.replace('_', " "),
                    Importance::Secondary, Target::Form(FormStep::ExpertArea(a)), None)).collect();
                out.push(self.actions("expert-area", areas));
                out.push(words(Role::Strong, format!("Status: {}", observed.replace('_', " "))));
                let statuses: Vec<ActionView> = expert.statuses.iter().map(|s| self.local(&s.replace('_', " "),
                    Importance::Secondary, Target::Form(FormStep::ExpertStatus(s)), None)).collect();
                out.push(self.actions("expert-status", statuses));
                out.push(Node::Field { secret: false, id: "field.note".into(), value: note.clone(), hint: String::new() });
                let valid = !note.trim().is_empty();
                (expert.observe_submit, valid, (!valid).then(|| "Describe what you observed.".to_string()))
            }
            Action::CandidateReview { accept, rationale, .. } => {
                cancel_label = expert.cancel;
                out.push(Node::Title(expert.title.into()));
                out.push(words(Role::Body, if *accept { expert.accept_intro } else { expert.needs_work_intro }));
                out.push(Node::Field { secret: false, id: "field.note".into(), value: rationale.clone(), hint: String::new() });
                let valid = !rationale.trim().is_empty();
                (expert.review_submit, valid, (!valid).then(|| "Write the reason for this decision.".to_string()))
            }
            _ => {
                cancel_label = words_.cancel;
                (words_.submit, false, Some(offers::WAITING.to_string()))
            }
        };
        let reason = refusal.filter(|_| !valid).or_else(|| availability.reason().map(str::to_owned));
        let submit = self.local(submit_label, Importance::Primary, Target::Form(FormStep::SubmitRecord), reason);
        let cancel = self.local(cancel_label, Importance::Secondary, Target::Form(FormStep::CancelRecord), None);
        out.push(self.actions("record", vec![submit, cancel]));
        (out, None)
    }

}
