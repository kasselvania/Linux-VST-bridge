//! A DAW block longer than one transport block (BLOCK_CAP frames) is played as
//! consecutive parts of at most BLOCK_CAP frames inside the same DAW callback.
//! The Windows plug-in is set up for the transport block, so it never sees a
//! block larger than the one it accepted.
//!
//! This module only plans each part's input events; it never allocates.
//! - Note events keep their DAW order and move to the part containing their
//!   sample, rebased to that part.
//! - Every DAW parameter point is kept, rebased to its part. A point exactly
//!   at the block end stays a final anchor of the last part.
//! - A linear segment between two DAW points that crosses a part boundary
//!   gains an anchor at the last sample before the boundary and one at the
//!   first sample after it, with the segment's exact interpolated values, so
//!   every part reproduces the DAW's curve sample for sample.
//! - A segment whose left end is the plug-in's own previous value (the block's
//!   first point lies in a later part) has no known left value; that part ramps
//!   from the plug-in's current value instead. The DAW's point values and
//!   positions are always kept.
//! - Anchors always fit: each needs a DAW point of its parameter outside the
//!   part, so a part's events plus anchors never exceed the DAW block's 256.
//!   Were a part ever full, an anchor would be left out and counted; a DAW
//!   event never is.
use ap1_native_client::{
    events::{Event, MAX_EVENTS, PARAMETER},
    BLOCK_CAP,
};

/// Largest DAW block (maxSamplesPerBlock) the proxy accepts. Larger setups
/// are refused with a plain reason. PipeWire's default quantum limit is 8192;
/// this leaves room for hosts that announce a generous maximum.
pub const HOST_BLOCK_MAX: usize = 32768;
/// Parts at most in one DAW callback.
pub const MAX_PARTS: usize = HOST_BLOCK_MAX / BLOCK_CAP;

