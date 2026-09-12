//! Optional worker-only input witness. No samples retained, no callback calls.
#[derive(Clone, Copy, Default)]
struct Row { epoch:u64, sequence:u64, position:u64, frames:usize, silence:u64, hash:[u64;2], peak:[f32;2] }
pub struct InputObservation { enabled:bool, rows:[Row;32], count:usize, overflow:u64, last:Option<bool> }
impl InputObservation {
 pub fn new(enabled:bool)->Self {Self{enabled,rows:[Row::default();32],count:0,overflow:0,last:None}}
 pub fn observe(&mut self,epoch:u64,sequence:u64,position:u64,silence:u64,input:[&[f32];2]) {
  if !self.enabled||input[0].is_empty(){return;}
  let mut row=Row{epoch,sequence,position,frames:input[0].len(),silence,..Row::default()};
  for (ch,plane) in input.iter().enumerate(){row.hash[ch]=14695981039346656037;for sample in *plane {for byte in sample.to_bits().to_le_bytes(){row.hash[ch]=(row.hash[ch]^u64::from(byte)).wrapping_mul(1099511628211);}row.peak[ch]=row.peak[ch].max(sample.abs());}}
  let nonzero=row.peak.iter().any(|p| *p>0.);
  if self.last==Some(nonzero){return;}self.last=Some(nonzero);
  if self.count==self.rows.len(){self.overflow+=1;return;}self.rows[self.count]=row;self.count+=1;
 }
 pub fn report(&self)->String {
  if !self.enabled{return String::new();}
  let mut text=format!("{{\"event\":\"ap18_input_observation\",\"retained\":{},\"overflow\":{},\"capacity\":32,\"collection\":\"native_transport_worker\",\"hash\":\"fnv1a64_f32le_per_channel\"}}\n",self.count,self.overflow);
  for r in &self.rows[..self.count]{text+=&format!("{{\"event\":\"ap18_input\",\"epoch\":{},\"request_sequence\":{},\"position\":{},\"frames\":{},\"silence\":{},\"hash\":[{},{}],\"peak\":[{},{}]}}\n",r.epoch,r.sequence,r.position,r.frames,r.silence,r.hash[0],r.hash[1],r.peak[0],r.peak[1]);}text
 }
}
#[cfg(test)] mod tests {
 use super::*;
 #[test] fn transitions_channels_capacity_and_disabled(){
  let mut t=InputObservation::new(true);let z=[0.,0.];let l=[0.25,0.5];let r=[-0.75,-0.125];
  t.observe(1,1,0,3,[&z,&z]);t.observe(1,2,2,0,[&l,&r]);t.observe(1,3,4,0,[&l,&r]);t.observe(1,4,6,3,[&z,&z]);
  assert_eq!(t.rows[1].hash,[1596836236129080722,15537787510424757890]);assert_eq!(t.count,3);assert_eq!(t.rows[1].sequence,2);assert_ne!(t.rows[1].hash[0],t.rows[1].hash[1]);assert_eq!(t.rows[0].hash,t.rows[2].hash);
  for n in 0..80{t.observe(1,n,n,0,if n%2==0{[&l,&r]}else{[&z,&z]});}assert_eq!(t.count,32);assert!(t.overflow>0);
  let mut disabled=InputObservation::new(false);disabled.observe(1,1,0,0,[&l,&r]);assert!(disabled.report().is_empty());
 }
}
