//! Draws Bridge's presentation nodes with the Product UI Foundation: offers and
//! controls are `ActionGroup`s (each button a tab stop with a 44 px touch
//! box and a printed reason when held), status is a `StatusChip`, facts are
//! a `FactSection`, disclosures are `Disclosure`s whose content the window
//! draws beneath them. Text is drawn directly in the Console type roles.

use {
    crate::presentation::{Node, PanelKind, Role},
    makepad_product_ui::{makepad_widgets::*, *},
    std::collections::{HashMap, HashSet},
};

script_mod! {
    use mod.prelude.product_ui.*
    use mod.widgets.*

    mod.widgets.BridgeBlocksBase = #(Blocks::register_widget(vm))
    mod.widgets.BridgeBlocks = set_type_default() do mod.widgets.BridgeBlocksBase{
        width: Fill
        height: Fit
        gap: ui.space_3
        row_gap: ui.space_3
        panel_pad: ui.panel_pad
        indent: ui.space_4
        draw_title +: {color: ui.ink, text_style: ui.font_title}
        draw_heading +: {color: ui.ink, text_style: ui.font_heading}
        draw_label +: {color: ui.muted, text_style: ui.font_label}
        draw_body +: {color: ui.ink, text_style: ui.font_body}
        draw_strong +: {color: ui.ink, text_style: ui.font_body_strong}
        draw_detail +: {color: ui.muted, text_style: ui.font_small}
        draw_caution +: {color: ui.ink, text_style: ui.font_small_strong}
        draw_lead +: {color: ui.muted, text_style: ui.font_body}
        draw_mono +: {color: ui.ink, text_style: ui.font_mono}
        draw_record +: {color: ui.muted, text_style: ui.font_mono}
        draw_panel +: {
            color: ui.panel
            border_color: ui.line
            radius: ui.radius_panel
            highlight: uniform(0.045 * ui.dark)
            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, self.radius)
                sdf.fill_keep(self.color)
                sdf.stroke(self.border_color, 1.0)
                sdf.rect(self.radius, 1.0, self.rect_size.x - 2.0 * self.radius, 1.0)
                sdf.fill(vec4(1.0, 1.0, 1.0, self.highlight))
                return sdf.result
            }
        }
        draw_raised +: {
            color: ui.raised
            border_color: ui.raised
            radius: ui.radius_panel
            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, self.radius)
                sdf.fill(self.color)
                return sdf.result
            }
        }
        draw_preview +: {
            color: ui.app
            border_color: ui.line
            radius: ui.radius_panel
            pixel: fn() {
                // The preview's bar: a dashed outline, so it never reads as product.
                let p = self.pos * self.rect_size
                let sdf = Sdf2d.viewport(p)
                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, self.radius)
                sdf.fill_keep(self.color)
                let dash = step(0.5, fract((p.x + p.y) / 12.0))
                sdf.stroke(vec4(self.border_color.xyz, dash), 1.0)
                return sdf.result
            }
        }
        draw_recovery +: {
            fill: uniform(ui.panel)
            line: uniform(ui.line)
            problem: uniform(ui.problem)
            radius: uniform(ui.radius_panel)
            pixel: fn() {
                // A Problem rule on the left; status colour never fills the panel.
                let p = self.pos * self.rect_size
                let sdf = Sdf2d.viewport(p)
                sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, self.radius)
                sdf.fill_keep(self.fill)
                sdf.stroke(self.line, 1.0)
                sdf.box(0.0, 0.0, self.rect_size.x, self.rect_size.y, self.radius)
                sdf.fill(vec4(self.problem.xyz, 1.0 - step(3.0, p.x)))
                return sdf.result
            }
        }
        draw_rule +: {color: ui.line}
        draw_accent +: {color: ui.accent}
        draw_track +: {color: ui.raised}
        draw_fill +: {color: ui.working}

        Actions := mod.widgets.ActionGroup{}
        Stack := mod.widgets.ActionGroup{stacked: true}
        Tab := mod.widgets.ActionButton{}
        Library := mod.widgets.ActionButton{width: Fill, height: Fit, draw_outline: false, draw_face.radius: 0.0}
        LibraryChip := mod.widgets.StatusChip{wrap_words: true}
        draw_plugin +: {color: ui.muted, svg: icons.plugin}
        draw_brand +: {color: ui.muted, svg: crate_resource("self:resources/bridge-mark.svg")}
        draw_stage +: {glow_ring: 0.0}
        color_ready: ui.ready
        color_working: ui.working
        color_needs_you: ui.needs_you
        color_problem: ui.problem
        color_unknown: ui.unknown
        Chip := mod.widgets.StatusChip{}
        Facts := mod.widgets.FactSection{}
        Notice := mod.widgets.Notice{}
        Fold := mod.widgets.Disclosure{caller_body: true}
        Field := mod.widgets.TextInputFlat{
            width: Fill
            height: ui.control_height
            padding: Inset{left: 12.0, right: 12.0, top: 11.0, bottom: 11.0}
            margin: Inset{}
            empty_text: ""
            draw_bg +: {
                fill: uniform(ui.app)
                line: uniform(ui.line)
                ring: uniform(ui.accent)
                radius: uniform(ui.radius_control)
                pixel: fn() {
                    let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                    sdf.box(0.5, 0.5, self.rect_size.x - 1.0, self.rect_size.y - 1.0, self.radius)
                    sdf.fill_keep(self.fill)
                    sdf.stroke(mix(self.line, self.ring, self.focus), 1.0 + self.focus)
                    return sdf.result
                }
            }
            draw_text +: {
                color: ui.ink
                color_hover: uniform(ui.ink)
                color_focus: uniform(ui.ink)
                color_down: uniform(ui.ink)
                color_disabled: uniform(ui.faint)
                color_empty: uniform(ui.muted)
                color_empty_hover: uniform(ui.muted)
                color_empty_focus: uniform(ui.muted)
                text_style: ui.font_body
            }
            draw_selection +: {
                color: uniform(ui.raised)
                color_hover: uniform(ui.raised)
                color_focus: uniform(ui.raised)
                color_down: uniform(ui.raised)
                color_empty: uniform(ui.raised)
            }
            draw_cursor +: {color: uniform(ui.accent)}
        }
    }
}