/// Number of transport parts for an `n`-frame DAW block.
pub fn parts(n: usize) -> usize {
    n.max(1).div_ceil(BLOCK_CAP)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PartEvents {
    pub count: usize,
    pub anchors: u32,
    pub anchors_omitted: u32,
}

/// The value of parameter `id`'s DAW curve at `position`, when `position`
/// lies strictly between two DAW points of that parameter. None when a DAW
/// point is at `position` (nothing to add) or either end is not a DAW point.
fn interpolated(events: &[Event], id: u32, position: usize) -> Option<f64> {
    let mut left: Option<(usize, f64)> = None;
    for e in events.iter().filter(|e| e.kind == PARAMETER && e.id == id) {
        let offset = e.offset as usize;
        if offset == position {
            return None;
        }
        if offset < position {
            // Repeated points at one position: the final one is the value.
            left = Some((offset, e.value));
            continue;
        }
        let (start, value) = left?;
        let fraction = (position - start) as f64 / (offset - start) as f64;
        return Some(value + (e.value - value) * fraction);
    }
    None
}

/// Write the events of part `[start, start + count)` of an `n`-frame DAW
/// block into `out`, rebased to the part. `events` were validated against
/// the whole block and number at most MAX_EVENTS.
pub fn part_events(
    events: &[Event],
    n: usize,
    start: usize,
    count: usize,
    out: &mut [Event; MAX_EVENTS],
) -> PartEvents {
    debug_assert!(events.len() <= MAX_EVENTS && count <= BLOCK_CAP && start + count <= n);
    let end = start + count;
    let last = end == n;
    let mut result = PartEvents::default();
    // Reserve the front for start anchors: count them first.
    let distinct = |i: usize| {
        let e = &events[i];
        e.kind == PARAMETER && !events[..i].iter().any(|p| p.kind == PARAMETER && p.id == e.id)
    };
    let inside = |e: &Event| {
        let offset = e.offset as usize;
        (offset >= start && offset < end) || (last && e.kind == PARAMETER && offset == n)
    };
    let originals = events.iter().filter(|e| inside(e)).count();
    let mut free = MAX_EVENTS - originals;
    let mut used = 0;
    if start > 0 {
        for i in 0..events.len() {
            if !distinct(i) { continue; }
            let id = events[i].id;
            let Some(value) = interpolated(events, id, start) else { continue; };
            if free == 0 { result.anchors_omitted += 1; continue; }
            out[used] = Event { offset: 0, kind: PARAMETER, id, value, ..Event::default() };
            used += 1;
            free -= 1;
            result.anchors += 1;
        }
    }
    for e in events.iter().filter(|e| inside(e)) {
        out[used] = Event { offset: e.offset - start as u32, ..*e };
        used += 1;
    }
    if !last {
        for i in 0..events.len() {
            if !distinct(i) { continue; }
            let id = events[i].id;
            let Some(value) = interpolated(events, id, end - 1) else { continue; };
            if free == 0 { result.anchors_omitted += 1; continue; }
            out[used] = Event {
                offset: (count - 1) as u32, kind: PARAMETER, id, value, ..Event::default()
            };
            used += 1;
            free -= 1;
            result.anchors += 1;
        }
    }
    result.count = used;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ap1_native_client::events::{NOTE_OFF, NOTE_ON};
    fn point(offset: u32, id: u32, value: f64) -> Event {
        Event { offset, kind: PARAMETER, id, value, ..Event::default() }
    }
    fn note(kind: u32, offset: u32, pitch: i16) -> Event {
        Event { offset, kind, pitch, value: 0.75, id: pitch as u32, ..Event::default() }
    }
    fn plan(events: &[Event], n: usize) -> Vec<(Vec<Event>, PartEvents)> {
        (0..parts(n)).map(|part| {
            let start = part * BLOCK_CAP;
            let count = (n - start).min(BLOCK_CAP);
            let mut out = [Event::default(); MAX_EVENTS];
            let (planned, allocations) = crate::allocation_test::measure(||
                part_events(events, n, start, count, &mut out));
            assert_eq!(allocations, [0; 3], "planning a part never allocates");
            for e in &out[..planned.count] {
                assert!(e.valid_host(count), "part event within its part: {e:?}");
            }
            (out[..planned.count].to_vec(), planned)
        }).collect()
    }
    // The SDK's piecewise-linear reading of one queue within one block, with
    // the plug-in's previous value `before` ahead of the first point.
    fn curve(points: &[Event], frames: usize, before: f64) -> Vec<f64> {
        (0..frames).map(|position| {
            let mut left = (-1i64, before);
            for p in points {
                if p.offset as usize <= position { left = (p.offset as i64, p.value); continue; }
                let fraction = (position as i64 - left.0) as f64 / (p.offset as i64 - left.0) as f64;
                return left.1 + (p.value - left.1) * fraction;
            }
            left.1
        }).collect()
    }

    #[test]
    fn notes_keep_order_and_pairing_across_parts() {
        // Note on in part 1, its off in part 3; a pair exactly on a boundary.
        let events = [note(NOTE_ON, 10, 60), note(NOTE_ON, 1023, 62), note(NOTE_OFF, 1024, 62),
            note(NOTE_ON, 2048, 64), note(NOTE_OFF, 2600, 60), note(NOTE_OFF, 4095, 64)];
        let parts = plan(&events, 4096);
        assert_eq!(parts.len(), 4);
        let offsets: Vec<Vec<(u32, u32, i16)>> = parts.iter()
            .map(|(e, _)| e.iter().map(|e| (e.kind, e.offset, e.pitch)).collect()).collect();
        assert_eq!(offsets, vec![
            vec![(NOTE_ON, 10, 60), (NOTE_ON, 1023, 62)],
            vec![(NOTE_OFF, 0, 62)],
            vec![(NOTE_ON, 0, 64), (NOTE_OFF, 552, 60)],
            vec![(NOTE_OFF, 1023, 64)],
        ]);
        // Reassembled in host time, the stream is the DAW's stream unchanged.
        let rebuilt: Vec<Event> = parts.iter().enumerate().flat_map(|(i, (e, _))|
            e.iter().map(move |e| Event { offset: e.offset + (i * BLOCK_CAP) as u32, ..*e })).collect();
        assert_eq!(rebuilt, events);
    }

    #[test]
    fn automation_spanning_parts_is_the_same_curve_sample_for_sample() {
        let n = 4096;
        // Two parameters: one ramp crossing three boundaries, one with a
        // final anchor exactly at the block end and a repeated position.
        let events = [point(0, 7, 0.125), point(337, 7, 0.75), point(3500, 7, 0.25),
            point(1500, 9, 0.5), point(1500, 9, 0.6), point(4096, 9, 1.0),
            note(NOTE_ON, 2000, 60)];
        let parts = plan(&events, n);
        let before = 0.4; // the plug-in's own value ahead of each queue
        for id in [7u32, 9] {
            let whole: Vec<Event> = events.iter().filter(|e| e.kind == PARAMETER && e.id == id)
                .copied().collect();
            let reference = curve(&whole, n, before);
            let mut previous = before;
            let mut joined = Vec::new();
            for (part, (planned, _)) in parts.iter().enumerate() {
                let count = (n - part * BLOCK_CAP).min(BLOCK_CAP);
                let queue: Vec<Event> = planned.iter().filter(|e| e.kind == PARAMETER && e.id == id)
                    .copied().collect();
                assert!(queue.windows(2).all(|w| w[0].offset <= w[1].offset), "ordered queue");
                let values = curve(&queue, count, previous);
                previous = queue.last().map_or(previous, |p| p.value);
                joined.extend(values);
            }
            // Parameter 9's lead-in to its first point (1500, in part 2)
            // ramps from the plug-in's own value within part 2 only; from
            // that DAW point on, the curve is the DAW's.
            let from = if id == 9 { 1500 } else { 0 };
            for position in from..n {
                assert!((joined[position] - reference[position]).abs() < 1e-12,
                    "id {id} sample {position}: {} vs {}", joined[position], reference[position]);
            }
        }
        // The block-end anchor stays the last part's final anchor.
        assert!(parts[3].0.iter().any(|e| e.id == 9 && e.offset == 1024 && e.value == 1.0));
        assert!(parts.iter().all(|(_, p)| p.anchors_omitted == 0));
    }

    #[test]
    fn single_part_blocks_are_unchanged() {
        let events = [note(NOTE_ON, 0, 60), point(5, 3, 0.5), point(1024, 3, 0.25),
            note(NOTE_OFF, 1023, 60)];
        let parts = plan(&events, 1024);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].0, events);
        assert_eq!(parts[0].1.anchors, 0);
    }

    #[test]
    fn anchors_always_fit_beside_the_daw_events() {
        // Each anchor needs a DAW point of its parameter outside the part, so
        // a part's own events plus its anchors never exceed the DAW block's
        // 256. The fullest case: 252 notes in part 2 and two ramps across it.
        let mut events = vec![point(0, 1, 0.), point(3000, 1, 1.), point(0, 2, 0.), point(3000, 2, 1.)];
        for i in 0..252 { events.push(note(NOTE_ON, 1024 + i, 60)); }
        let parts = plan(&events, 3072);
        let (second, planned) = &parts[1];
        assert_eq!(second.len(), MAX_EVENTS);
        assert_eq!(second.iter().filter(|e| e.kind == NOTE_ON).count(), 252);
        assert_eq!((planned.anchors, planned.anchors_omitted), (4, 0));
        assert!(parts.iter().all(|(_, p)| p.anchors_omitted == 0));
    }

    #[test]
    fn the_stated_bound_divides_into_whole_parts() {
        assert_eq!(HOST_BLOCK_MAX % BLOCK_CAP, 0);
        assert_eq!(parts(1), 1);
        assert_eq!(parts(BLOCK_CAP), 1);
        assert_eq!(parts(BLOCK_CAP + 1), 2);
        assert_eq!(parts(HOST_BLOCK_MAX), MAX_PARTS);
    }
}
