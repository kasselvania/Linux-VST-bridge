//! Console home composition. Owner data and offers come from the shared view model.
use super::*;

impl Builder<'_> {
    pub(super) fn home(&mut self, home: &Home) -> Vec<Node> {
        let Home { heading, add, freshness, next, summary: _, plugins, computer } = home;
        let mut add = self.control(add);
        add.importance = Importance::Primary;
        let mut out = if plugins.is_empty() {
            vec![Node::Label("Home".into()), Node::Title("Add your first plug-in".into()),
                words(Role::Lead, "Choose a Windows installer you downloaded from the plug-in maker. The manager runs it in a managed setup and connects it to your native Linux DAW."),
                Node::Panel { kind: PanelKind::Raised, children: vec![
                    words(Role::Strong, "A Windows .exe or .msi installer"), self.one(add)] },
                Node::Columns(vec![
                    Node::Panel { kind: PanelKind::Card, children: vec![Node::Label("01 · Install".into()),
                        words(Role::Body, "Use the maker’s installer and sign in through their own windows.")] },
                    Node::Panel { kind: PanelKind::Card, children: vec![Node::Label("02 · Publish".into()),
                        words(Role::Body, "The manager connects discovered plug-ins to your DAW.")] },
                    Node::Panel { kind: PanelKind::Card, children: vec![Node::Label("03 · Play".into()),
                        words(Role::Body, "Load a plug-in on a track, then check its sound, editor and saved settings.")] },
                ])]
        } else { vec![Node::Header(vec![Node::Title((*heading).into()), self.one(add)])] };
        if let Some(freshness) = freshness {
            out.push(text(freshness));
            let refresh = self.control(&Control { label: "Try again", importance: offers::Importance::Secondary,
                enabled: !self.why.pending, does: Local::Refresh });
            out.push(self.one(refresh));
        }
        // Each product is its own destination. A setup journey must not put
        // healthy siblings into another product's problem callout.
        if matches!(next, Next::Service { .. } | Next::Computer { .. } | Next::Setup { .. }
            | Next::Recovery { .. }) || plugins.is_empty() {
            out.push(self.next(next));
        }
        for plugin in plugins { out.push(self.home_plugin(plugin)); }
        out.push(Node::Fold { id: "fold.computer".into(), title: "Computer setup".into(),
            start_open: plugins.is_empty(), children: vec![self.computer(computer)] });
        out
    }

    pub(super) fn next(&mut self, next: &Next) -> Node {
        match next {
            Next::Service { title, what_happened, offers, review } => {
                let mut children = vec![Node::Heading((*title).into()), words(Role::Body, *what_happened)];
                children.extend(self.row(offers));
                let review = self.control(review);
                children.push(self.one(review));
                Node::Panel { kind: PanelKind::Recovery, children }
            }
            Next::Setup { title, status: said, journey: _, primary, open } => {
                let mut children = vec![Node::Heading(title.clone()), words(Role::Body, said.clone())];
                if let Some(primary) = primary {
                    children.push(self.main_offer(primary, After::Nothing));
                }
                let open = self.control(open);
                children.push(self.one(open));
                Node::Panel { kind: PanelKind::Card, children }
            }
            Next::Computer { note, title, detail, action, blocker, review } => {
                let mut children = vec![words(Role::Detail, *note), Node::Heading(title.clone())];
                if let Some(detail) = detail {
                    children.push(words(Role::Body, detail.clone()));
                }
                if let Some(action) = action {
                    children.push(self.main_offer(action, After::Nothing));
                }
                if let Some(blocker) = blocker {
                    children.push(words(Role::Caution, blocker.clone()));
                }
                let review = self.control(review);
                children.push(self.one(review));
                Node::Panel { kind: PanelKind::Card, children }
            }
            Next::Recovery { title, detail, open } | Next::Go { title, detail, open } => {
                let mut children = vec![Node::Heading(title.clone())];
                if let Some(detail) = detail {
                    children.push(words(Role::Body, detail.clone()));
                }
                let open = self.control(open);
                children.push(self.one(open));
                Node::Panel { kind: PanelKind::Card, children }
            }
            Next::Ready { title, help } => {
                let help = self.fold(help);
                Node::Panel { kind: PanelKind::Card, children: vec![Node::Heading(title.clone()), help] }
            }
        }
    }

    pub(super) fn home_plugin(&mut self, plugin: &HomePlugin) -> Node {
        let mut action = self.control(&plugin.manage);
        action.label = plugin.name.clone();
        Node::LibraryRow { action,
            detail: plugin.problem.clone().unwrap_or_else(|| plugin.subtitle.clone()),
            status: plugin.status.map(|said| status(&vm::Status { words: said, class: plugin.status_class })),
        }
    }

    pub(super) fn computer(&mut self, computer: &ComputerCheck) -> Node {
        let ComputerCheck { title, intro, check, result } = computer;
        let check = self.control(check);
        let mut children = vec![Node::Heading((*title).into()), words(Role::Lead, *intro), self.one(check)];
        if let Some(CheckResult { blockers, export, export_note, observations }) = result {
            for blocker in blockers {
                children.push(words(Role::Caution, blocker.clone()));
            }
            children.extend(self.row(export));
            children.push(words(Role::Detail, *export_note));
            children.push(self.fold(observations));
        }
        Node::Panel { kind: PanelKind::Card, children }
    }

}
