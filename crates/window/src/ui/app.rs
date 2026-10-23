//! Native Makepad rendering for sample and installed Bridge controllers.
//! Product requests run on the persistent client worker; this thread draws
//! shared view-model records and handles local input.

use {
    super::{blocks::BlocksWidgetRefExt, controller::Controller},
    crate::Outcome,
    makepad_product_ui::{makepad_widgets::*, *},
    std::sync::OnceLock,
};

app_main!(App);

/// Package inspection exits before Makepad creates a display, remote server or window.
pub fn run() {
    if has("--package-info") {
        match option_env!("LVB_FRONTEND_PACKAGE_INFO") {
            Some(info) => println!("{info}"),
            None => {
                eprintln!("This frontend has no declared package source identity");
                std::process::exit(1);
            }
        }
        return;
    }
    main();
}

/// The Bridge's accent.
const BRASS: Accent = Accent {
    dark: Rgba::rgb(0xcf9a5e),
    light: Rgba::rgb(0x8a5d2b),
};

static POSTURE: OnceLock<ProductUi> = OnceLock::new();

fn args() -> Vec<String> {
    std::env::args().collect()
}

fn has(flag: &str) -> bool {
    args().iter().any(|arg| arg == flag)
}

fn posture() -> ProductUi {
    *POSTURE.get_or_init(|| ProductUi {
        density: if has("--density=compact") { Density::Compact } else { Density::Comfortable },
        appearance: if has("--light") { Appearance::Light } else { Appearance::Dark },
        motion: if has("--reduced-motion") { Motion::Reduced } else { Motion::Full },
        accent: BRASS,
    })
}

fn posture_words() -> String {
    let p = posture();
    format!(
        "density {} · {} · motion {} · controls {} px, touch box {} px",
        match p.density { Density::Comfortable => "comfortable", Density::Compact => "compact" },
        match p.appearance { Appearance::Dark => "dark", Appearance::Light => "light" },
        match p.motion { Motion::Full => "full", Motion::Reduced => "reduced" },
        p.metrics().control_height,
        p.metrics().touch_target,
    )
}

