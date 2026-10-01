//! Translate SDK automation curves into bounded transport blocks.
//!
//! A host's point at `frames` is a curve anchor, never a note/sample event.
//! Preserve the curve at each transport boundary; do not clamp that anchor's
//! value onto the last sample. The exact boundary value belongs at offset zero
//! of the next continuous callback unless a new host point or edit supersedes it.
use ap1_native_client::{
    events::{Event, MAX_EVENTS, PARAMETER},
    CAP,
};

const CHUNKS: usize = 1024 / CAP;
// Match the admitted SDK parameter census. Allocate/configure on instance
// preparation, then use bounded binary lookups and generation invalidation.
const PARAMETERS: usize = 8192;
struct KnownValue {
    id: u32,
    generation: u64,
    value: f64,
}
pub(crate) struct Values {
    entries: Vec<KnownValue>,
    generation: u64,
    configured: bool,
}
impl Values {
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
            generation: 1,
            configured: false,
        }
    }
    pub fn configure(&mut self, ids: &[u32]) -> Result<(), ()> {
        if self.configured || ids.len() > PARAMETERS {
            return Err(());
        }
        let mut entries: Vec<_> = ids
            .iter()
            .map(|&id| KnownValue {
                id,
                generation: 0,
                value: 0.,
            })
            .collect();
        entries.sort_unstable_by_key(|entry| entry.id);
        if entries.windows(2).any(|pair| pair[0].id == pair[1].id) {
            return Err(());
        }
        self.entries = entries;
        self.configured = true;
        Ok(())
    }
    fn index(&self, id: u32) -> Option<usize> {
        self.entries
            .binary_search_by_key(&id, |entry| entry.id)
            .ok()
    }
    fn get(&self, id: u32) -> Option<f64> {
        let entry = &self.entries[self.index(id)?];
        (self.generation != 0 && entry.generation == self.generation).then_some(entry.value)
    }
    pub fn invalidate(&mut self) {
        // Exhaustion disables inference rather than reusing an old generation.
        self.generation = if self.generation == 0 {
            0
        } else {
            self.generation.checked_add(1).unwrap_or(0)
        };
    }
    pub fn commit(&mut self, values: &Carry) {
        for point in &values.events[..values.count] {
            if let Some(index) = self.index(point.id) {
                self.entries[index].generation = self.generation;
                self.entries[index].value = point.value;
            }
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) struct Carry {
    pub events: [Event; MAX_EVENTS],
    pub count: usize,
}
impl Carry {
    pub const fn empty() -> Self {
        Self {
            events: [Event {
                offset: 0,
                kind: 0,
                id: 0,
                channel: 0,
                pitch: 0,
                value: 0.,
                tuning: 0.,
                reserved: 0,
            }; MAX_EVENTS],
            count: 0,
        }
    }
}
pub(crate) struct Block {
    pub events: [Event; MAX_EVENTS],
    pub count: usize,
}
impl Block {
    const fn empty() -> Self {
        Self {
            events: Carry::empty().events,
            count: 0,
        }
    }
    fn push(&mut self, mut event: Event, start: usize, frames: usize) -> Result<(), ()> {
        event.offset -= start as u32;
        if self.count == MAX_EVENTS || !event.valid(frames) {
            return Err(());
        }
        self.events[self.count] = event;
        self.count += 1;
        Ok(())
    }
}
pub(crate) struct Plan {
    pub blocks: [Block; CHUNKS],
    pub next: Carry,
    pub last: Carry,
    points: [Event; MAX_EVENTS + 1],
    ids: [u32; MAX_EVENTS],
}
fn value_at(points: &[Event], position: usize, previous: Option<f64>) -> Option<f64> {
    // Keep repeated points in source order. The final point at a position wins;
    // between positions the SDK defines a linear segment, not a step event.
    // The SDK's implicit previous point is at -1, not at the first sample.
    let mut left = previous.map(|value| (-1i64, value));
    for point in points {
        if point.offset as usize <= position {
            left = Some((i64::from(point.offset), point.value));
            continue;
        }
        let (offset, value) = left?;
        let fraction =
            (position as i64 - offset) as f64 / (i64::from(point.offset) - offset) as f64;
        return Some(value + (point.value - value) * fraction);
    }
    left.map(|(_, value)| value)
}
impl Plan {
    pub fn empty() -> Self {
        Self {
            blocks: std::array::from_fn(|_| Block::empty()),
            next: Carry::empty(),
            last: Carry::empty(),
            points: [Event::default(); MAX_EVENTS + 1],
            ids: [0; MAX_EVENTS],
        }
    }
    #[cfg(test)]
    pub fn build(events: &[Event], frames: usize, carry: &Carry) -> Result<Self, ()> {
        let mut plan = Self::empty();
        plan.prepare(events, frames, carry, &Values::empty())?;
        Ok(plan)
    }
    pub fn prepare(
        &mut self,
        events: &[Event],
        frames: usize,
        carry: &Carry,
        previous: &Values,
    ) -> Result<(), ()> {
        if frames > 1024 || events.len() > MAX_EVENTS || carry.count > MAX_EVENTS {
            return Err(());
        }
        for block in &mut self.blocks {
            block.count = 0;
        }
        self.next.count = 0;
        self.last.count = 0;
        let chunks = frames.max(1).div_ceil(CAP);
        let mut ids_count = 0;
        for event in events.iter().chain(&carry.events[..carry.count]) {
            if !event.valid_host(frames) {
                return Err(());
            }
            if event.kind != PARAMETER {
                let chunk = event.offset as usize / CAP;
                let start = chunk * CAP;
                self.blocks[chunk].push(*event, start, (frames - start).min(CAP))?;
            } else if !self.ids[..ids_count].contains(&event.id) {
                if ids_count == MAX_EVENTS {
                    return Err(());
                }
                self.ids[ids_count] = event.id;
                ids_count += 1;
            }
        }
        for &id in &self.ids[..ids_count] {
            if previous.configured && previous.index(id).is_none() {
                return Err(());
            }
            let baseline = previous.get(id);
            let points = &mut self.points;
            let mut length = 0;
            // A current explicit start replaces the predecessor's future anchor.
            let explicit_start = events
                .iter()
                .any(|e| e.kind == PARAMETER && e.id == id && e.offset == 0);
            if !explicit_start {
                if let Some(point) = carry.events[..carry.count].iter().find(|e| e.id == id) {
                    points[0] = *point;
                    length = 1;
                }
            }
            for event in events.iter().filter(|e| e.kind == PARAMETER && e.id == id) {
                if length > 0 && points[length - 1].offset > event.offset {
                    return Err(());
                }
                if length == points.len() {
                    return Err(());
                }
                points[length] = *event;
                length += 1;
            }
            let points = &points[..length];
            for chunk in 0..chunks {
                let start = chunk * CAP;
                let count = (frames - start).min(CAP);
                let end = start + count;
                let block = &mut self.blocks[chunk];
                // Only a split segment needs an extra start. The first segment
                // can retain the vendor's own implicit previous value unchanged.
                if start > 0
                    && points.iter().any(|p| p.offset as usize > start)
                    && !points.iter().any(|p| p.offset as usize == start)
                {
                    let value = value_at(points, start, baseline).ok_or(())?;
                    block.push(
                        Event {
                            offset: start as u32,
                            kind: PARAMETER,
                            id,
                            value,
                            ..Event::default()
                        },
                        start,
                        count,
                    )?;
                }
                for point in points {
                    if (count == 0 && point.offset == 0)
                        || (point.offset as usize >= start && (point.offset as usize) < end)
                    {
                        block.push(*point, start, count)?;
                    }
                }
                if count > 0
                    && points.iter().any(|p| p.offset as usize >= end)
                    && !points.iter().any(|p| p.offset as usize == end - 1)
                {
                    // No descriptor default or guessed current value supplies a
                    // missing left anchor. Refuse before admitting any block.
                    let value = value_at(points, end - 1, baseline).ok_or(())?;
                    block.push(
                        Event {
                            offset: (end - 1) as u32,
                            kind: PARAMETER,
                            id,
                            value,
                            ..Event::default()
                        },
                        start,
                        count,
                    )?;
                }
            }
            if frames > 0 {
                if let Some(value) = value_at(points, frames - 1, baseline) {
                    self.last.events[self.last.count] = Event {
                        offset: 0,
                        kind: PARAMETER,
                        id,
                        value,
                        ..Event::default()
                    };
                    self.last.count += 1;
                }
                if let Some(point) = points.last().filter(|p| p.offset as usize == frames) {
                    if self.next.count == MAX_EVENTS {
                        return Err(());
                    }
                    self.next.events[self.next.count] = Event {
                        offset: 0,
                        ..*point
                    };
                    self.next.count += 1;
                }
            } else if let Some(point) = points.last() {
                self.last.events[self.last.count] = *point;
                self.last.count += 1;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn point(offset: u32, id: u32, value: f64) -> Event {
        Event {
            offset,
            kind: PARAMETER,
            id,
            value,
            ..Event::default()
        }
    }
    #[test]
    fn linear_endpoint_preserves_every_sample_across_transport_boundaries() {
        let points = [
            point(0, 7, 0.125),
            point(337, 7, 0.75),
            point(1024, 7, 0.25),
        ];
        let (plan, allocations) = crate::allocation_test::measure(|| {
            Plan::build(&points, 1024, &Carry::empty()).unwrap()
        });
        assert_eq!(allocations, [0; 3]);
        for (chunk, block) in plan.blocks.iter().enumerate() {
            let slice = &block.events[..block.count];
            assert!(slice.iter().all(|e| e.valid(CAP)));
            for sample in 0..CAP {
                let expected = value_at(&points, chunk * CAP + sample, None).unwrap();
                let actual = value_at(slice, sample, None).unwrap();
                assert!(
                    (actual - expected).abs() < 1e-14,
                    "chunk {chunk} sample {sample}"
                );
            }
        }
        assert_eq!(plan.next.count, 1);
        assert_eq!(plan.next.events[0], point(0, 7, 0.25));
        let held = Plan::build(&[], 512, &plan.next).unwrap();
        assert_eq!(
            &held.blocks[0].events[..held.blocks[0].count],
            &[point(0, 7, 0.25)]
        );
        let replacement = Plan::build(&[point(0, 7, 0.9)], 512, &plan.next).unwrap();
        assert_eq!(
            &replacement.blocks[0].events[..replacement.blocks[0].count],
            &[point(0, 7, 0.9)]
        );
    }
    #[test]
    fn implicit_previous_sample_preserves_a_sparse_curve_and_never_uses_metadata_defaults() {
        let mut previous = Values::empty();
        previous.configure(&[7]).unwrap();
        let mut plan = Plan::empty();
        assert!(plan
            .prepare(&[point(511, 7, 0.9)], 512, &Carry::empty(), &previous)
            .is_err());
        plan.prepare(
            &[point(0, 7, 0.2), point(255, 7, 0.8)],
            256,
            &Carry::empty(),
            &previous,
        )
        .unwrap();
        previous.commit(&plan.last);
        let points = [point(511, 7, 0.9)];
        let (result, allocations) = crate::allocation_test::measure(|| {
            plan.prepare(&points, 512, &Carry::empty(), &previous)
        });
        assert_eq!(result, Ok(()));
        assert_eq!(allocations, [0; 3]);
        let mut vendor_previous = 0.8;
        for (chunk, block) in plan.blocks[..2].iter().enumerate() {
            let points = &block.events[..block.count];
            for sample in 0..CAP {
                let expected = 0.8 + 0.1 * (chunk * CAP + sample + 1) as f64 / 512.;
                assert!(
                    (value_at(points, sample, Some(vendor_previous)).unwrap() - expected).abs()
                        < 1e-14
                );
            }
            vendor_previous = value_at(points, CAP - 1, Some(vendor_previous)).unwrap();
        }
        previous.commit(&plan.last);
        assert_eq!(previous.get(7), Some(0.9));
        previous.invalidate();
        assert!(plan
            .prepare(&points, 512, &Carry::empty(), &previous)
            .is_err());
        plan.prepare(&[point(0, 7, 0.4)], 0, &Carry::empty(), &previous)
            .unwrap();
        previous.commit(&plan.last);
        assert_eq!(previous.get(7), Some(0.4));
    }
    #[test]
    fn exact_metadata_table_has_no_values_until_accepted_input_and_invalidates_in_constant_space() {
        let ids: Vec<_> = (0..8192).rev().collect();
        let mut values = Values::empty();
        values.configure(&ids).unwrap();
        assert!(values.configure(&ids).is_err());
        for id in [0, 4095, 8191] {
            assert_eq!(values.get(id), None);
        }
        let mut update = Carry::empty();
        update.count = 3;
        for (i, id) in [0, 4095, 8191].into_iter().enumerate() {
            update.events[i] = point(0, id, 0.625);
        }
        let (_, allocations) = crate::allocation_test::measure(|| values.commit(&update));
        assert_eq!(allocations, [0; 3]);
        for id in [0, 4095, 8191] {
            assert_eq!(values.get(id), Some(0.625));
        }
        let (_, allocations) = crate::allocation_test::measure(|| values.invalidate());
        assert_eq!(allocations, [0; 3]);
        for id in [0, 4095, 8191] {
            assert_eq!(values.get(id), None);
        }
        values.generation = u64::MAX;
        values.invalidate();
        values.commit(&update);
        assert_eq!(values.get(0), None);
        values.invalidate();
        assert_eq!(values.get(0), None);
        assert!(Values::empty().configure(&[7, 7]).is_err());
        assert!(Values::empty().configure(&vec![7; 8193]).is_err());
    }
    #[test]
    fn implicit_vendor_segment_is_kept_but_unknown_split_baseline_is_refused() {
        let implicit = [point(11, 7, 0.75)];
        let plan = Plan::build(&implicit, 1024, &Carry::empty()).unwrap();
        assert_eq!(&plan.blocks[0].events[..plan.blocks[0].count], &implicit);
        assert!(plan.blocks[1..].iter().all(|b| b.count == 0));
        assert!(Plan::build(&[point(511, 7, 0.75)], 1024, &Carry::empty()).is_err());
        assert!(Plan::build(&[point(32, 7, 0.75)], 32, &Carry::empty()).is_err());
    }
    #[test]
    fn jumps_repeated_points_notes_and_zero_frame_flush_preserve_order() {
        let points = [
            point(0, 7, 0.2),
            point(255, 7, 0.2),
            point(256, 7, 0.8),
            point(256, 7, 0.9),
            point(512, 7, 0.9),
        ];
        let plan = Plan::build(&points, 512, &Carry::empty()).unwrap();
        assert_eq!(&plan.blocks[0].events[..plan.blocks[0].count], &points[..2]);
        assert_eq!(
            &plan.blocks[1].events[..2],
            &[point(0, 7, 0.8), point(0, 7, 0.9)]
        );
        let note = Event {
            offset: 511,
            kind: 1,
            id: 23,
            pitch: 60,
            value: 0.5,
            ..Event::default()
        };
        let plan = Plan::build(
            &[note, point(0, 7, 0.2), point(512, 7, 0.8)],
            512,
            &Carry::empty(),
        )
        .unwrap();
        assert_eq!(
            plan.blocks[1].events[0],
            Event {
                offset: 255,
                ..note
            }
        );
        let flush = [point(0, 9, 0.4)];
        let plan = Plan::build(&flush, 0, &Carry::empty()).unwrap();
        assert_eq!(&plan.blocks[0].events[..plan.blocks[0].count], &flush);
        assert!(Plan::build(&[note], 0, &Carry::empty()).is_err());
    }
    #[test]
    fn overflow_invalid_extent_nonfinite_and_order_fail_before_publication() {
        let full = [Event {
            offset: 1,
            kind: 0,
            pitch: 60,
            value: 0.5,
            ..Event::default()
        }; MAX_EVENTS];
        let mut carry = Carry::empty();
        carry.count = 1;
        carry.events[0] = point(0, 7, 0.5);
        assert!(Plan::build(&full, 1024, &carry).is_err());
        for value in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
            assert!(Plan::build(&[point(0, 7, value)], 32, &Carry::empty()).is_err());
        }
        assert!(Plan::build(&[point(33, 7, 0.5)], 32, &Carry::empty()).is_err());
        assert!(Plan::build(&[point(9, 7, 0.5), point(8, 7, 0.5)], 32, &Carry::empty()).is_err());
        let plan = Plan::build(&[point(0, 7, 0.), point(1, 7, 1.)], 1, &Carry::empty()).unwrap();
        assert_eq!(plan.blocks[0].events[0], point(0, 7, 0.));
        assert_eq!(plan.next.events[0], point(0, 7, 1.));
    }
}