#[derive(Clone, Debug, Default)]
pub enum BlocksAction {
    #[default]
    None,
    /// The musician typed into a field.
    Field { id: String, text: String },
}

/// A list of nodes, drawn top to bottom. Child widgets are kept by node id,
/// so focus, open disclosures and cross-fades survive a rebuild.
#[derive(Script, WidgetRef, WidgetSet, WidgetRegister)]
pub struct Blocks {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[walk]
    walk: Walk,
    #[rust]
    area: Area,
    #[rust]
    bootstrap_draw: Option<(u64, &'static str)>,
    #[live]
    gap: f64,
    #[live]
    row_gap: f64,
    #[live]
    panel_pad: f64,
    #[live]
    indent: f64,
    #[live]
    draw_stage: DrawStatusDot,
    #[live]
    color_ready: Vec4f,
    #[live]
    color_working: Vec4f,
    #[live]
    color_needs_you: Vec4f,
    #[live]
    color_problem: Vec4f,
    #[live]
    color_unknown: Vec4f,
    #[live]
    draw_plugin: DrawSvg,
    #[live]
    draw_brand: DrawSvg,
    #[live]
    draw_title: DrawText,
    #[live]
    draw_heading: DrawText,
    #[live]
    draw_label: DrawText,
    #[live]
    draw_body: DrawText,
    #[live]
    draw_strong: DrawText,
    #[live]
    draw_detail: DrawText,
    #[live]
    draw_caution: DrawText,
    #[live]
    draw_lead: DrawText,
    #[live]
    draw_mono: DrawText,
    #[live]
    draw_record: DrawText,
    #[live]
    draw_panel: DrawBox,
    #[live]
    draw_raised: DrawBox,
    #[live]
    draw_preview: DrawBox,
    #[live]
    draw_recovery: DrawQuad,
    #[live]
    draw_rule: DrawColor,
    #[live]
    draw_accent: DrawColor,
    #[live]
    draw_track: DrawColor,
    #[live]
    draw_fill: DrawColor,
    #[rust]
    templates: HashMap<LiveId, ScriptObjectRef>,
    #[rust]
    children: HashMap<LiveId, (LiveId, WidgetRef)>,
    /// Children drawn in the last frame, in order; only they get events.
    #[rust]
    drawn: Vec<LiveId>,
    #[rust]
    nodes: Vec<Node>,
    /// Child widgets in reading order, for the widget tree.
    #[rust]
    order: Vec<LiveId>,
    /// For each control id: its group, its place in the group and its words.
    #[rust]
    controls: HashMap<String, (LiveId, usize, String)>,
    #[rust]
    fields: HashMap<LiveId, String>,
    #[rust]
    folds: HashMap<LiveId, String>,
    /// A control to focus once it has been drawn.
    #[rust]
    focus: Option<String>,
}

impl ScriptHook for Blocks {
    fn on_after_apply(&mut self, vm: &mut ScriptVm, apply: &Apply, _scope: &mut Scope, value: ScriptValue) {
        if apply.is_eval() {
            return;
        }
        if let Some(obj) = value.as_object() {
            vm.vec_with(obj, |vm, vec| {
                for kv in vec {
                    if let (Some(id), Some(template)) = (kv.key.as_id(), kv.value.as_object()) {
                        self.templates.insert(id, vm.bx.heap.new_object_ref(template));
                    }
                }
            });
        }
    }
}

fn live(id: &str) -> LiveId {
    LiveId::from_str_with_lut(id).unwrap_or_else(|_| LiveId::from_str(id))
}

impl WidgetNode for Blocks {
    fn widget_uid(&self) -> WidgetUid {
        self.uid
    }