script_mod! {
    use mod.prelude.product_ui.*
    use mod.widgets.*

    startup() do #(App::script_component(vm)){
        ui: Root{
            main_window := Window{
                window.inner_size: vec2(1180, 740)
                window.title: "Linux Plugin Bridge"
                pass +: {clear_color: ui.app}
                caption_bar +: {
                    draw_bg.color: ui.app
                    caption_label +: {label +: {draw_text +: {color: ui.muted, text_style: ui.font_small}}}
                }
                body +: {
                    View{
                        width: Fill
                        height: Fill
                        flow: Down
                        View{
                            width: Fill
                            height: Fit
                            padding: Inset{}
                            preview := BridgeBlocks{gap: ui.space_2, row_gap: ui.space_2}
                        }
                        View{
                            width: Fill
                            height: Fit
                            padding: Inset{left: 24, right: 24, top: 6, bottom: 6}
                            header := BridgeBlocks{}
                        }
                        View{width: Fill, height: 1, show_bg: true, draw_bg +: {color: ui.line}}
                        page_scroll := ScrollYView{
                            width: Fill
                            height: Fill
                            padding: Inset{left: 22, right: 22, top: 20, bottom: 24}
                            page := BridgeBlocks{}
                        }
                    }
                    // Last, so an open form is on top and sees input first.
                    form_modal := Modal{
                        content +: {
                            width: 640
                            height: 660
                            form_scroll := ScrollYView{
                            width: Fill
                            height: Fill
                            form_panel := RoundedView{
                                width: Fill
                                height: Fit
                                flow: Down
                                padding: ui.space_6
                                draw_bg +: {
                                    color: ui.panel
                                    border_color: ui.line
                                    border_size: 1.0
                                    border_radius: ui.radius_panel
                                }
                                form := BridgeBlocks{}
                            }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Script, ScriptHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[rust]
    controller: Option<Controller>,
    #[rust]
    commands: CommandMap,
    #[rust]
    form_open: bool,
    /// The last input that asked for a change, until its frame is drawn.
    #[rust]
    pressed: Option<(f64, String)>,
    #[rust]
    feedback_ms: Vec<f64>,
    #[rust]
    frames: u64,
    #[rust]
    report: Timer,
    #[rust]
    modal_size: Vec2d,
    #[rust]
    service_tick: Timer,
}

impl App {
    fn controller(&mut self) -> &mut Controller {
        self.controller.as_mut().expect("the controller starts at startup")
    }

    /// Draws what the controller built and prints what it said.
    fn show(&mut self, cx: &mut Cx) {
        let Some(controller) = self.controller.as_mut() else { return };
        let built = controller.built().clone();
        for line in controller.new_log() {
            log!("[window] {line}");
        }
        self.ui.blocks(cx, ids!(preview)).set_nodes(cx, if has("--presentation-only") { vec![] } else { built.preview });
        self.ui.blocks(cx, ids!(header)).set_nodes(cx, built.header);
        self.ui.blocks(cx, ids!(page)).set_nodes(cx, built.page);
        let modal = self.ui.modal(cx, ids!(form_modal));
        match built.form {
            Some(form) => {
                let focus = built.form_focus.clone().or_else(|| first_control(&form));
                self.ui.blocks(cx, ids!(form)).set_nodes(cx, form);
                if !self.form_open {
                    self.ui.view(cx, ids!(form_scroll)).set_scroll_pos(cx, dvec2(0.0, 0.0));
                    modal.open(cx);
                    self.form_open = true;
                    if let Some(focus) = focus {
                        self.ui.blocks(cx, ids!(form)).focus_control(cx, &focus);
                    }
                }
            }
            None => {
                // Retire the old form's controls before close restores focus;
                // focus changes are queued until this event has finished.
                self.ui.blocks(cx, ids!(form)).set_nodes(cx, vec![]);
                if self.form_open {
                    self.form_open = false;
                    modal.close(cx);
                }
            }
        }
    }

    fn press(&mut self, cx: &mut Cx, id: &str, via: InputPath) {
        self.pressed = Some((Cx::monotonic_now(), format!("{id} via {via:?}")));
        let persistent_navigation = matches!(self.controller().built().targets.get(id),
            Some(crate::presentation::Target::Tab(_) | crate::presentation::Target::Preview(_)));
        let outcome = self.controller().press(id, via);
        self.show(cx);
        if outcome == Outcome::Navigated {
            self.ui.view(cx, ids!(page_scroll)).set_scroll_pos(cx, dvec2(0.0, 0.0));
            // A keyboard user lands on the new page's first control.
            if matches!(via, InputPath::Keyboard) && !persistent_navigation {
                if let Some(first) = self.controller().page_first() {
                    self.ui.blocks(cx, ids!(page)).focus_control(cx, &first);
                }
            }
        }
    }

    fn handle_shortcut(&mut self, cx: &mut Cx, event: &KeyEvent) {
        let active = if self.form_open {
            vec![CommandScope::Transient, CommandScope::Application]
        } else {
            vec![CommandScope::Document, CommandScope::Application]
        };
        let Some(command) = self.commands.resolve(KeyChord::from_event(event), &active).cloned() else { return };
        let target = match command.as_str() {
            "preview.form.dismiss" => {
                self.pressed = Some((Cx::monotonic_now(), "Escape".into()));
                self.controller().dismiss_form("by Escape");
                self.show(cx);
                return;
            }
            "preview.sample.next" => "preview.next-sample",
            "preview.sample.previous" => "preview.previous-sample",
            "preview.page.next" => "preview.next-page",
            "preview.page.previous" => "preview.previous-page",
            "preview.request.next" => "preview.next-request",
            "preview.update.next" => "preview.next-update",
            _ => return,
        };
        self.press(cx, target, InputPath::Shortcut);
    }

    fn bind(&mut self) {
        let bind = |map: &mut CommandMap, scope, chord, name| map.bind(scope, chord, CommandId::new(name));
        if self.controller().is_live() { return; }
        let map = &mut self.commands;
        bind(map, CommandScope::Transient, KeyChord::key(KeyCode::Escape), "preview.form.dismiss");
        bind(map, CommandScope::Application, KeyChord::primary(KeyCode::RBracket), "preview.sample.next");
        bind(map, CommandScope::Application, KeyChord::primary(KeyCode::LBracket), "preview.sample.previous");
        bind(map, CommandScope::Application, KeyChord::primary(KeyCode::RBracket).with_shift(), "preview.page.next");
        bind(map, CommandScope::Application, KeyChord::primary(KeyCode::LBracket).with_shift(), "preview.page.previous");
        bind(map, CommandScope::Application, KeyChord::primary(KeyCode::KeyJ), "preview.request.next");
        bind(map, CommandScope::Application, KeyChord::primary(KeyCode::KeyU), "preview.update.next");
    }
}

fn first_control(nodes: &[crate::presentation::Node]) -> Option<String> {
    use crate::presentation::Node;
    nodes.iter().find_map(|node| match node {
        Node::LibraryRow { action, .. } => Some(action.id.0.clone()),
        Node::Actions { actions, .. } => actions.first().map(|action| action.id.0.clone()),
        Node::Panel { children, .. } | Node::Row(children) | Node::Stack(children) | Node::Columns(children) | Node::Header(children) => first_control(children),
        _ => None,
    })
}

impl MatchEvent for App {
    fn handle_startup(&mut self, cx: &mut Cx) {
        let args = args();
        self.controller = Some(Controller::new(&args, posture_words(), has("--reveal")));
        if self.controller().is_live() {
            self.service_tick = cx.start_interval(0.2);
        }
        self.bind();
        self.show(cx);
        log!("[window] started: {}", posture_words());
        if let Some(size) = args.iter().find_map(|arg| {
            let (w, h) = arg.strip_prefix("--size=")?.split_once('x')?;
            Some(dvec2(w.parse().ok()?, h.parse().ok()?))
        }) {
            self.ui.window(cx, ids!(main_window)).resize(cx, size);
        }
        if has("--frame-report") {
            self.report = cx.start_interval(1.0);
        }
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        for intent in action_intents(actions) {
            match intent {
                ActionIntent::Invoke { id, via } => self.press(cx, &id.0, via),
                ActionIntent::Unavailable { id, via, reason } => {
                    self.pressed = Some((Cx::monotonic_now(), format!("{} via {via:?} (held)", id.0)));
                    let line = format!("Not passed on: {} via {via:?}. {reason}", id.0);
                    self.controller().note(line);
                    self.show(cx);
                }
                ActionIntent::AlreadyPending { id, via } => {
                    let line = format!("Not passed on: {} via {via:?} is already waiting", id.0);
                    self.controller().note(line);
                    self.show(cx);
                }
                ActionIntent::ContextRequested { id, via, .. } => {
                    let line = format!("{} via {via:?} asked for a context menu; the window has none, every action is on the page", id.0);
                    self.controller().note(line);
                    self.show(cx);
                }
                ActionIntent::None => {}
            }
        }
        for blocks in [ids!(page), ids!(form), ids!(header)] {
            let changed = self.ui.blocks(cx, blocks).fields_changed(actions);
            for (id, text) in changed {
                self.pressed = Some((Cx::monotonic_now(), format!("typing in {id}")));
                self.controller().field(&id, &text);
                self.show(cx);
            }
        }
        if self.ui.modal(cx, ids!(form_modal)).dismissed(actions) {
            self.form_open = false;
            self.controller().dismiss_form("by a click outside it");
            self.show(cx);
        }
    }
}

impl AppMain for App {
    fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
        makepad_product_ui::register_embedded_resources(vm.cx_mut());
        let app_crate = module_path!().split("::").next().expect("application crate");
        vm.cx_mut().register_embedded_resource(
            &format!("{app_crate}/resources/bridge-mark.svg"),
            include_bytes!("../../resources/bridge-mark.svg"));
        crate::makepad_widgets::script_mod(vm);
        makepad_product_ui::script_mod(vm, &posture());
        super::blocks::script_mod(vm);
        self::script_mod(vm)
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if matches!(event, Event::Draw(_)) {
            let size = self.ui.window(cx, ids!(main_window)).get_inner_size(cx);
            let bounded = dvec2((size.x - 32.0).clamp(1.0, 640.0), (size.y - 64.0).clamp(1.0, 660.0));
            if bounded != self.modal_size {
                self.modal_size = bounded;
                self.ui.view(cx, ids!(form_modal.content)).set_walk(cx, Walk {
                    width: Size::Fixed(bounded.x), height: Size::Fixed(bounded.y), ..Walk::default()
                });
            }
        }
        if let Event::KeyDown(key) = event {
            self.handle_shortcut(cx, key);
        }
        if self.report.is_event(event).is_some() {
            log!("[window] frames drawn in the last second: {}", self.frames);
            self.frames = 0;
        }
        if self.service_tick.is_event(event).is_some() {
            if let Some(controller) = self.controller.as_mut() {
                if controller.tick() { self.show(cx); }
            }
        }
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
        if let Some(controller) = self.controller.as_mut() {
            let changed = controller.pump();
            if let Some(text) = controller.take_clipboard() { cx.copy_to_clipboard(&text); }
            let close = controller.take_close_request();
            if close { self.ui.window(cx, ids!(main_window)).close(cx); }
            if changed { self.show(cx); }
        }
        if let Event::Draw(_) = event {
            self.frames += 1;
            if let Some((at, what)) = self.pressed.take() {
                let ms = (Cx::monotonic_now() - at) * 1000.0;
                self.feedback_ms.push(ms);
                let worst = self.feedback_ms.iter().copied().fold(0.0, f64::max);
                log!("[window] feedback drawn {ms:.1} ms after {what} (worst so far {worst:.1} ms over {})",
                    self.feedback_ms.len());
            }
        }
    }
}
