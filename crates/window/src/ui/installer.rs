//! First-run intake uses the same native controls and embedded resources as the
//! manager. The client worker owns verification, files and process handover.
use super::blocks::BlocksWidgetRefExt;
use crate::presentation::{Node, PanelKind, Role};
use lvb_client::staging::{FailedAt, Phase, Step, View as StagingView, Worker};
use makepad_product_ui::{makepad_widgets::*, *};

app_main!(InstallerApp);

pub fn run() {
    main();
}

fn posture() -> ProductUi {
    ProductUi {
        density: Density::Comfortable,
        appearance: Appearance::Dark,
        motion: Motion::Full,
        accent: Accent {
            dark: Rgba::rgb(0xcf9a5e),
            light: Rgba::rgb(0x8a5d2b),
        },
    }
}

fn installation_advice(reason: &str) -> &'static str {
    match reason {
        "Not enough disk space. Existing application and projects were retained" =>
            "Free space on the drive that contains your home folder, then reopen this installer. Your selected bridge and projects were retained.",
        "user_package_intake_busy" =>
            "Let any other installer finish, then reopen this one. If this repeats, send the detail below to Bridge support.",
        "user_package_bundle_changed" | "user_package_payload_changed" | "user_package_entry_changed" =>
            "Get a fresh installer from your trusted download source and open it again. Your selected bridge and projects were retained.",
        _ =>
            "Your selected bridge and projects were retained. Send the detail below to Bridge support for help completing installation.",
    }
}

script_mod! {
    use mod.prelude.product_ui.*
    use mod.widgets.*

    startup() do #(InstallerApp::script_component(vm)){
        ui: Root{
            main_window := Window{
                window.inner_size: vec2(640, 500)
                window.title: "Install Linux Plugin Bridge"
                pass +: {clear_color: ui.app}
                caption_bar +: {
                    draw_bg.color: ui.app
                    caption_label +: {label +: {draw_text +: {color: ui.muted, text_style: ui.font_small}}}
                    // The app owns close while atomic staging settles. The
                    // default DesktopButton closes immediately inside Window.
                    windows_buttons +: {close := ActionButton{}}
                }
                body +: {
                    ScrollYView{
                        width: Fill
                        height: Fill
                        padding: Inset{left: 24, right: 24, top: 20, bottom: 24}
                        page := BridgeBlocks{}
                    }
                }
            }
        }
    }
}

#[derive(Script, ScriptHook)]
pub struct InstallerApp {
    #[live]
    ui: WidgetRef,
    #[rust]
    worker: Option<Worker>,
    #[rust]
    tick: Timer,
}

fn nodes(view: &StagingView) -> Vec<Node> {
    let mut body = vec![Node::Title("Install Linux Plugin Bridge".into())];
    if let Some(package) = &view.package {
        body.push(Node::Text(
            Role::Detail,
            format!("Version {}", package.version),
        ));
        if package.internal {
            body.push(Node::Text(
                Role::Caution,
                "Internal test build · not a customer release".into(),
            ));
        }
    }
    let (status, advice) = match &view.phase {
        Phase::Checking => ("Checking the installer…", "Verifying this package’s signature before offering installation."),
        Phase::Ready => ("Ready to install", "Install the application in your account. Setup will help you select this version and acquire the compatibility runtime."),
        Phase::Installing => ("Verifying and installing application files…", "This can take a moment. Your current bridge keeps its selected version."),
        Phase::AddingShortcut => ("Adding Setup to your applications…", "The application files are staged. Making their Setup entry available."),
        Phase::Staged => ("Application files installed", "Open Setup to select this version and start the bridge. You can also open Linux VST Bridge Setup and Updates from Applications."),
        Phase::Opening => ("Opening Setup…", "Checking the staged application before opening it."),
        Phase::Opened => ("Setup opened", "Continue in Setup."),
        Phase::Failed { at: FailedAt::Verification, .. } => ("The installer could not be verified", "Get a fresh installer from your trusted download source and open it again. No application files were installed."),
        Phase::Failed { at: FailedAt::Installation, reason } => ("Installation stopped", installation_advice(reason)),
        Phase::Failed { at: FailedAt::Opening, .. } => ("Setup could not open", "Reopen this installer to verify its application files, then try Open Setup again."),
        Phase::Failed { at: FailedAt::Worker, .. } => ("The installer stopped", "Close this window and reopen the installer to check its files and continue."),
    };
    body.push(Node::Heading(status.into()));
    body.push(Node::Text(Role::Body, advice.into()));
    if let Phase::Failed { reason, .. } = &view.phase {
        body.push(Node::Text(Role::Detail, reason.clone()));
    }
    if view.closing && !view.should_close() {
        body.push(Node::Text(
            Role::Strong,
            "Finishing the current step before closing…".into(),
        ));
    }
    let mut actions = vec![];
    for (step, id, label) in [
        (
            Step::Install,
            "installer.install",
            "Install application files",
        ),
        (Step::OpenSetup, "installer.open", "Open Setup"),
    ] {
        if view.offers(step) {
            actions.push(ActionView::offered(id, label).importance(Importance::Primary));
        }
    }
    actions.push(if view.closing {
        ActionView::disabled("installer.close", "Closing…", "Finishing the current step.")
    } else {
        ActionView::offered("installer.close", "Close")
    });
    body.push(Node::Actions {
        id: "installer.steps".into(),
        stacked: false,
        actions,
        notes: vec![],
    });
    vec![
        Node::Brand("Linux Plugin Bridge".into()),
        Node::Panel {
            kind: if matches!(view.phase, Phase::Failed { .. }) {
                PanelKind::Recovery
            } else {
                PanelKind::Card
            },
            children: body,
        },
    ]
}