    fn walk(&mut self, _cx: &mut Cx) -> Walk {
        self.walk
    }

    fn area(&self) -> Area {
        self.area
    }

    fn redraw(&mut self, cx: &mut Cx) {
        self.area.redraw(cx)
    }

    fn children(&self, visit: &mut dyn FnMut(LiveId, WidgetRef)) {
        for id in &self.order {
            if let Some((_, child)) = self.children.get(id) {
                visit(*id, child.clone());
            }
        }
    }
}

impl Widget for Blocks {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        for id in self.drawn.clone() {
            if let Some((_, child)) = self.children.get(&id) {
                child.handle_event(cx, event, scope);
            }
        }
        if let Event::Actions(actions) = event {
            for (id, field) in &self.fields {
                if let Some((_, child)) = self.children.get(id) {
                    if let Some(text) = child.as_text_input().changed(actions) {
                        cx.widget_action(self.uid, BlocksAction::Field { id: field.clone(), text });
                    }
                }
            }
            if disclosure_toggled(actions).is_some() {
                self.area.redraw(cx);
            }
        }
        if let Event::KeyFocus(kf) = event {
            if let Some(name) = self.focused_name(kf.focus) {
                log!("[window] focus {name}");
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.drawn.clear();
        // Focus rings and status glows reach a few pixels past their controls.
        cx.begin_turtle(walk, Layout { flow: Flow::Down, spacing: self.gap, clip_x: false, clip_y: false,
            ..Layout::default() });
        let nodes = std::mem::take(&mut self.nodes);
        self.draw_nodes(cx, scope, &nodes);
        self.nodes = nodes;
        cx.end_turtle_with_area(&mut self.area);
        if let Some((revision, phase)) = self.bootstrap_draw.take() {
            log!("[window] bootstrap page_drawn revision={} phase={}", revision, phase);
        }
        if let Some(id) = self.focus.take() {
            match self.control_area(&id) {
                Some(area) => cx.set_key_focus(area),
                None => log!("[window] could not focus {id}"),
            }
        }
        DrawStep::done()
    }
}

impl Blocks {
    fn child(&mut self, cx: &mut Cx, id: &str, template: LiveId) -> WidgetRef {
        let key = live(id);
        if let Some((kind, child)) = self.children.get(&key) {
            if *kind == template {
                return child.clone();
            }
        }
        let Some(source) = self.templates.get(&template).map(|t| t.as_object()) else {
            log!("BridgeBlocks has no {template} template");
            return WidgetRef::empty();
        };
        let child = cx.with_vm(|vm| WidgetRef::script_from_value(vm, source.into()));
        cx.widget_tree_insert_child(self.uid, key, child.clone());
        self.children.insert(key, (template, child.clone()));
        child
    }

    fn keep(&mut self, kept: &mut HashSet<LiveId>, id: LiveId) {
        if kept.insert(id) {
            self.order.push(id);
        }
    }

    /// Shows `nodes`, creating or updating each child widget.
    pub fn set_nodes(&mut self, cx: &mut Cx, nodes: Vec<Node>) {
        // A whole group may disappear (Clear search, Show all, a completed
        // form). Remember which old child owns focus before rebuilding its
        // lookup tables, and retire that focus with the child.
        let focus = cx.key_focus();
        let focused_child = self.controls.iter().find_map(|(id, (group, _, _))|
            (self.control_area(id) == Some(focus)).then_some(*group))
            .or_else(|| self.fields.keys().find(|id|
                self.children.get(id).is_some_and(|(_, child)| child.area() == focus)).copied())
            .or_else(|| self.folds.keys().find(|id|
                self.children.get(id).is_some_and(|(_, child)| child.as_disclosure().borrow()
                    .is_some_and(|fold| fold.summary_area() == focus))).copied());
        let mut kept = HashSet::new();
        self.order.clear();
        self.controls.clear();
        self.fields.clear();
        self.folds.clear();
        self.apply(cx, &nodes, &mut kept);
        if focused_child.is_some_and(|id| !kept.contains(&id)) {
            cx.set_key_focus(Area::Empty);
        }
        self.children.retain(|id, _| kept.contains(id));
        self.nodes = nodes;
        self.area.redraw(cx);
    }

