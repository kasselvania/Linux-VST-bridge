//! Console frame composition. Owner data and offers come from the shared view model.
use super::*;

impl Builder<'_> {
    // ----- the frame -----

    /// Compact product navigation and service health. Diagnostics live in
    /// Support; only actionable request or supersession feedback heads a page.
    pub fn frame(&mut self, frame: &Frame, page: Page) -> (Vec<Node>, Vec<Node>) {
        let Frame { title, superseded, preview, health, headline, request } = frame;
        let TitleRow { title, refreshing, .. } = title;
        let HealthBar { status: owner, note: health_note, last_observed, .. } = health;
        let selected = Tab::of(page);
        let tabs: Vec<ActionView> = Tab::ALL
            .iter()
            .map(|tab| self.local(tab.label(), Importance::Quiet, Target::Tab(*tab), None))
            .collect();
        let health_chip = Node::Chip { id: "chip.health".into(), status: if *last_observed {
            StatusView::new(class(owner.class), format!("{} · {health_note}", owner.words))
        } else { status(owner) } };
        let header = vec![Node::Header(vec![
            Node::Brand("Linux Plugin Bridge".into()),
            Node::Tabs { id: "tabs".into(), tabs,
                selected: Tab::ALL.iter().position(|tab| *tab == selected).unwrap() },
            health_chip,
        ])];
        let mut top = Vec::new();
        if let Some(Superseded { title, explanation, open }) = superseded {
            let open = self.control(open);
            top.push(Node::Panel { kind: PanelKind::Raised, children: vec![
                Node::Heading((*title).into()), words(Role::Body, *explanation), self.one(open),
            ] });
        }
        let _ = (title, preview, refreshing);
        if let Some((line, view)) = headline {
            let view = self.control(view);
            top.push(words(Role::Strong, line.clone()));
            top.push(self.one(view));
        }
        if let Some(bar) = request {
            if !(page == Page::Diagnostics && self.ui.report.is_some() && bar.report.is_some()) {
                top.push(self.request(bar));
            }
        }
        (header, top)
    }

    pub(super) fn service_details(&mut self, frame: &Frame) -> Vec<Node> {
        let TitleRow { version, note, refresh, .. } = &frame.title;
        let HealthBar { show, facts, unknown, separate, .. } = &frame.health;
        let mut details = Vec::new();
        if let Some(version) = version { details.push(words(Role::Code, version.clone())); }
        if let Some(note) = note { details.push(words(Role::Detail, note.clone())); }
        match facts {
            Some(facts) => details.push(Node::Facts { id: "facts.service".into(),
                section: FactSectionView { title: String::new(), rows: fact_rows(facts), empty: None } }),
            None => details.push(words(Role::Detail, *unknown)),
        }
        details.push(words(Role::Detail, *separate));
        let refresh = self.control(refresh);
        details.push(self.one(refresh));
        vec![Node::Fold { id: "fold.service-details".into(), title: (*show).into(),
            start_open: false, children: details }]
    }

    pub(super) fn request(&mut self, bar: &RequestBar) -> Node {
        let RequestBar { title, working, words: said, progress, details, report, dismiss } = bar;
        let mut children = vec![Node::Label((*title).into())];
        if *working {
            children.push(Node::Chip { id: "chip.request".into(),
                status: StatusView::new(StatusClass::Working, said.clone()) });
        } else {
            children.push(words(Role::Body, said.clone()));
        }
        if let Some(RequestProgress { counts, problem, title, headline, results }) = progress {
            children.push(Node::Label((*title).into()));
            children.push(words(Role::Strong, *headline));
            if let Some(counts) = counts {
                children.push(words(Role::Detail, counts.clone()));
            }
            if let Some(problem) = problem {
                children.push(words(Role::Caution, problem.clone()));
            }
            children.extend(self.results(results));
        }
        if let Some(details) = details {
            children.push(self.fold(details));
        }
        let mut controls = Vec::new();
        if let Some((note, save)) = report {
            children.push(words(Role::Detail, *note));
            controls.push(self.control(save));
        }
        if let Some(dismiss) = dismiss {
            controls.push(self.control(dismiss));
        }
        if !controls.is_empty() {
            children.push(self.actions("request", controls));
        }
        Node::Panel { kind: PanelKind::Card, children }
    }

}
