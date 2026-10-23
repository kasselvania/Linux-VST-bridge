//! Headless render digests of the whole window, for proving that a change to
//! how screens are built leaves what they draw unchanged.
//!
//! Draws every page in every sample state, in both themes at two widths, and
//! again after one click on each short label drawn there (opening a
//! disclosure, a form or a choice), and after a second click on each label
//! that click revealed (a confirmation, a checkbox, a dropdown's choices).
//! Setup's update panel is drawn in each of its states the same way. Each
//! case's digest covers every frame's meshes (clip rectangles, vertices,
//! colours, indices), texture updates, and what the window chose to send or
//! do, so equal digests mean equal pixels and equal intents. Run it on two
//! trees and compare the lists:
//!
//! `LVB_RENDER_DIGESTS=/tmp/digests.tsv cargo test --release --locked render_digests -- --ignored`
//!
//! The list stays outside Git. Nothing here contacts the manager.
use crate::operator::{Operator, Page};
use crate::{preview_states, presentation, updates::Updates};
use eframe::egui;
use std::hash::{Hash, Hasher};

/// Tall enough that no page is cut off below the fold.
const HEIGHT: f32 = 6000.0;
const WIDTHS: [f32; 2] = [960.0, 560.0];
const PAGES: [(Page, &str); 6] = [(Page::Home, "home"), (Page::Plugins, "plugins"),
    (Page::Workspaces, "workspaces"), (Page::Activity, "activity"), (Page::Setup, "setup"),
    (Page::Diagnostics, "diagnostics")];
/// Clicks only labels this short: buttons, headings and choices, not prose.
const CLICKABLE: usize = 90;
/// Second clicks per first click, at most.
const REVEALED: usize = 40;

fn hash_output(ctx: &egui::Context, output: egui::FullOutput, hasher: &mut impl Hasher) {
    let mut textures: Vec<_> = output.textures_delta.set.iter().collect();
    textures.sort_by_key(|(id, _)| format!("{id:?}"));
    for (id, deltas) in textures {
        format!("{id:?}").hash(hasher);
        for delta in deltas {
            format!("{:?} {:?}", delta.options, delta.pos).hash(hasher);
            let egui::ImageData::Color(image) = &delta.image;
            image.size.hash(hasher);
            for pixel in &image.pixels { pixel.to_array().hash(hasher); }
        }
    }
    let mut freed: Vec<_> = output.textures_delta.free.iter().map(|id| format!("{id:?}")).collect();
    freed.sort();
    freed.hash(hasher);
    for primitive in ctx.tessellate(output.shapes, output.pixels_per_point) {
        let clip = primitive.clip_rect;
        [clip.min.x, clip.min.y, clip.max.x, clip.max.y].map(f32::to_bits).hash(hasher);
        match primitive.primitive {
            egui::epaint::Primitive::Mesh(mesh) => {
                format!("{:?}", mesh.texture_id).hash(hasher);
                mesh.indices.hash(hasher);
                for vertex in &mesh.vertices {
                    [vertex.pos.x, vertex.pos.y, vertex.uv.x, vertex.uv.y].map(f32::to_bits).hash(hasher);
                    vertex.color.to_array().hash(hasher);
                }
            }
            egui::epaint::Primitive::Callback(_) => "callback".hash(hasher),
        }
    }
}

fn labels(shape: &egui::epaint::Shape, out: &mut Vec<(String, egui::Pos2)>) {
    match shape {
        egui::epaint::Shape::Text(text) => {
            out.push((text.galley.text().to_owned(), text.pos + text.galley.size() * 0.5));
        }
        egui::epaint::Shape::Vec(shapes) => for shape in shapes { labels(shape, out); },
        _ => {}
    }
}

/// One case: frames 0-2, then for each click a press and release and four
/// more frames. Returns the digest and the labels drawn on the last frame.
fn render(theme: egui::Theme, width: f32, clicks: &[egui::Pos2],
    mut draw: impl FnMut(&mut egui::Ui)) -> (u64, Vec<(String, egui::Pos2)>) {
    let ctx = egui::Context::default();
    ctx.set_theme(theme);
    presentation::configure_style(&ctx);
    ctx.global_style_mut(|style| style.animation_time = 0.0);
    crate::operator::PREVIEW_INTENT.with(|intent| intent.borrow_mut().clear());
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let mut seen = Vec::new();
    let last = 2 + 6 * clicks.len();
    for frame in 0..=last {
        let events = match clicks.get(frame.saturating_sub(3) / 6) {
            Some(pos) if frame >= 3 && matches!((frame - 3) % 6, 0 | 1) => vec![egui::Event::PointerMoved(*pos),
                egui::Event::PointerButton { pos: *pos, button: egui::PointerButton::Primary,
                    pressed: (frame - 3) % 6 == 0, modifiers: egui::Modifiers::NONE }],
            _ => vec![],
        };
        let output = ctx.run_ui(egui::RawInput { events, screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO, egui::vec2(width, HEIGHT))), ..Default::default() }, &mut draw);
        if frame == last {
            for clipped in &output.shapes { labels(&clipped.shape, &mut seen); }
        }
        hash_output(&ctx, output, &mut hasher);
        crate::operator::PREVIEW_INTENT.with(|intent| std::mem::take(&mut *intent.borrow_mut())).hash(&mut hasher);
    }
    (hasher.finish(), seen)
}