    fn apply(&mut self, cx: &mut Cx, nodes: &[Node], kept: &mut HashSet<LiveId>) {
        for node in nodes {
            match node {
                Node::Actions { id, stacked, actions, .. } => {
                    let template = if *stacked { live_id!(Stack) } else { live_id!(Actions) };
                    let group = self.child(cx, id, template);
                    group.as_action_group().set_actions(cx, actions);
                    for (index, action) in actions.iter().enumerate() {
                        self.controls.insert(action.id.0.clone(), (live(id), index, action.label.clone()));
                    }
                    self.keep(kept, live(id));
                }
                Node::LibraryRow { action, status, .. } => {
                    self.child(cx, &action.id.0, live_id!(Library)).as_action_button().set_action(cx, action);
                    self.controls.insert(action.id.0.clone(), (live(&action.id.0), usize::MAX, action.label.clone()));
                    self.keep(kept, live(&action.id.0));
                    if let Some(status) = status {
                        let id = format!("{}.status", action.id.0);
                        self.child(cx, &id, live_id!(LibraryChip)).as_status_chip().set_status(cx, status);
                        self.keep(kept, live(&id));
                    }
                }
                Node::Tabs { tabs, .. } => {
                    for tab in tabs {
                        let button = self.child(cx, &tab.id.0, live_id!(Tab));
                        button.as_action_button().set_action(cx, tab);
                        self.controls.insert(tab.id.0.clone(), (live(&tab.id.0), usize::MAX, tab.label.clone()));
                        self.keep(kept, live(&tab.id.0));
                    }
                }
                Node::Chip { id, status } => {
                    self.child(cx, id, live_id!(Chip)).as_status_chip().set_status(cx, status);
                    self.keep(kept, live(id));
                }
                Node::Facts { id, section } => {
                    self.child(cx, id, live_id!(Facts)).as_fact_section().set_section(cx, section);
                    self.keep(kept, live(id));
                }
                Node::Notice { id, notice } => {
                    self.child(cx, id, live_id!(Notice)).as_notice().set_notice(cx, notice);
                    self.keep(kept, live(id));
                }
                Node::Fold { id, title, start_open, children } => {
                    let fresh = !self.children.contains_key(&live(id));
                    let fold = self.child(cx, id, live_id!(Fold));
                    fold.as_disclosure().set_content(cx, title, "");
                    if fresh {
                        fold.as_disclosure().set_open(cx, *start_open);
                    }
                    self.folds.insert(live(id), title.clone());
                    self.keep(kept, live(id));
                    self.apply(cx, children, kept);
                }
                Node::Field { id, value, hint, secret } => {
                    let field = self.child(cx, id, live_id!(Field));
                    if let Some(mut input) = field.as_text_input().borrow_mut() {
                        if input.is_password() != *secret { input.set_is_password(cx, *secret); }
                    }
                    if field.text() != *value {
                        field.set_text(cx, value);
                    }
                    if let Some(mut input) = field.as_text_input().borrow_mut() {
                        if input.empty_text() != hint.as_str() {
                            input.set_empty_text(cx, hint.clone());
                        }
                    }
                    self.fields.insert(live(id), id.clone());
                    self.keep(kept, live(id));
                }
                Node::Panel { children, .. } | Node::Row(children) | Node::Stack(children) | Node::Columns(children) | Node::Header(children) => self.apply(cx, children, kept),
                _ => {}
            }
        }
    }

    /// Focuses the control `id` once it has been drawn.
    pub fn focus_control(&mut self, cx: &mut Cx, id: &str) {
        self.focus = Some(id.to_owned());
        self.area.redraw(cx);
    }

    fn control_area(&self, id: &str) -> Option<Area> {
        let (group, index, _) = self.controls.get(id)?;
        let (_, child) = self.children.get(group)?;
        if *index == usize::MAX {
            return Some(child.area());
        }
        child.as_action_group().control_areas().get(*index).copied()
    }

    /// The control holding key focus, by id and words, for the keyboard log.
    fn focused_name(&self, focus: Area) -> Option<String> {
        if focus.is_empty() {
            return None;
        }
        for (id, (group, index, label)) in &self.controls {
            let Some((_, child)) = self.children.get(group) else { continue };
            let area = if *index == usize::MAX {
                child.area()
            } else {
                match child.as_action_group().control_areas().get(*index) {
                    Some(area) => *area,
                    None => continue,
                }
            };
            if area == focus {
                return Some(format!("{id} \u{201c}{label}\u{201d}"));
            }
        }
        for (id, title) in &self.folds {
            if let Some((_, child)) = self.children.get(id) {
                if child.as_disclosure().borrow().is_some_and(|fold| fold.summary_area() == focus) {
                    return Some(format!("{id} \u{201c}{title}\u{201d} (disclosure)"));
                }
            }
        }
        for (id, field) in &self.fields {
            if let Some((_, child)) = self.children.get(id) {
                if child.area() == focus {
                    return Some(format!("{field} (text field)"));
                }
            }
        }
        None
    }

