use crate::direct_audio::{AudioEndpoint, Endpoint, Failure, CAPACITY};
use std::time::Instant;

/// Callback-owned AUDIO state. It deliberately has no socket, capture parser,
/// snapshot store, manager owner or mutable control-sequence state.
pub(crate) struct AudioSession {
    pub(crate) channel: Endpoint,
    audio: AudioEndpoint,
    pub(crate) mapping: ap1_native_client::mapping::Mapping,
    session: [u8;16],
    epoch:u64,
    next:u64,
    position:u64,
    mode:u32,
    payload:Vec<u8>,
    wire:Vec<u8>,
    reply:Vec<u8>,
}
unsafe impl Send for AudioSession {}
pub(crate) struct Completed {
    pub(crate) flags:u64,
    pub(crate) returned:crate::process_results::Packet,
    pub(crate) sequence:u64,
    pub(crate) process_ns:u64,
    pub(crate) notices:u32,
    pub(crate) traits:u64,
}
impl AudioSession {
    pub(crate) fn prepare(channel:Endpoint,mapping:ap1_native_client::mapping::Mapping,
        session:[u8;16],mode:u32,epoch:u64)->Result<Self,Failure> {
        let audio=channel.claim_audio()?;
        Ok(Self {channel,audio,mapping,session,epoch,next:1,position:0,mode,
            payload:Vec::with_capacity(8360),wire:vec![0;CAPACITY],reply:vec![0;CAPACITY]})
    }
    pub(crate) fn process(&mut self,item:&mut crate::queued::Item,deadline:Instant)->Result<Completed,Failure> {
        use ap1_native_client::{get,put,HEADER,INPUT,BLOCK_CAP};
        use ap1_native_client::mapping::barrier;
        let n=item.n as usize;
        if n>self.mapping.capacity || item.event_count as usize>ap1_native_client::events::MAX_EVENTS
            || item.process_mode>2 || (item.process_mode==2)!=(self.mode==2)
            || !item.gain.is_nan() || item.flags>3 {return Err(Failure::Extent);}
        if item.epoch!=self.epoch {
            if self.epoch.checked_add(1)!=Some(item.epoch) || item.position!=0 || item.ticket!=1 {
                return Err(Failure::Ownership);
            }
            self.epoch=item.epoch;self.next=1;self.position=0;
        }
        if item.ticket!=self.next || item.position!=self.position || self.next==u64::MAX
            || self.position.checked_add(n as u64).is_none() {return Err(Failure::Ownership);}
        let channels=self.mapping.output_channels;
        if !(2..=64).contains(&channels) {return Err(Failure::Extent);}
        // Normal AUDIO touches actual N only. The prepared map/stride and
        // admitted channel census establish extent; prototype poison/unused
        // plane scans stay in the diagnostic/reference transport.
        let mut plane=[0u8;ap1_native_client::BLOCK_CAP*4];
        if n!=0 {
            for ch in 0..2 {
                for (sample,bytes) in item.data[ch][..n].iter().zip(plane.chunks_exact_mut(4)) {
                    if !sample.is_finite() || item.flags&(1u64<<ch)!=0 && *sample!=0. {return Err(Failure::Extent);}
                    bytes.copy_from_slice(&sample.to_le_bytes());
                }
                if !self.mapping.write_rt(INPUT+ch*self.mapping.stride+4,&plane[..n*4]) {
                    return Err(Failure::Extent);
                }
            }
            barrier();
        }
        self.payload.clear();self.payload.resize(32,0);
        for (offset,value) in [(0,n as u64),(4,INPUT as u64),(8,self.mapping.output as u64),
            (12,self.mapping.stride as u64),(24,item.flags)] {
            put(&mut self.payload[offset..offset+4],value);
        }
        self.payload.extend(item.epoch.to_le_bytes());self.payload.extend(item.position.to_le_bytes());
        let start=self.payload.len();let count=item.event_count as usize;
        self.payload.resize(start+8+count*32,0);put(&mut self.payload[start..start+4],count as u64);
        for (event,bytes) in item.events[..count].iter().zip(self.payload[start+8..].chunks_exact_mut(32)) {
            if !event.valid_host(n) {return Err(Failure::Extent);}
            put(&mut bytes[..4],event.offset as u64);put(&mut bytes[4..8],event.kind as u64);
            put(&mut bytes[8..12],event.id as u64);bytes[12..14].copy_from_slice(&event.channel.to_le_bytes());
            bytes[14..16].copy_from_slice(&event.pitch.to_le_bytes());bytes[16..24].copy_from_slice(&event.value.to_le_bytes());
            bytes[24..28].copy_from_slice(&event.tuning.to_le_bytes());
        }
        if !item.context.valid() {return Err(Failure::Extent);}
        item.context.encode_into(&mut self.payload);
        self.payload.extend(item.gui_revision.to_le_bytes());
        self.payload.extend(item.process_mode.to_le_bytes());self.payload.extend(0u32.to_le_bytes());
        let size=HEADER+self.payload.len();if size>self.wire.len() {return Err(Failure::Extent);}
        self.wire[..HEADER].fill(0);put(&mut self.wire[..4],0x3141504c);
        put(&mut self.wire[4..6],1);put(&mut self.wire[6..8],15);put(&mut self.wire[8..10],3);
        put(&mut self.wire[12..16],self.payload.len() as u64);self.wire[16..32].copy_from_slice(&self.session);
        put(&mut self.wire[32..40],1);put(&mut self.wire[40..48],self.next);
        self.wire[HEADER..size].copy_from_slice(&self.payload);
        if Instant::now()>=deadline {self.channel.cancel(2);return Err(Failure::Expired);}
        self.audio.publish(&self.wire[..size])?;
        let size=self.audio.receive(deadline,&mut self.reply)?;
        let bytes=&self.reply[..size];
        if size<HEADER+72 || get(&bytes[..4])!=0x3141504c || get(&bytes[4..6])!=1
            || get(&bytes[6..8])!=15 || get(&bytes[8..10])!=4 || get(&bytes[10..12])!=0
            || get(&bytes[12..16]) as usize!=size-HEADER || bytes[16..32]!=self.session
            || get(&bytes[32..40])!=1 || get(&bytes[40..48])!=self.next || get(&bytes[48..56])!=0 {
            self.channel.cancel(3);return Err(Failure::Ownership);
        }
        let p=&bytes[HEADER..];let flags=get(&p[8..16]);
        if get(&p[..4])!=n as u64 || get(&p[4..8])!=self.mapping.output as u64
            || get(&p[16..24])!=item.epoch || get(&p[24..32])!=item.position
            || channels<64 && flags>>channels!=0 || get(&p[40..44])&!10!=0 || get(&p[52..56])!=0 {
            self.channel.cancel(3);return Err(Failure::Ownership);
        }
        let returned=crate::process_results::Packet::decode_rt(&p[56..],n).map_err(|_|Failure::Extent)?;
        if n!=0 {
            barrier();
            for ch in 0..channels {
                if !self.mapping.read_rt(self.mapping.output+ch*self.mapping.stride+4,&mut plane[..n*4]) {
                    return Err(Failure::Extent);
                }
                for (i,bytes) in plane[..n*4].chunks_exact(4).enumerate() {
                    let sample=f32::from_le_bytes(bytes.try_into().unwrap());
                    if !sample.is_finite() || flags&(1u64<<ch)!=0 && sample!=0. {return Err(Failure::Extent);}
                    if ch<2 {item.data[ch][i]=sample;} else {self.mapping.extra[ch-2][i]=sample;}
                }
            }
        }
        if self.channel.terminal()!=0 {return Err(Failure::Cancelled);}
        if Instant::now()>=deadline {self.channel.cancel(2);return Err(Failure::Expired);}
        let completed=Completed {flags,returned,sequence:self.next,process_ns:get(&p[32..40]),notices:get(&p[40..44]) as u32,traits:get(&p[44..52])};
        self.next+=1;self.position+=n as u64;
        // Keep the compiler honest about fixed actual negotiated storage.
        debug_assert!(self.mapping.capacity<=BLOCK_CAP);
        Ok(completed)
    }
}