fn clickable(label: &str) -> bool {
    !label.trim().is_empty() && label.len() <= CLICKABLE && !label.contains('\n')
}

/// Labels drawn after a click that were not drawn before it: new words, or
/// more of the same words (a confirmation repeating its button's label).
fn revealed(before: &[(String, egui::Pos2)], after: Vec<(String, egui::Pos2)>) -> Vec<(String, egui::Pos2)> {
    let mut counts = std::collections::HashMap::new();
    for (label, _) in before { *counts.entry(label.clone()).or_insert(0usize) += 1; }
    let mut chosen = std::collections::BTreeSet::new();
    after.into_iter().filter(|(label, _)| {
        let count = counts.entry(label.clone()).or_insert(0usize);
        if *count > 0 { *count -= 1; return false; }
        clickable(label) && chosen.insert(label.clone())
    }).take(REVEALED).collect()
}

/// The base case, then one case per distinct short label clicked, then one
/// per label that click revealed.
fn cases(name: &str, out: &mut Vec<String>, mut build: impl FnMut() -> Box<dyn FnMut(&mut egui::Ui)>) {
    for (theme, theme_name) in [(egui::Theme::Light, "light"), (egui::Theme::Dark, "dark")] {
        for width in WIDTHS {
            let (digest, seen) = render(theme, width, &[], build());
            out.push(format!("{name}/{theme_name}/{width}\t{digest:016x}"));
            // Clicks change state, not colour: one theme is enough.
            if theme == egui::Theme::Light { continue; }
            let mut clicked = std::collections::BTreeSet::new();
            for (label, pos) in &seen {
                if !clickable(label) || !clicked.insert(label.clone()) { continue; }
                let (digest, after) = render(theme, width, &[*pos], build());
                out.push(format!("{name}/{theme_name}/{width}/click:{label}\t{digest:016x}"));
                for (second, then) in revealed(&seen, after) {
                    let (digest, _) = render(theme, width, &[*pos, then], build());
                    out.push(format!("{name}/{theme_name}/{width}/click:{label}/then:{second}\t{digest:016x}"));
                }
            }
        }
    }
}

fn operator(page: Page, state: &str) -> Operator {
    let snapshot = preview_states::snapshot(page, Some(state));
    let readiness = preview_states::readiness(Some(state), &snapshot);
    Operator::preview_current(snapshot, readiness, page,
        page == Page::Plugins && state != "current_loading", state == "pending",
        matches!(state, "plugin_details" | "plugin_history_open" | "plugin_technical"), state == "stale")
}

#[test]
#[ignore = "diagnostic: writes render digests to LVB_RENDER_DIGESTS"]
fn render_digests() {
    let path = std::env::var("LVB_RENDER_DIGESTS").expect("LVB_RENDER_DIGESTS names the output list");
    let only = std::env::var("LVB_RENDER_ONLY").ok();
    let mut out = Vec::new();
    for (page, page_name) in PAGES {
        for state in preview_states::STATES {
            let name = format!("{page_name}/{state}");
            if only.as_deref().is_some_and(|prefix| !name.starts_with(prefix)) { continue; }
            cases(&name, &mut out, || {
                let mut window = operator(page, state);
                let mut frame = eframe::Frame::_new_kittest();
                Box::new(move |ui: &mut egui::Ui| eframe::App::ui(&mut window, ui, &mut frame))
            });
        }
    }
    // The request bar and window states, over two pages.
    for sample in Operator::REQUEST_SAMPLES {
        for (page, page_name, state) in [(Page::Home, "home", "populated"), (Page::Plugins, "plugins", "plugin_ready")] {
            let name = format!("frame/{sample}/{page_name}");
            if only.as_deref().is_some_and(|prefix| !name.starts_with(prefix)) { continue; }
            cases(&name, &mut out, || {
                let mut window = operator(page, state).with_request_sample(sample);
                let mut frame = eframe::Frame::_new_kittest();
                Box::new(move |ui: &mut egui::Ui| eframe::App::ui(&mut window, ui, &mut frame))
            });
        }
    }
    for (sample, _) in Updates::samples() {
        let name = format!("updates/{sample}");
        if only.as_deref().is_some_and(|prefix| !name.starts_with(prefix)) { continue; }
        cases(&name, &mut out, || {
            let mut updates = Updates::samples().into_iter().find(|(name, _)| *name == sample)
                .expect("sample").1;
            Box::new(move |ui: &mut egui::Ui| updates.show(ui))
        });
        // The one-line headline on another page.
        cases(&format!("{name}/headline"), &mut out, || {
            let updates = Updates::samples().into_iter().find(|(name, _)| *name == sample)
                .expect("sample").1;
            let mut window = operator(Page::Home, "available").with_updates(updates);
            let mut frame = eframe::Frame::_new_kittest();
            Box::new(move |ui: &mut egui::Ui| eframe::App::ui(&mut window, ui, &mut frame))
        });
    }
    let clicks = out.iter().filter(|line| line.contains("/click:")).count();
    let seconds = out.iter().filter(|line| line.contains("/then:")).count();
    std::fs::write(&path, out.join("\n") + "\n").expect("write render digests");
    eprintln!("{} cases ({} after a click, {} of them after a second) written to {path}", out.len(), clicks, seconds);
}