    fn panel_draw(&mut self, kind: PanelKind) -> &mut DrawQuad {
        match kind {
            PanelKind::Card => &mut self.draw_panel,
            PanelKind::Raised => &mut self.draw_raised,
            PanelKind::Preview => &mut self.draw_preview,
            PanelKind::Recovery => &mut self.draw_recovery,
        }
    }

    /// Opens a panel background. Panels of one kind nest (a group inside a
    /// held-open section), and one draw shader keeps only its latest
    /// instance, so each panel keeps its own and gives it back to `end_box`.
    fn begin_box(&mut self, cx: &mut Cx2d, kind: PanelKind, layout: Layout) -> Area {
        let draw = self.panel_draw(kind);
        draw.begin(cx, Walk::fill_fit(), layout);
        draw.draw_vars.area
    }

    fn end_box(&mut self, cx: &mut Cx2d, kind: PanelKind, area: Area) {
        let draw = self.panel_draw(kind);
        draw.draw_vars.area = area;
        draw.end(cx);
    }

    fn text_style(&mut self, role: Role) -> &mut DrawText {
        match role {
            Role::Body => &mut self.draw_body,
            Role::Strong => &mut self.draw_strong,
            Role::Detail | Role::Hint => &mut self.draw_detail,
            Role::Caution | Role::Alert | Role::CautionHint => &mut self.draw_caution,
            Role::Record => &mut self.draw_record,
            Role::Code => &mut self.draw_mono,
            Role::Lead => &mut self.draw_lead,
        }
    }

    fn wrapped(cx: &mut Cx2d, draw: &mut DrawText, text: &str, inline: bool) {
        if inline {
            draw.draw_walk(cx, Walk::fit(), Align::default(), text);
        } else {
            cx.begin_turtle(Walk::fill_fit(), Layout::flow_right_wrap());
            draw.draw_walk(cx, Walk::fit(), Align::default(), text);
            cx.end_turtle();
        }
    }

    fn draw_child(&mut self, cx: &mut Cx2d, scope: &mut Scope, id: &str) {
        let key = live(id);
        if let Some((_, child)) = self.children.get(&key).cloned() {
            child.draw_all(cx, scope);
            self.drawn.push(key);
        }
    }

    /// Consecutive collection rows share one panel and separators. Their own
    /// action faces supply hover, focus, and the complete content-sized target.
    fn draw_nodes(&mut self, cx: &mut Cx2d, scope: &mut Scope, nodes: &[Node]) {
        let mut at = 0;
        while at < nodes.len() {
            if matches!(nodes[at], Node::LibraryRow { .. }) {
                let area = self.begin_box(cx, PanelKind::Card, Layout {
                    flow: Flow::Down, padding: Inset { left: 1.0, right: 1.0, top: 1.0, bottom: 1.0 },
                    clip_x: false, clip_y: false, ..Layout::default()
                });
                let first = at;
                while at < nodes.len() && matches!(nodes[at], Node::LibraryRow { .. }) {
                    if at > first {
                        self.draw_rule.draw_walk(cx, Walk { width: Size::fill(), height: Size::Fixed(1.0), ..Walk::fit() });
                    }
                    self.draw_node(cx, scope, &nodes[at], false);
                    at += 1;
                }
                self.end_box(cx, PanelKind::Card, area);
            } else {
                self.draw_node(cx, scope, &nodes[at], false);
                at += 1;
            }
        }
    }

