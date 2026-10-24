//! DAW blocks longer than one transport block (#402): the proxy accepts a DAW
//! maximum above 1024, sets the Windows side up for 1024, keeps the bridge
//! delay, and plays each long DAW block as consecutive transport parts.
use super::*;
use ap1_native_client::events::{NOTE_OFF, NOTE_ON, PARAMETER};
use std::sync::Mutex;

const D: usize = 1024;

fn input(t: usize, ch: usize) -> f32 {
    ((t * 7919 + ch * 131) % 1000) as f32 / 1000. - 0.5
}
// A stateful reference processor: the Windows plug-in stand-in. Its output
// depends on every earlier sample, so any lost, repeated or reordered frame
// changes all later output.
#[derive(Default, Clone, Copy)]
struct Reference { state: f32, phase: f32 }
impl Reference {
    fn frame(&mut self, left: f32, right: f32) -> (f32, f32) {
        self.state = 0.5 * left + 0.25 * self.state;
        self.phase += 0.001;
        (self.state, right * 0.75 + self.phase)
    }
}

fn live(shared: Arc<Shared>, delivery: crate::performance::DeliveryMode, delay: usize) -> u64 {
    INSTANCES.insert(|| Ok::<_, ()>(Live {
        shared, callback: UnsafeCell::new(Callback::new()), busy: AtomicBool::new(false),
        worker: None, report: None, max: 256, recovery_blocked: false, proxy: None,
        installed_delay: Some(delay as u32), delivery_mode: delivery, minor: 15, setup: None,
    })).unwrap().unwrap()
}
fn stereo_contract() -> Vec<u8> {
    let mut buses = 2u32.to_le_bytes().to_vec();
    for direction in [0u32, 1] {
        for word in [0u32, direction, 0, 2, 0, 1] { buses.extend(word.to_le_bytes()); }
        buses.extend(3u64.to_le_bytes());
    }
    buses
}
// Answer one setup as the Windows side would, reporting `vendor` latency;
// checks that it is set up for the `transport` block.
fn setup(shared: &Arc<Shared>, id: u64, maximum: u32, mode: u32, vendor: u32, transport: u32) -> [u32; 3] {
    let owner = shared.clone();
    let peer = thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(pending) = owner.control.lock().unwrap().as_mut() {
                assert_eq!(pending.op, 20);
                assert_eq!(ap1_native_client::get(&pending.bytes[..4]), u64::from(transport),
                    "the Windows side is set up for one transport block");
                let mut reply = vec![0; 16];
                reply[..4].copy_from_slice(&vendor.to_le_bytes());
                reply[8] = 1;
                complete_control(&owner, pending, Ok(reply), ([0; 5], None)).unwrap();
                break;
            }
            assert!(Instant::now() < until);
            thread::yield_now();
        }
    });
    let buses = stereo_contract();
    let mut traits = [0u32; 3];
    assert_eq!(unsafe { ap10_setup(id, maximum, mode, 48000., buses.as_ptr(), buses.len() as u32, 1,
        traits.as_mut_ptr()) }, 0);
    peer.join().unwrap();
    traits
}
// Activation names the transport block the Windows side was set up for,
// whatever DAW maximum the native side passes.
fn activate(shared: &Arc<Shared>, id: u64, maximum: u32, transport: u32) {
    let owner = shared.clone();
    let peer = thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(pending) = owner.control.lock().unwrap().as_mut() {
                assert_eq!(pending.op, 8);
                assert_eq!(ap1_native_client::get(&pending.bytes[..4]), u64::from(transport));
                complete_control(&owner, pending, Ok(vec![]), ([0; 5], None)).unwrap();
                break;
            }
            assert!(Instant::now() < until);
            thread::yield_now();
        }
    });
    assert_eq!(unsafe { ap4_activate(id, maximum, 0) }, 0);
    peer.join().unwrap();
}
fn start(shared: &Shared, id: u64) {
    assert_eq!(unsafe { ap3_transition(id, START) }, 0);
    assert_eq!(shared.requests.pop().unwrap().kind, START);
    shared.processing_ready_epoch.store(1, Ordering::Release);
}

