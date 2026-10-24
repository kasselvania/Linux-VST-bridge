//! Console updates composition. Owner data and offers come from the shared view model.
use super::*;

impl Builder<'_> {
    pub(super) fn update_panel(&mut self, panel: &updates::Panel, snapshot: Option<&Snapshot>) -> Node {
        let updates::Panel { title, link, message, status: block, check } = panel;
        let mut children = vec![Node::Heading((*title).into())];
        match link {
            updates::Link::Entry { prompt, hint, save, keep, privacy } => {
                children.push(words(Role::Body, *prompt));
                children.push(Node::Field { secret: true, id: "field.beta-link".into(), value: self.ui.link_draft.clone(),
                    hint: (*hint).into() });
                let reason = (!updates::can_save(&self.ui.link_draft)).then(|| (*hint).to_string());
                let mut controls = vec![self.local(save, Importance::Primary,
                    Target::Update(updates::Step::SaveLink), reason)];
                if let Some(keep) = keep {
                    controls.push(self.update_control(keep));
                }
                children.push(self.actions("link", controls));
                children.push(words(Role::Detail, *privacy));
            }
            updates::Link::Saved { line, change } => {
                children.push(words(Role::Body, *line));
                if let Some(change) = change {
                    let view = self.update_control(change);
                    children.push(self.one(view));
                }
            }
        }
        if let Some(message) = message {
            children.push(words(Role::Caution, message.clone()));
        }
        if let Some(block) = block {
            if matches!(block, updates::Block::Available { .. } | updates::Block::Waiting { .. }) {
                children.push(Node::Label("What changes for your setups".into()));
                children.extend(updates::package_impact(snapshot).into_iter().map(|line| words(Role::Body, line)));
            }
            children.extend(self.update_block(block));
        }
        if let Some(check) = check {
            let view = self.update_control(check);
            children.push(self.one(view));
        }
        Node::Panel { kind: PanelKind::Card, children }
    }

    pub(super) fn update_control(&mut self, control: &updates::UpdateControl) -> ActionView {
        let updates::UpdateControl { label, enabled, does } = control;
        let reason = (!enabled).then(|| self.why.control());
        self.local(label, Importance::Secondary, Target::Update(*does), reason)
    }

    pub(super) fn notice(&mut self, notice: &updates::UpdateNotice, tone: NoticeTone) -> Vec<Node> {
        let updates::UpdateNotice { text: said, technical } = notice;
        let id = self.id("notice", said);
        let mut out = vec![Node::Notice { id, notice: NoticeView::new(tone, said.clone()) }];
        if let Some(technical) = technical {
            let fold = self.id("fold", updates::TECHNICAL);
            out.push(Node::Fold { id: fold, title: updates::TECHNICAL.into(), start_open: false,
                children: vec![words(Role::Code, technical.clone())] });
        }
        out
    }

    pub(super) fn update_block(&mut self, block: &updates::Block) -> Vec<Node> {
        match block {
            updates::Block::Idle => vec![],
            updates::Block::Working(said) => vec![Node::Chip { id: "chip.update".into(),
                status: StatusView::new(StatusClass::Working, *said) }],
            updates::Block::Line(line) => vec![words(Role::Body, line.clone())],
            updates::Block::Notice { notice, warn } => {
                self.notice(notice, if *warn { NoticeTone::Refused } else { NoticeTone::Note })
            }
            updates::Block::Available { headline, notes, notice, advice, download } => {
                let mut out = vec![words(Role::Strong, headline.clone())];
                if let Some((title, notes)) = notes {
                    out.push(Node::Label((*title).into()));
                    out.push(words(Role::Body, notes.clone()));
                }
                if let Some(notice) = notice {
                    out.extend(self.notice(notice, NoticeTone::Note));
                }
                out.push(words(Role::Body, *advice));
                let view = self.update_control(download);
                out.push(self.one(view));
                out
            }
            updates::Block::Downloading { line, fraction, progress, note } => vec![
                words(Role::Strong, line.clone()),
                Node::Progress(*fraction),
                words(Role::Code, progress.clone()),
                words(Role::Detail, *note),
            ],
            updates::Block::Waiting { headline, warning, continue_anyway, cancel, recheck } => {
                let mut out = vec![words(Role::Strong, headline.clone()), words(Role::Body, *warning)];
                let mut controls = Vec::new();
                if let Some(continue_anyway) = continue_anyway {
                    controls.push(self.update_control(continue_anyway));
                }
                controls.push(self.update_control(cancel));
                out.push(self.actions("waiting", controls));
                if let Some(recheck) = recheck {
                    out.push(words(Role::Detail, *recheck));
                }
                out
            }
        }
    }

}
