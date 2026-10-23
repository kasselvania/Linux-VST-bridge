//! The package owner's Setup panel, before ordinary Library admission.
use super::*;
pub fn build(panel: &lvb_client::bootstrap::Panel, waiting: bool) -> Built {
    let mut built = Built::default();
    built.header.push(Node::Header(vec![Node::Brand("Linux Plugin Bridge".into())]));
    let mut children = vec![Node::Title(panel.title.into()), Node::Text(Role::Lead, panel.introduction.into())];
    if let Some(progress) = panel.progress { children.push(Node::Text(Role::Strong, progress.into())); }
    children.push(Node::Text(Role::Body, panel.advice.into()));
    if !panel.detail.is_empty() { children.push(Node::Text(Role::Hint, panel.detail.into())); }
    if let Some(result) = &panel.result { children.push(Node::Text(Role::Alert, result.clone())); }
    let actions = panel.controls.iter().map(|control| {
        let id = format!("bootstrap.{}::{:?}", slug(control.label), control.step);
        built.targets.insert(id.clone(), Target::Bootstrap(control.step));
        if waiting { ActionView::disabled(id, control.label, "Waiting for Setup.") }
        else { ActionView::offered(id, control.label) }
    }).collect::<Vec<_>>();
    if !actions.is_empty() { children.push(Node::Actions { id: "bootstrap.steps".into(), stacked: false, actions, notes: vec![] }); }
    built.page.push(Node::Panel { kind: if panel.result.is_some() { PanelKind::Recovery } else { PanelKind::Card }, children });
    built
}