#[test]
fn a_daw_maximum_above_one_transport_block_keeps_the_transport_block_and_latency() {
    let _registry_owner = crate::registry_test();
    let shared = Arc::new(Shared::new());
    shared.state_capable.store(true, Ordering::Release);
    let id = live(shared.clone(), crate::performance::DeliveryMode::Buffered, D);
    for maximum in [4096u32, 2048, 1025, crate::daw_blocks::HOST_BLOCK_MAX as u32, 512] {
        let transport = maximum.min(1024);
        let traits = setup(&shared, id, maximum, 0, 7, transport);
        assert_eq!(traits, [D as u32 + 7, 0, 7], "reported latency is the 1024-frame bridge delay");
        let l = INSTANCES.lease(id).unwrap();
        assert_eq!(l.max, maximum as usize);
        assert_eq!(unsafe { (*l.callback.get()).audio.maximum }, transport as usize);
        drop(l);
        activate(&shared, id, maximum, transport);
    }
    // A plug-in set to shorter buffering keeps it: its transport block is
    // that buffering, whatever the DAW's block size.
    for (delay, maximum) in [(256usize, 2048u32), (512, 1024), (256, 256), (512, 128)] {
        let shorter = live(shared.clone(), crate::performance::DeliveryMode::Buffered, delay);
        let transport = maximum.min(delay as u32);
        assert_eq!(setup(&shared, shorter, maximum, 0, 7, transport), [delay as u32 + 7, 0, 7]);
        activate(&shared, shorter, maximum, transport);
        INSTANCES.remove(shorter, |_| ()).unwrap();
    }
    // Above the stated bound: refused before anything reaches the Windows side.
    let buses = stereo_contract();
    let mut traits = [0u32; 3];
    assert_ne!(unsafe { ap10_setup(id, crate::daw_blocks::HOST_BLOCK_MAX as u32 + 1, 0, 48000.,
        buses.as_ptr(), buses.len() as u32, 1, traits.as_mut_ptr()) }, 0);
    let mut detail = [0u8; 385];
    unsafe { crate::ap2_error(detail.as_mut_ptr(), detail.len() as u32) };
    let detail = String::from_utf8_lossy(&detail);
    assert!(detail.contains("DAW block size above the bridge's limit"), "{detail}");
    INSTANCES.remove(id, |_| ()).unwrap();
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Seen { position: u64, n: u32, event: Event }
struct Played { seen: Vec<Seen>, sizes: Vec<u32>, long_blocks: u64, anchors: u64, omitted: u64 }

// Realtime, buffered: set up for `maximum`, play `blocks` in place with
// `events` in block `at`, and check every output frame against the reference
// processor fed the same input in one piece, D frames earlier.
fn play(delay: usize, maximum: u32, blocks: &[usize], at: usize, events: &[Event]) -> Played {
    let mut shared = Shared::new();
    shared.identity = Some(state::Identity { class: [3; 16], module: [4; 32] });
    shared.state_capable.store(true, Ordering::Release);
    let shared = Arc::new(shared);
    let id = live(shared.clone(), crate::performance::DeliveryMode::Buffered, delay);
    let transport = maximum.min(delay as u32);
    assert_eq!(setup(&shared, id, maximum, 0, 0, transport)[0], delay as u32,
        "latency is the bridge delay whatever the DAW maximum");
    start(&shared, id);

    // The Windows side stand-in: processes each transport block in order and
    // returns one output event at its last sample and one parameter point
    // at its first, both naming the input sample they belong to.
    let seen = Arc::new(Mutex::new(Vec::<Seen>::new()));
    let sizes = Arc::new(Mutex::new(Vec::<u32>::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let (owner, record, sized, halt) = (shared.clone(), seen.clone(), sizes.clone(), stop.clone());
    let worker = thread::spawn(move || {
        let mut reference = Reference::default();
        loop {
            // Drain everything published before the stop, then end.
            let Some(request) = owner.requests.pop() else {
                if halt.load(Ordering::Acquire) { break; }
                thread::yield_now(); continue;
            };
            assert_eq!(request.kind, AUDIO);
            sized.lock().unwrap().push(request.n);
            let mut seen = record.lock().unwrap();
            for e in &request.events[..request.event_count as usize] {
                assert!(e.valid_host(request.n as usize), "transport event within its block");
                seen.push(Seen { position: request.position, n: request.n, event: *e });
            }
            drop(seen);
            let mut done = Completion::from(request);
            done.audio.flags = 0;
            for i in 0..request.n as usize {
                let (l, r) = reference.frame(request.data[0][i], request.data[1][i]);
                done.audio.data[0][i] = l;
                done.audio.data[1][i] = r;
            }
            if request.n > 0 {
                let returned = &mut done.returned;
                returned.events = 1;
                returned.event[0] = crate::process_results::Event { offset: request.n as i32 - 1,
                    kind: 1, value: request.position + u64::from(request.n) - 1, ..Default::default() };
                returned.points = 1;
                returned.point[0] = crate::process_results::Point { offset: 0, id: 5,
                    value: request.position as f64 };
            }
            assert!(owner.publish_result(done).is_some());
        }
    });

    let total: usize = blocks.iter().sum();
    let largest = blocks.iter().copied().max().unwrap();
    let mut expected = vec![(0f32, 0f32); total];
    let mut reference = Reference::default();
    for (t, slot) in expected.iter_mut().enumerate() { *slot = reference.frame(input(t, 0), input(t, 1)); }
    let mut left = vec![0f32; largest];
    let mut right = vec![0f32; largest];
    let mut begin = 0usize;
    let mut returned = crate::process_results::Packet::default();
    let mut returned_events = 0;
    for (index, &n) in blocks.iter().enumerate() {
        for i in 0..n { left[i] = input(begin + i, 0); right[i] = input(begin + i, 1); }
        let planes = [left.as_mut_ptr(), right.as_mut_ptr()];
        let events: &[Event] = if index == at { events } else { &[] };
        let context = crate::context::Context { present: 1, rate: 48000., state: 0x20602,
            project: begin as i64, continuous: begin as i64, tempo: 120., music: begin as f64 / 24000.,
            ..Default::default() };
        let mut flags = 0; let mut delivery = Delivery::default();
        // In place, as REAPER and Bitwig may call: output planes are the input planes.
        let (rc, allocations) = crate::allocation_test::measure(|| unsafe {
            ap23_process_outputs(id, n as u32, 0, events.as_ptr(), events.len() as u32, &context,
                0, planes[0], planes[1], planes.as_ptr(), 2, &mut flags, &mut delivery, 0)
        });
        assert_eq!(rc, 0, "block {index}");
        assert_eq!(allocations, [0; 3], "the process path never allocates");
        assert_eq!(delivery.missing_frames, 0);
        for i in 0..n {
            let t = begin + i;
            let want = if t < delay { (0., 0.) } else { expected[t - delay] };
            assert_eq!((left[i], right[i]), want, "frame {t}");
        }
        // Output events and points come back rebased to this DAW block.
        loop {
            let (rc, allocations) = crate::allocation_test::measure(|| unsafe {
                ap10_take_results(id, &mut returned) });
            assert_eq!((rc, allocations), (0, [0; 3]));
            if returned.events == 0 && returned.points == 0 { break; }
            for e in &returned.event[..returned.events as usize] {
                assert!((0..n as i32).contains(&e.offset));
                assert_eq!(begin as u64 + e.offset as u64, e.value + delay as u64);
                returned_events += 1;
            }
            for p in &returned.point[..returned.points as usize] {
                assert!((0..n as i32).contains(&p.offset));
                assert_eq!((begin + p.offset as usize) as f64, p.value + delay as f64);
            }
        }
        begin += n;
    }
    stop.store(true, Ordering::Release);
    worker.join().unwrap();
    let sizes = std::mem::take(&mut *sizes.lock().unwrap());
    // Every transport block fits the Windows side's 1024-frame setup, and
    // the parts of each DAW block are consecutive.
    assert!(sizes.iter().all(|&n| (1..=transport).contains(&n)));
    assert_eq!(sizes.iter().map(|&n| n as usize).sum::<usize>(), total);
    assert_eq!(sizes.len(), blocks.iter()
        .map(|&n| n.max(1).div_ceil(transport as usize)).sum::<usize>());
    // Results are due D frames after their input: every block's result
    // except those due after the last DAW block reached the DAW.
    let mut due = 0usize;
    let mut position = 0usize;
    for &n in &sizes { position += n as usize; due += usize::from(position - 1 + delay < total); }
    assert_eq!(returned_events, due);
    let l = INSTANCES.lease(id).unwrap();
    let callback = unsafe { &*l.callback.get() };
    let played = Played { seen: std::mem::take(&mut *seen.lock().unwrap()), sizes,
        long_blocks: callback.long_blocks, anchors: callback.boundary_anchors,
        omitted: callback.boundary_anchors_omitted };
    drop(l);
    assert_eq!(unsafe { ap3_transition(id, STOP) }, 0);
    INSTANCES.remove(id, |_| ()).unwrap();
    played
}

#[test]
fn long_daw_blocks_play_as_transport_parts_bit_exact_without_allocating() {
    let _registry_owner = crate::registry_test();
    // Host events in the second 4096 block: a note held from part 1 to part
    // 3, a pair on the first boundary, and a ramp across every boundary.
    let host_events = [
        Event { offset: 100, kind: NOTE_ON, pitch: 60, id: 1, value: 0.8, ..Default::default() },
        Event { offset: 1023, kind: NOTE_ON, pitch: 62, id: 2, value: 0.8, ..Default::default() },
        Event { offset: 1024, kind: NOTE_OFF, pitch: 62, id: 2, value: 0.5, ..Default::default() },
        Event { offset: 2100, kind: NOTE_OFF, pitch: 60, id: 1, value: 0.5, ..Default::default() },
        Event { offset: 0, kind: PARAMETER, id: 7, value: 0., ..Default::default() },
        Event { offset: 4000, kind: PARAMETER, id: 7, value: 1., ..Default::default() },
    ];
    let played = play(D, 4096, &[4096, 4096, 2049, 1024, 3000, 4096, 1, 4096, 1500, 4096], 1, &host_events);
    // Notes reach the plug-in at the same host samples, in the same order.
    let notes: Vec<(u64, u32, i16)> = played.seen.iter().filter(|s| s.event.kind != PARAMETER)
        .map(|s| (s.position + u64::from(s.event.offset), s.event.kind, s.event.pitch)).collect();
    assert_eq!(notes, vec![(4196, NOTE_ON, 60), (5119, NOTE_ON, 62), (5120, NOTE_OFF, 62),
        (6196, NOTE_OFF, 60)]);
    // The ramp keeps both DAW points and gains exact anchors at each boundary.
    let ramp: Vec<(u64, f64)> = played.seen.iter()
        .filter(|s| s.event.kind == PARAMETER && s.event.id == 7)
        .map(|s| (s.position + u64::from(s.event.offset), s.event.value)).collect();
    assert_eq!(ramp.first(), Some(&(4096, 0.)));
    assert_eq!(ramp.last(), Some(&(8096, 1.)));
    for &(position, value) in &ramp {
        assert!((value - (position - 4096) as f64 / 4000.).abs() < 1e-12, "{position}: {value}");
    }
    assert_eq!(ramp.len(), 2 + 6, "start and end anchors at the three inner boundaries");
    assert_eq!((played.long_blocks, played.anchors, played.omitted), (8, 6, 0));
}

#[test]
fn a_large_announced_maximum_with_small_blocks_is_unchanged_playback() {
    // Bitwig's automatic block size: a large maxSamplesPerBlock, then the
    // device's small blocks. Nothing is split and latency stays 1024.
    let _registry_owner = crate::registry_test();
    let blocks: Vec<usize> = [256usize, 512, 1024, 128, 1024, 1024, 64, 1000]
        .into_iter().cycle().take(40).collect();
    let events = [
        Event { offset: 0, kind: NOTE_ON, pitch: 48, id: 9, value: 0.9, ..Default::default() },
        Event { offset: 255, kind: NOTE_OFF, pitch: 48, id: 9, value: 0.1, ..Default::default() },
        Event { offset: 256, kind: PARAMETER, id: 3, value: 0.5, ..Default::default() },
    ];
    for maximum in [8192, crate::daw_blocks::HOST_BLOCK_MAX as u32] {
        let played = play(D, maximum, &blocks, 3 * 8, &events);
        assert_eq!(played.sizes, blocks.iter().map(|&n| n as u32).collect::<Vec<_>>());
        assert_eq!((played.long_blocks, played.anchors), (0, 0));
        let first: usize = blocks[..24].iter().sum();
        let seen: Vec<(u64, Event)> = played.seen.iter().map(|s| (s.position, s.event)).collect();
        assert_eq!(seen, events.iter().map(|e| (first as u64, *e)).collect::<Vec<_>>());
    }
}

#[test]
fn shorter_buffering_plays_a_larger_daw_block_in_parts_of_that_buffering() {
    // Buffering 256 with REAPER at 2048 used to refuse setup; it now plays
    // in 256-frame parts with the musician's 256-frame latency.
    let _registry_owner = crate::registry_test();
    let played = play(256, 2048, &[2048, 2048, 700, 2048], 9, &[]);
    assert!(played.sizes.iter().all(|&n| n <= 256));
    assert_eq!(played.long_blocks, 4, "every block longer than the buffering is played in parts");
}

#[test]
fn offline_long_blocks_wait_for_each_part_and_stay_exact() {
    let _registry_owner = crate::registry_test();
    let shared = Arc::new(Shared::new());
    shared.state_capable.store(true, Ordering::Release);
    let id = live(shared.clone(), crate::performance::DeliveryMode::SameCallback, D);
    assert_eq!(setup(&shared, id, 8192, 2, 0, 1024)[0], 0, "same-callback delivery has no bridge delay");
    start(&shared, id);
    let stop = Arc::new(AtomicBool::new(false));
    let (owner, halt) = (shared.clone(), stop.clone());
    let worker = thread::spawn(move || {
        let mut reference = Reference::default();
        loop {
            // Drain everything published before the stop, then end.
            let Some(request) = owner.requests.pop() else {
                if halt.load(Ordering::Acquire) { break; }
                thread::yield_now(); continue;
            };
            assert!(request.n as usize <= CAP && request.process_mode == 2);
            let mut done = Completion::from(request);
            done.audio.flags = 0;
            for i in 0..request.n as usize {
                let (l, r) = reference.frame(request.data[0][i], request.data[1][i]);
                done.audio.data[0][i] = l; done.audio.data[1][i] = r;
            }
            assert!(owner.publish_result(done).is_some());
        }
    });
    let mut reference = Reference::default();
    let mut left = vec![0f32; 8192]; let mut right = vec![0f32; 8192];
    let mut output = [vec![0f32; 8192], vec![0f32; 8192]];
    let mut begin = 0;
    for n in [8192usize, 5000, 8192] {
        for i in 0..n { left[i] = input(begin + i, 0); right[i] = input(begin + i, 1); }
        let planes = [output[0].as_mut_ptr(), output[1].as_mut_ptr()];
        let mut flags = 0;
        let (rc, allocations) = crate::allocation_test::measure(|| unsafe {
            ap23_process_outputs(id, n as u32, 2, std::ptr::null(), 0,
                &crate::context::Context::default(), 0, left.as_ptr(), right.as_ptr(),
                planes.as_ptr(), 2, &mut flags, std::ptr::null_mut(), 0)
        });
        assert_eq!((rc, allocations), (0, [0; 3]));
        for i in 0..n {
            assert_eq!((output[0][i], output[1][i]), reference.frame(left[i], right[i]),
                "offline frame {}", begin + i);
        }
        begin += n;
    }
    stop.store(true, Ordering::Release);
    worker.join().unwrap();
    assert_eq!(unsafe { ap3_transition(id, STOP) }, 0);
    INSTANCES.remove(id, |_| ()).unwrap();
}

#[test]
fn notes_held_through_an_unacknowledged_start_span_all_parts_of_a_long_block() {
    // #357/#374: notes from blocks answered before the Windows side
    // acknowledged its start are held and played at sample 0 of the first
    // rendered block. A long DAW block's parts behave as one block: its first
    // part spends the bounded wait, later parts answer at once, and every
    // part's notes are held in DAW order.
    let _registry_owner = crate::registry_test();
    let mut shared = Shared::new();
    shared.identity = Some(state::Identity { class: [3; 16], module: [4; 32] });
    shared.state_capable.store(true, Ordering::Release);
    let shared = Arc::new(shared);
    let id = live(shared.clone(), crate::performance::DeliveryMode::Buffered, D);
    setup(&shared, id, 4096, 0, 0, 1024);
    assert_eq!(unsafe { ap3_transition(id, START) }, 0);
    assert_eq!(shared.requests.pop().unwrap().kind, START);
    let on = |offset, pitch| Event { offset, kind: NOTE_ON, pitch, id: pitch as u32, value: 0.8, ..Default::default() };
    let off = |offset, pitch| Event { offset, kind: NOTE_OFF, pitch, id: pitch as u32, value: 0.5, ..Default::default() };
    let held = [on(100, 60), off(2100, 60), on(3000, 64)];
    let mut left = vec![0.25f32; 4096]; let mut right = vec![0.25f32; 4096];
    let planes = [left.as_mut_ptr(), right.as_mut_ptr()];
    let context = crate::context::Context::default();
    let (mut flags, mut delivery) = (0, Delivery::default());
    let started = Instant::now();
    let (rc, allocations) = crate::allocation_test::measure(|| unsafe {
        ap23_process_outputs(id, 4096, 0, held.as_ptr(), held.len() as u32, &context, 0,
            planes[0], planes[1], planes.as_ptr(), 2, &mut flags, &mut delivery, 0) });
    assert_eq!((rc, allocations), (0, [0; 3]));
    // One bound for the whole DAW block: half its duration (4096 / 48000 / 2).
    assert!(started.elapsed() < Duration::from_millis(400), "{:?}", started.elapsed());
    assert!(left.iter().chain(&right).all(|&v| v == 0.), "unready: silence");
    assert!(shared.requests.pop().is_none(), "no part of an unready block is queued");
    {
        let l = INSTANCES.lease(id).unwrap();
        let callback = unsafe { &*l.callback.get() };
        assert_eq!((callback.unready_callbacks, callback.unready_frames), (4, 4096));
        assert_eq!(callback.startup_notes.pending(), 3);
    }
    // Acknowledged: the first rendered block carries the held notes at
    // sample 0, in DAW order, ahead of its own.
    shared.processing_ready_epoch.store(1, Ordering::Release);
    let own = [off(10, 64)];
    left.fill(0.25); right.fill(0.25);
    let (rc, allocations) = crate::allocation_test::measure(|| unsafe {
        ap23_process_outputs(id, 1024, 0, own.as_ptr(), 1, &context, 0,
            planes[0], planes[1], planes.as_ptr(), 2, &mut flags, &mut delivery, 0) });
    assert_eq!((rc, allocations), (0, [0; 3]), "no result can be due yet: nothing is awaited");
    let first = shared.requests.pop().unwrap();
    let events: Vec<(u32, u32, i16)> = first.events[..first.event_count as usize].iter()
        .map(|e| (e.kind, e.offset, e.pitch)).collect();
    assert_eq!(events, vec![(NOTE_ON, 0, 60), (NOTE_OFF, 0, 60), (NOTE_ON, 0, 64), (NOTE_OFF, 10, 64)]);
    assert_eq!((first.position, first.n), (0, 1024));
    assert!(shared.requests.pop().is_none());
    assert_eq!(unsafe { ap3_transition(id, STOP) }, 0);
    INSTANCES.remove(id, |_| ()).unwrap();
}