    fn draw_library_row(&mut self, cx: &mut Cx2d, scope: &mut Scope, action: &ActionView,
        detail: &str, has_status: bool) {
        let key = live(&action.id.0);
        let Some((_, child)) = self.children.get(&key).cloned() else { return };
        let button = child.as_action_button();
        let Some(mut button) = button.borrow_mut() else { return };
        let status_id = format!("{}.status", action.id.0);
        let status_width = if has_status {
            self.children.get(&live(&status_id)).and_then(|(_, chip)|
                chip.as_status_chip().borrow().map(|chip| chip.content_width(cx))).unwrap_or_default()
        } else { 0.0 };
        button.draw_with_content(cx, Walk::fill_fit(), Layout {
            flow: Flow::right(), spacing: 12.0, align: Align { x: 0.0, y: 0.5 },
            padding: Inset { left: 14.0, right: 14.0, top: 12.0, bottom: 12.0 },
            clip_x: false, clip_y: false, ..Layout::default()
        }, |cx, action, label_colour| {
            let label_before = self.draw_strong.color;
            let detail_before = self.draw_detail.color;
            let icon_before = self.draw_plugin.color;
            self.draw_strong.color = label_colour;
            if action.reason().is_some() {
                self.draw_detail.color.w *= 0.4;
                self.draw_plugin.color.w *= 0.4;
            }
            self.draw_plugin.draw_walk(cx, Walk::fixed(24.0, 24.0));
            let chevron = action.invoke(InputPath::Pointer).is_invoke();
            let width = (cx.turtle().unused_inner_width() - 12.0 - if chevron { 22.0 } else { 0.0 }).max(1.0);
            let wide = width >= 680.0 && status_width <= width * 0.45;
            cx.begin_turtle(Walk { width: Size::Fixed(width), height: Size::fit(), ..Walk::fit() },
                Layout { flow: Flow::Down, spacing: 8.0, ..Layout::default() });
            cx.begin_turtle(Walk::fill_fit(), Layout {
                flow: if wide { Flow::right() } else { Flow::Down },
                spacing: 10.0, align: Align { x: 0.0, y: 0.5 }, ..Layout::default()
            });
            cx.begin_turtle(Walk { width: if wide && has_status { Size::Fixed((width - status_width - 10.0).max(1.0)) }
                else { Size::fill() }, height: Size::fit(), ..Walk::fit() },
                Layout { flow: Flow::Down, spacing: 4.0, ..Layout::default() });
            Self::wrapped(cx, &mut self.draw_strong, action.shown_words(), false);
            if !detail.is_empty() { Self::wrapped(cx, &mut self.draw_detail, detail, false); }
            cx.end_turtle();
            self.draw_strong.color = label_before;
            self.draw_detail.color = detail_before;
            self.draw_plugin.color = icon_before;
            if has_status {
                if let Some((_, chip)) = self.children.get(&live(&status_id)).cloned() {
                    chip.draw_walk_all(cx, scope, Walk { width: Size::Fixed(status_width.min(width)),
                        height: Size::fit(), ..Walk::fit() });
                    self.drawn.push(live(&status_id));
                }
            }
            cx.end_turtle();
            for note in action_notes(action) {
                Self::wrapped(cx, &mut self.draw_detail, note, false);
            }
            cx.end_turtle();
            if chevron { self.draw_detail.draw_walk(cx, Walk::fit(), Align::default(), "›"); }
        });
        self.drawn.push(key);
    }

