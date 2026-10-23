//! Console support composition. Owner data and offers come from the shared view model.
use super::*;

impl Builder<'_> {
    pub(super) fn diagnostics(&mut self, diagnostics: &Diagnostics, frame: &Frame) -> Vec<Node> {
        let Diagnostics { heading: _, version, system, actions_title, actions, records, technical } = diagnostics;
        let VersionSection { installed, setup, apply, runtime, copy, copied } = version;
        let mut out = vec![Node::Title("Support".into())];
        if let Some(export) = &frame.title.export { out.extend(self.row(export)); }
        let mut children = vec![Node::Label("Version details".into()), words(Role::Body, installed.clone())];
        if let Some(setup) = setup {
            children.push(words(Role::Code, setup.clone()));
        }
        if let Some(apply) = apply {
            children.push(words(Role::Body, *apply));
        }
        children.push(words(Role::Code, runtime.clone()));
        let copy = self.control(copy);
        children.push(self.one(copy));
        if self.ui.copied {
            children.push(words(Role::Detail, *copied));
        }
        let versions = Node::Panel { kind: PanelKind::Card, children };
        let mut rows = vec![Node::Heading((*actions_title).into())];
        for row in actions {
            rows.extend(self.row(row));
        }
        out.push(Node::Columns(vec![versions, Node::Panel { kind: PanelKind::Card, children: rows }]));
        out.push(self.fold(system));
        out.extend(self.service_details(frame));
        out.push(self.fold(records));
        out.push(self.fold(technical));
        out
    }

}