impl InstallerApp {
    fn show(&self, cx: &mut Cx) {
        if let Some(worker) = &self.worker {
            self.ui
                .blocks(cx, ids!(page))
                .set_nodes(cx, nodes(worker.view()));
        }
    }

    fn close(&mut self, cx: &mut Cx) {
        if let Some(worker) = &mut self.worker {
            worker.request_close();
            if worker.view().should_close() {
                self.ui.window(cx, ids!(main_window)).close(cx);
            } else {
                self.show(cx);
            }
        }
    }
}

impl MatchEvent for InstallerApp {
    fn handle_startup(&mut self, cx: &mut Cx) {
        self.worker = Some(Worker::start(SignalToUI::set_ui_signal));
        self.tick = cx.start_interval(0.2);
        self.ui
            .action_button(cx, ids!(main_window.caption_bar.windows_buttons.close))
            .set_action(cx, &ActionView::offered("installer.close", "Close"));
        self.show(cx);
    }

    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        for intent in action_intents(actions) {
            let ActionIntent::Invoke { id, .. } = intent else {
                continue;
            };
            if id.0 == "installer.close" {
                self.close(cx);
                continue;
            }
            let step = match id.0.as_str() {
                "installer.install" => Step::Install,
                "installer.open" => Step::OpenSetup,
                _ => continue,
            };
            if self
                .worker
                .as_mut()
                .is_some_and(|worker| worker.press(step))
            {
                self.show(cx);
            }
        }
    }
}

impl AppMain for InstallerApp {
    fn script_mod(vm: &mut ScriptVm) -> ScriptValue {
        makepad_product_ui::register_embedded_resources(vm.cx_mut());
        let app_crate = module_path!()
            .split("::")
            .next()
            .expect("application crate");
        vm.cx_mut().register_embedded_resource(
            &format!("{app_crate}/resources/bridge-mark.svg"),
            include_bytes!("../../resources/bridge-mark.svg"),
        );
        makepad_widgets::script_mod(vm);
        makepad_product_ui::script_mod(vm, &posture());
        super::blocks::script_mod(vm);
        self::script_mod(vm)
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        if let Event::WindowCloseRequested(request) = event {
            if Some(request.window_id) == self.ui.window(cx, ids!(main_window)).window_id() {
                if let Some(worker) = &mut self.worker {
                    worker.request_close();
                    request.accept_close.set(worker.view().should_close());
                    self.show(cx);
                }
            }
        }
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());
        // A reply consumed during Draw would replace the nodes after drawing,
        // where Makepad ignores the ordinary redraw request. Pump on signals,
        // timer and input only, as in the manager's native shell.
        if !matches!(event, Event::Draw(_)) {
            if let Some(worker) = &mut self.worker {
                let changed = worker.poll();
                if worker.view().should_close() {
                    self.ui.window(cx, ids!(main_window)).close(cx);
                } else if changed {
                    self.show(cx);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actions(view: &StagingView) -> Vec<ActionView> {
        nodes(view)
            .into_iter()
            .flat_map(|node| match node {
                Node::Panel { children, .. } => children,
                _ => vec![],
            })
            .flat_map(|node| match node {
                Node::Actions { actions, .. } => actions,
                _ => vec![],
            })
            .collect()
    }

    #[test]
    fn installer_draws_only_the_step_offered_by_its_worker() {
        let mut view = StagingView::default();
        assert_eq!(
            actions(&view)
                .iter()
                .map(|a| a.id.0.as_str())
                .collect::<Vec<_>>(),
            ["installer.close"]
        );
        view.phase = Phase::Ready;
        assert_eq!(actions(&view)[0].id.0, "installer.install");
        view.phase = Phase::Installing;
        assert_eq!(actions(&view).len(), 1);
        view.phase = Phase::Staged;
        assert_eq!(actions(&view)[0].id.0, "installer.open");
        view.closing = true;
        assert_eq!(actions(&view).len(), 1);
        assert!(actions(&view)[0].reason().is_some());
    }

    #[test]
    fn verification_failure_is_actionable_and_has_no_unsigned_version_or_install_offer() {
        let view = StagingView {
            phase: Phase::Failed {
                at: FailedAt::Verification,
                reason: "signature refused".into(),
            },
            ..StagingView::default()
        };
        let rendered = format!("{:?}", nodes(&view));
        assert!(rendered.contains("trusted download source"));
        assert!(rendered.contains("signature refused"));
        assert!(!rendered.contains("Version "));
        assert!(!rendered.contains("installer.install"));
    }

    #[test]
    fn installation_failure_explains_the_next_step_without_bypassing_the_refusal() {
        for (reason, advice) in [
            (
                "Not enough disk space. Existing application and projects were retained",
                "Free space",
            ),
            ("user_package_intake_busy", "Let any other installer finish"),
            ("user_package_payload_changed", "fresh installer"),
            (
                "Existing Setup entry is not owned by this installer; it was retained",
                "Bridge support",
            ),
        ] {
            let view = StagingView {
                phase: Phase::Failed {
                    at: FailedAt::Installation,
                    reason: reason.into(),
                },
                ..StagingView::default()
            };
            let rendered = format!("{:?}", nodes(&view));
            assert!(rendered.contains(advice));
            assert!(rendered.contains(reason));
            assert_eq!(actions(&view).len(), 1);
            assert_eq!(actions(&view)[0].id.0, "installer.close");
        }
    }
}
