//! Prepared start-up audio and parameter storage. The callback owns both; no
//! allocation or control owner is involved after activation.
use ap1_native_client::{events::{Event, MAX_EVENTS, PARAMETER}, get, invalid, need, BLOCK_CAP};
use std::io;

pub(crate) const MAX_PARAMETERS: usize = 8192;
pub(crate) const MAX_PARAMETER_PACKETS: usize = MAX_PARAMETERS.div_ceil(MAX_EVENTS);
// The existing transport admits at most this many retained sample frames.
pub(crate) const MAX_DELAY: usize = crate::queued::DESCRIPTORS * BLOCK_CAP;
struct Value { id: u32, value: Option<f64> }
pub(crate) struct Parameters { values: Vec<Value>, configured: bool }
impl Parameters {
    pub fn empty() -> Self { Self { values: Vec::new(), configured: false } }
    pub fn configure(&mut self, ids: &[u32]) -> Result<(), ()> {
        if self.configured || ids.len() > MAX_PARAMETERS { return Err(()); }
        let mut values: Vec<_> = ids.iter().map(|&id| Value { id, value: None }).collect();
        values.sort_unstable_by_key(|v| v.id);
        if values.windows(2).any(|p| p[0].id == p[1].id) { return Err(()); }
        self.values = values; self.configured = true; Ok(())
    }
    pub fn clear(&mut self) { for v in &mut self.values { v.value = None; } }
    pub fn pending(&self) -> bool { self.values.iter().any(|v| v.value.is_some()) }
    pub fn retain(&mut self, events: &[Event]) -> Result<(), ()> {
        for event in events.iter().filter(|e| e.kind == PARAMETER) {
            let index = self.values.binary_search_by_key(&event.id, |v| v.id).map_err(|_| ())?;
            self.values[index].value = Some(event.value);
        }
        Ok(())
    }
    pub fn packet(&mut self, events: &mut [Event; MAX_EVENTS]) -> usize {
        let mut n = 0;
        for v in &mut self.values {
            if let Some(value) = v.value.take() {
                events[n] = Event { kind: PARAMETER, id: v.id, value, ..Event::default() };
                n += 1;
                if n == MAX_EVENTS { break; }
            }
        }
        n
    }
}