    fn draw_node(&mut self, cx: &mut Cx2d, scope: &mut Scope, node: &Node, inline: bool) {
        match node {
            Node::PluginIdentity { name, detail } => {
                cx.begin_turtle(Walk::fill_fit(), Layout { flow: Flow::right(), spacing: 14.0,
                    align: Align { x: 0.0, y: 0.5 }, ..Layout::default() });
                self.draw_panel.begin(cx, Walk::fixed(52.0, 52.0), Layout {
                    align: Align { x: 0.5, y: 0.5 }, ..Layout::default() });
                self.draw_plugin.draw_walk(cx, Walk::fixed(28.0, 28.0));
                self.draw_panel.end(cx);
                cx.begin_turtle(Walk::fill_fit(), Layout { flow: Flow::Down, spacing: 4.0,
                    ..Layout::default() });
                Self::wrapped(cx, &mut self.draw_label, "PLUG-IN", false);
                Self::wrapped(cx, &mut self.draw_title, name, false);
                Self::wrapped(cx, &mut self.draw_detail, detail, false);
                cx.end_turtle();
                cx.end_turtle();
            }
            Node::Brand(words) => {
                cx.begin_turtle(Walk::fit(), Layout { flow: Flow::right(), spacing: 10.0,
                    align: Align { x: 0.0, y: 0.5 }, ..Layout::default() });
                self.draw_brand.draw_walk(cx, Walk::fixed(24.0, 24.0));
                Self::wrapped(cx, &mut self.draw_label, &words.to_uppercase(), true);
                cx.end_turtle();
            }
            Node::Readiness { status, detail } => {
                cx.begin_turtle(Walk::fill_fit(), Layout { flow: Flow::right(), spacing: 12.0,
                    padding: Inset { top: 6.0, bottom: 6.0, ..Inset::default() }, ..Layout::default() });
                let colour = match status.class {
                    StatusClass::Ready => self.color_ready,
                    StatusClass::Working => self.color_working,
                    StatusClass::NeedsYou => self.color_needs_you,
                    StatusClass::Problem => self.color_problem,
                    StatusClass::Unknown => self.color_unknown,
                };
                self.draw_stage.set_class(colour, status.class);
                cx.begin_turtle(Walk::fixed(12.0, 20.0), Layout { align: Align { x: 0.5, y: 0.5 }, ..Layout::default() });
                self.draw_stage.draw_dot(cx, 7.0);
                cx.end_turtle();
                cx.begin_turtle(Walk::fill_fit(), Layout { flow: Flow::Down, spacing: 4.0, ..Layout::default() });
                Self::wrapped(cx, &mut self.draw_strong, &status.words, false);
                Self::wrapped(cx, &mut self.draw_detail, detail, false);
                cx.end_turtle();
                cx.end_turtle();
            }
            Node::Title(words) => Self::wrapped(cx, &mut self.draw_title, words, inline),
            Node::Heading(words) => Self::wrapped(cx, &mut self.draw_heading, words, inline),
            Node::Label(words) => {
                let upper = words.to_uppercase();
                Self::wrapped(cx, &mut self.draw_label, &upper, inline)
            }
            Node::Text(role, words) => {
                if *role == Role::Alert {
                    // A retained hazard: on the raised neutral, in weight, not colour.
                    let area = self.begin_box(cx, PanelKind::Raised, Layout {
                        padding: Inset { left: 12.0, right: 12.0, top: 10.0, bottom: 10.0 },
                        ..Layout::flow_right_wrap()
                    });
                    self.draw_caution.draw_walk(cx, Walk::fit(), Align::default(), words);
                    self.end_box(cx, PanelKind::Raised, area);
                } else {
                    Self::wrapped(cx, self.text_style(*role), words, inline);
                }
            }
            Node::Chip { id, .. } | Node::Facts { id, .. } | Node::Notice { id, .. } | Node::Field { id, .. }
            | Node::Actions { id, .. } => {
                self.draw_child(cx, scope, id);
                if let Node::Actions { notes, .. } = node {
                    for note in notes {
                        Self::wrapped(cx, &mut self.draw_detail, note, false);
                    }
                }
            }
            Node::Tabs { tabs, selected, .. } => {
                cx.begin_turtle(Walk::fit(), Layout { flow: Flow::right(), spacing: 2.0, clip_x: false,
                    clip_y: false, ..Layout::default() });
                for (index, tab) in tabs.iter().enumerate() {
                    self.draw_child(cx, scope, &tab.id.0);
                    if index == *selected {
                        if let Some((_, child)) = self.children.get(&live(&tab.id.0)) {
                            let rect = child.area().rect(cx);
                            self.draw_accent.draw_abs(cx, Rect {
                                pos: dvec2(rect.pos.x + 6.0, rect.pos.y + rect.size.y - 2.0),
                                size: dvec2((rect.size.x - 12.0).max(0.0), 2.0),
                            });
                        }
                    }
                }
                cx.end_turtle();
            }
            Node::Panel { kind, children } => {
                let layout = Layout {
                    flow: Flow::Down,
                    spacing: self.row_gap,
                    padding: match kind {
                        PanelKind::Recovery => Inset { left: self.panel_pad + 3.0, right: self.panel_pad,
                            top: self.panel_pad, bottom: self.panel_pad },
                        PanelKind::Raised => Inset { left: 12.0, right: 12.0, top: 10.0, bottom: 10.0 },
                        _ => Inset { left: self.panel_pad, right: self.panel_pad, top: self.panel_pad,
                            bottom: self.panel_pad },
                    },
                    clip_x: false,
                    clip_y: false,
                    ..Layout::default()
                };
                let area = self.begin_box(cx, *kind, layout);
                self.draw_nodes(cx, scope, children);
                self.end_box(cx, *kind, area);
            }
            Node::Fold { id, children, .. } => {
                self.draw_child(cx, scope, id);
                let open = self.children.get(&live(id)).is_some_and(|(_, fold)| fold.as_disclosure().is_open());
                if open && !children.is_empty() {
                    cx.begin_turtle(Walk::fill_fit(), Layout {
                        flow: Flow::Down,
                        spacing: self.row_gap,
                        padding: Inset { left: self.indent, ..Inset::default() },
                        clip_x: false,
                        clip_y: false,
                        ..Layout::default()
                    });
                    self.draw_nodes(cx, scope, children);
                    cx.end_turtle();
                }
            }
            Node::Row(children) => {
                // Short items share a line when they fit; narrow windows
                // wrap the complete items instead of clipping navigation.
                cx.begin_turtle(Walk::fill_fit(), Layout {
                    flow: Flow::right_wrap(),
                    spacing: self.gap,
                    align: Align { x: 0.0, y: 0.5 },
                    clip_x: false,
                    clip_y: false,
                    ..Layout::default()
                });
                for child in children {
                    self.draw_node(cx, scope, child, true);
                }
                cx.end_turtle();
            }
            Node::Stack(children) => {
                cx.begin_turtle(Walk::fill_fit(), Layout { flow: Flow::Down, spacing: self.row_gap,
                    ..Layout::default() });
                self.draw_nodes(cx, scope, children);
                cx.end_turtle();
            }
            Node::Columns(children) => {
                let width = cx.turtle().size().x;
                let wide = width >= 760.0 && children.len() >= 2;
                cx.begin_turtle(Walk::fill_fit(), Layout {
                    flow: if wide { Flow::right() } else { Flow::Down }, spacing: 14.0,
                    ..Layout::default() });
                for (i, child) in children.iter().enumerate() {
                    let width = if wide { Size::Fixed((width - 14.0 * (children.len() - 1) as f64) *
                        if children.len() == 2 { if i == 0 { 0.6 } else { 0.4 } } else { 1.0 / children.len() as f64 }) }
                        else { Size::fill() };
                    cx.begin_turtle(Walk { width, height: Size::fit(), ..Walk::fit() }, Layout::flow_down());
                    self.draw_node(cx, scope, child, false);
                    cx.end_turtle();
                }
                cx.end_turtle();
            }
            Node::Header(children) => {
                let width = cx.turtle().size().x;
                let narrow = width < 700.0;
                cx.begin_turtle(Walk::fill_fit(), Layout { flow: if narrow { Flow::Down } else { Flow::right() },
                    spacing: 18.0, align: Align { x: 0.0, y: 0.5 }, ..Layout::default() });
                for (i, child) in children.iter().enumerate() {
                    // The final status owns the right edge; preceding identity
                    // and navigation stay together without a giant blank band.
                    if i + 1 == children.len() && !narrow {
                        let remaining = (cx.turtle().unused_width() - 18.0).max(0.0);
                        cx.begin_turtle(Walk { width: Size::Fixed(remaining), height: Size::fit(), ..Walk::fit() }, Layout { flow: Flow::right(),
                            align: Align { x: 1.0, y: 0.5 }, ..Layout::default() });
                        self.draw_node(cx, scope, child, true);
                        cx.end_turtle();
                    } else if children.len() == 2 && !narrow {
                        cx.begin_turtle(Walk { width: Size::Fixed((width - 300.0).max(200.0)),
                            height: Size::fit(), ..Walk::fit() }, Layout::flow_down());
                        self.draw_node(cx, scope, child, false);
                        cx.end_turtle();
                    } else { self.draw_node(cx, scope, child, true); }
                }
                cx.end_turtle();
            }
            Node::LibraryRow { action, detail, status } => {
                self.draw_library_row(cx, scope, action, detail, status.is_some());
            }
            Node::Progress(fraction) => {
                let rect = cx.walk_turtle(Walk { width: Size::fill(), height: Size::Fixed(8.0), ..Walk::fit() });
                self.draw_track.draw_abs(cx, rect);
                let filled = Rect { pos: rect.pos, size: dvec2(rect.size.x * (*fraction as f64).clamp(0.0, 1.0), 8.0) };
                self.draw_fill.draw_abs(cx, filled);
            }
            Node::Rule => {
                self.draw_rule.draw_walk(cx, Walk { width: Size::fill(), height: Size::Fixed(1.0), ..Walk::fit() });
            }
        }
    }
}

impl BlocksRef {
    /// One marker after these page nodes actually draw, not merely after an
    /// app-level Draw event that might skip a cached page.
    pub fn trace_bootstrap_draw(&self, marker: Option<(u64, &'static str)>) {
        if let Some(mut inner) = self.borrow_mut() { inner.bootstrap_draw = marker; }
    }

    pub fn set_nodes(&self, cx: &mut Cx, nodes: Vec<Node>) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_nodes(cx, nodes);
        }
    }

    pub fn focus_control(&self, cx: &mut Cx, id: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.focus_control(cx, id);
        }
    }

    /// The fields typed into during these actions.
    pub fn fields_changed(&self, actions: &Actions) -> Vec<(String, String)> {
        let uid = self.widget_uid();
        actions
            .iter()
            .filter_map(|action| action.downcast_ref::<WidgetAction>())
            .filter(|action| action.widget_uid == uid)
            .filter_map(|action| match action.action.downcast_ref::<BlocksAction>() {
                Some(BlocksAction::Field { id, text }) => Some((id.clone(), text.clone())),
                _ => None,
            })
            .collect()
    }
}
