//! Console activity composition. Owner data and offers come from the shared view model.
use super::*;

impl Builder<'_> {
    pub(super) fn activity(&mut self, activity: &Activity) -> Vec<Node> {
        let Activity { heading, intro, notice, retry, live, recent, attention, incidents } = activity;
        let mut out = vec![Node::Title((*heading).into()), words(Role::Lead, *intro)];
        if let Some(notice) = notice { out.push(text(notice)); }
        if let Some(retry) = retry { let retry = self.control(retry); out.push(self.one(retry)); }
        out.push(self.attentions(attention));
        out.push(self.sessions(live));
        out.push(self.sessions(recent));
        out.push(self.incidents(incidents));
        out
    }

    pub(super) fn sessions(&mut self, sessions: &Sessions) -> Node {
        let Sessions { title, warning, empty, rows } = sessions;
        let mut children = vec![Node::Heading((*title).into())];
        if let Some(warning) = warning {
            children.push(words(Role::Caution, *warning));
        }
        if let Some(empty) = empty {
            children.push(words(Role::Body, *empty));
        }
        for SessionRow { recent: _, index: _, session, name, state: said, cleanup, open, recovery, record } in rows {
            children.push(words(Role::Strong, name.clone()));
            children.push(words(Role::Body, said.clone()));
            if let Some(cleanup) = cleanup {
                children.push(words(Role::Detail, *cleanup));
            }
            if let Some(session) = session {
                children.push(words(Role::Code, session.clone()));
            }
            if let Some(open) = open {
                let view = self.control(open);
                children.push(self.one(view));
            }
            if let Some(recovery) = recovery { children.push(self.instance_recovery(recovery)); }
            children.push(self.fold(record));
        }
        Node::Panel { kind: PanelKind::Card, children }
    }

    pub(super) fn instance_recovery(&mut self, recovery: &vm::recovery::Recovery) -> Node {
        let mut children = vec![Node::Label("Recover this instance".into())];
        children.extend(recovery.detail.iter().map(text));
        if recovery.actions.offers.is_empty() {
            children.extend(recovery.actions.notes.iter().map(|note| words(Role::Detail, note.clone())));
        } else { children.extend(self.row(&recovery.actions)); }
        children.push(self.fold(&recovery.identity));
        Node::Stack(children)
    }

    pub(super) fn attentions(&mut self, attention: &Attentions) -> Node {
        let Attentions { title, empty, items, more } = attention;
        let mut children = vec![Node::Heading((*title).into())];
        if let Some(empty) = empty {
            children.push(words(Role::Body, *empty));
        }
        for AttentionItem { title, detail, open } in items {
            children.push(words(Role::Strong, title.clone()));
            children.push(words(Role::Body, detail.clone()));
            let view = self.control(open);
            children.push(self.one(view));
        }
        if let Some(more) = more {
            let view = self.control(more);
            children.push(self.one(view));
        }
        Node::Panel { kind: PanelKind::Card, children }
    }

    pub(super) fn incidents(&mut self, incidents: &Incidents) -> Node {
        let Incidents { title, empty, incidents } = incidents;
        let mut children = vec![Node::Heading((*title).into())];
        if let Some(empty) = empty {
            children.push(words(Role::Body, *empty));
        }
        for (_, incident) in incidents {
            children.push(self.fold(incident));
        }
        Node::Panel { kind: PanelKind::Card, children }
    }

}