pub(crate) struct Dry {
    samples: Box<[f32]>,
    delayed: [[f32; BLOCK_CAP]; 2],
    delay: usize,
    fade_frames: usize,
    output: usize,
    cursor: usize,
    populated: usize,
    fade: usize,
}
impl Dry {
    pub fn prepare(effect: bool, setup: &[u8], vendor: u32, bridge: usize) -> io::Result<Option<Self>> {
        if !effect || setup.len() == 24 { return Ok(None); }
        let input = setup[28..].chunks_exact(32).any(|r| get(&r[..4]) == 0
            && get(&r[4..8]) == 0 && get(&r[8..12]) == 0 && get(&r[12..16]) == 2
            && get(&r[16..20]) == 0 && get(&r[20..24]) == 1 && get(&r[24..32]) == 3);
        let output = setup[28..].chunks_exact(32).find(|r| get(&r[..4]) == 0
            && get(&r[4..8]) == 1 && get(&r[12..16]) == 2
            && get(&r[16..20]) == 0 && get(&r[20..24]) == 1 && get(&r[24..32]) == 3)
            .map(|r| get(&r[8..12]) as usize * 2);
        let Some(output) = output.filter(|_| input) else { return Ok(None); };
        let delay = bridge.checked_add(vendor as usize).ok_or_else(|| invalid("start-up latency overflow"))?;
        need(delay <= MAX_DELAY, "start-up dry latency exceeds retained sample capacity")?;
        let maximum = get(&setup[..4]) as usize;
        let rate = f64::from_le_bytes(setup[8..16].try_into().unwrap());
        need(rate.is_finite() && rate > 0. && rate <= u32::MAX as f64,
            "start-up crossfade sample rate")?;
        let fade_frames = (rate * 0.05).ceil() as usize;
        // A latency-changing parameter can change L while starting. Reserve the
        // existing finite frame ceiling now, then adjust the read distance only.
        let capacity = MAX_DELAY + maximum;
        let mut samples = crate::queue::preallocated(2 * capacity);
        samples.resize(2 * capacity, 0.);
        Ok(Some(Self { samples: samples.into_boxed_slice(), delayed: [[0.; BLOCK_CAP]; 2],
            delay, fade_frames, output, cursor: 0, populated: 0, fade: 0 }))
    }
    pub fn reset(&mut self) { self.cursor = 0; self.populated = 0; self.fade = 0; }
    pub fn active(&self) -> bool { self.fade < self.fade_frames }
    pub fn committed(&self) -> bool { self.fade != 0 }
    pub fn delay(&mut self, delay: usize) -> Result<(), ()> {
        if delay > MAX_DELAY { return Err(()); }
        self.delay = delay; Ok(())
    }
    pub fn feed(&mut self, input: &[[f32; BLOCK_CAP]; 2], n: usize) {
        if !self.active() { return; }
        let capacity = self.samples.len() / 2;
        for i in 0..n {
            for (ch, plane) in input.iter().enumerate() {
                self.samples[ch * capacity + self.cursor] = plane[i];
            }
            self.cursor = (self.cursor + 1) % capacity;
            self.populated = (self.populated + 1).min(capacity);
        }
    }
    pub unsafe fn present(&mut self, main: &mut [[f32; BLOCK_CAP]; 2], extra: &[*mut f32],
        destination: usize, n: usize, wet_position: Option<u64>, wet_delay: u64,
    ) -> u64 {
        let channels = extra.len() + 2;
        // N0 parameter replay may have changed L after this block was copied.
        // Read the prepared history at the latest delay without feeding twice.
        if self.active() {
            let capacity = self.samples.len() / 2;
            for i in 0..n {
                for ch in 0..2 {
                    self.delayed[ch][i] = if self.populated.saturating_sub(n - i) >= self.delay {
                        self.samples[ch * capacity + (self.cursor + 2 * capacity - n + i - self.delay) % capacity]
                    } else { 0. };
                }
            }
        }
        let mut flags = if channels == 64 { u64::MAX } else { (1 << channels) - 1 };
        for i in 0..n {
            let ready = wet_position.is_some_and(|p| p.saturating_add(i as u64) >= wet_delay);
            let mix = if ready {
                self.fade = (self.fade + 1).min(self.fade_frames);
                self.fade as f32 / self.fade_frames as f32
            } else { 0. };
            for ch in 0..channels {
                let pointer = if ch < 2 { main[ch].as_mut_ptr().add(i) }
                    else if extra[ch - 2].is_null() { continue; }
                    else { extra[ch - 2].add(destination + i) };
                let dry = if (self.output..self.output + 2).contains(&ch) {
                    self.delayed[ch - self.output][i]
                } else { 0. };
                let value = if mix == 1. { *pointer } else if mix == 0. { dry }
                    else { dry * (1. - mix) + *pointer * mix };
                *pointer = value;
                if value != 0. { flags &= !(1 << ch); }
            }
        }
        flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup(auxiliary_main: bool) -> Vec<u8> {
        let mut bytes = crate::performance::wire_version(4, 0, 48000., true).unwrap();
        bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
        bytes.extend((if auxiliary_main { 3u32 } else { 2u32 }).to_le_bytes());
        let outputs = if auxiliary_main { 2 } else { 1 };
        for (direction,index,kind) in std::iter::once((0u32,0u32,0u32))
            .chain((0..outputs).map(|index| (1,index,if auxiliary_main {u32::from(index==0)} else {0}))) {
            for value in [0u32,direction,index,2,kind,1] { bytes.extend(value.to_le_bytes()); }
            bytes.extend(3u64.to_le_bytes());
        }
        bytes
    }
    #[test]
    fn startup_values_survive_empty_callbacks_and_coalesce_across_full_census() {
        let ids: Vec<_> = (0..MAX_PARAMETERS as u32).rev().collect();
        let mut values = Parameters::empty(); values.configure(&ids).unwrap();
        let mut packet = [Event::default(); MAX_EVENTS];
        for id in 0..MAX_PARAMETERS as u32 {
            values.retain(&[Event { kind: PARAMETER, id, value: 0.25, ..Event::default() }]).unwrap();
        }
        values.retain(&[Event { kind: PARAMETER, id: 7, value: 0.75, ..Event::default() }]).unwrap();
        values.retain(&[]).unwrap();
        let ((count, seven), allocations) = crate::allocation_test::measure(|| {
            let mut count = 0; let mut seven = None;
            for _ in 0..MAX_PARAMETERS.div_ceil(MAX_EVENTS) {
                let n = values.packet(&mut packet);
                count += n;
                if let Some(e) = packet[..n].iter().find(|e| e.id == 7) { seven = Some(e.value); }
                assert!(packet[..n].iter().all(|e| e.offset == 0 && e.valid_host(0)));
            }
            (count,seven)
        });
        assert_eq!((count,seven), (MAX_PARAMETERS,Some(0.75))); assert_eq!(allocations,[0;3]);
        assert_eq!(values.packet(&mut packet),0);
        assert!(values.retain(&[Event { kind: PARAMETER, id: MAX_PARAMETERS as u32, ..Event::default() }]).is_err());
        values.retain(&[Event { kind: PARAMETER, id: 7, ..Event::default() }]).unwrap();
        values.clear(); assert_eq!(values.packet(&mut packet),0);
    }
    #[test]
    fn startup_dry_matches_reported_delay_with_variable_blocks_and_main_output_mapping() {
        let mut dry = Dry::prepare(true,&setup(true),1,2).unwrap().unwrap();
        let mut output = [[0.;BLOCK_CAP];2]; let mut auxiliary = [[0.;BLOCK_CAP];2];
        let extra = [auxiliary[0].as_mut_ptr(),auxiliary[1].as_mut_ptr()];
        let mut position = 0;
        for n in [1,2,4,1] {
            let mut input = [[0.;BLOCK_CAP];2];
            for i in 0..n { input[0][i] = (position+i+1) as f32; input[1][i] = input[0][i] * 2.; }
            let (flags,allocations) = crate::allocation_test::measure(|| {
                dry.feed(&input,n);
                unsafe { dry.present(&mut output,&extra,0,n,None,0) }
            });
            assert_eq!(allocations,[0;3]); assert_eq!(flags & 3,3);
            for i in 0..n {
                let expected = if position+i < 3 { 0. } else { (position+i-2) as f32 };
                assert_eq!((output[0][i],output[1][i]),(0.,0.));
                assert_eq!((auxiliary[0][i],auxiliary[1][i]),(expected,expected*2.));
            }
            position += n;
        }
        dry.reset(); dry.feed(&[[9.;BLOCK_CAP];2],2);
        unsafe { dry.present(&mut output,&extra,0,2,None,0); }
        assert_eq!(&auxiliary[0][..2],&[0.,0.]);
        assert!(dry.delay(MAX_DELAY+1).is_err());
        assert!(Dry::prepare(false,&setup(false),0,0).unwrap().is_none());
    }
    #[test]
    fn startup_dry_uses_latest_latency_and_waits_through_vendor_priming_before_fading() {
        let mut dry = Dry::prepare(true,&setup(false),0,0).unwrap().unwrap();
        let mut input = [[10.;BLOCK_CAP];2];
        dry.feed(&input,4); let mut output = [[0.;BLOCK_CAP];2];
        unsafe { dry.present(&mut output,&[],0,4,None,0); }
        // Current input is already captured when N0 replay changes L to 2.
        input[0][..4].copy_from_slice(&[20.,30.,40.,50.]); input[1]=input[0];
        dry.feed(&input,4); dry.delay(2).unwrap();
        output[0][..4].fill(1.); output[1][..4].fill(1.);
        unsafe { dry.present(&mut output,&[],0,4,Some(0),2); }
        let blend = |a,frame| a*(1.-frame as f32/2400.)+frame as f32/2400.;
        assert_eq!(&output[0][..4],&[10.,10.,blend(20.,1),blend(30.,2)]);
        dry.feed(&input,2); output[0][..2].fill(1.); output[1][..2].fill(1.);
        unsafe { dry.present(&mut output,&[],0,2,Some(4),2); }
        assert_eq!(&output[0][..2],&[blend(40.,3),blend(50.,4)]);
        assert!(dry.active()); assert!(dry.committed());
    }
    #[test]
    fn crossfade_is_fifty_milliseconds_across_rates_variable_blocks_and_silent_wet() {
        for rate in [44100u32,48000,96000,192000] {
            let mut bytes=setup(false);
            bytes[8..16].copy_from_slice(&(rate as f64).to_le_bytes());
            let mut dry=Dry::prepare(true,&bytes,0,0).unwrap().unwrap();
            let frames=rate as usize/20;let mut position=0;let input=[[1.;BLOCK_CAP];2];
            let mut output=[[0.;BLOCK_CAP];2];
            while position<frames+7 {
                let n=[1,3,4,2][position%4].min(frames+7-position);
                for plane in &mut output {plane[..n].fill(0.);} // valid silent effect output
                let (_,allocations)=crate::allocation_test::measure(|| {
                    dry.feed(&input,n);
                    unsafe {dry.present(&mut output,&[],0,n,Some(position as u64),0)}
                });
                assert_eq!(allocations,[0;3]);
                for i in 0..n {
                    let mix=((position+i+1).min(frames)) as f32/frames as f32;
                    assert_eq!(output[0][i],1.-mix);
                    assert_eq!(output[1][i],output[0][i]);
                }
                position+=n;assert_eq!(dry.active(),position<frames);
            }
            dry.reset();assert!(dry.active());assert!(!dry.committed());
            dry.feed(&input,4);output=[[0.;BLOCK_CAP];2];
            unsafe {dry.present(&mut output,&[],0,4,None,0);}
            assert_eq!(&output[0][..4],&[1.;4]);assert!(!dry.committed());
        }
    }
}
