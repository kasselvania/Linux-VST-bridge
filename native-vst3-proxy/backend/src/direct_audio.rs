//! Prepared delivery v4. Only the callback owns AUDIO bytes; control and death
//! owners touch separate atomic words. No socket, state store, or relay is used
//! by request publication or completion waiting.
use std::{fs::File, io, os::fd::AsRawFd, ptr::NonNull,
    sync::{Arc, atomic::{AtomicU32, Ordering}}, time::Instant};

pub(crate) const BYTES: usize = 33024;
pub(crate) const CONTROL: usize = 32;
pub(crate) const TERMINAL: usize = 36;
const REQUEST_WAKE: usize = 40;
const REPLY_WAKE: usize = 44;
const AUDIO_OWNER: usize = 48;
const REQUEST_FLAG: usize = 64;
const REPLY_FLAG: usize = 128;
const REQUEST: usize = 256;
const REPLY: usize = 16640;
pub(crate) const CAPACITY: usize = 16384;

struct Region { pointer: NonNull<u8>, _file: File }
// Only AudioEndpoint may access AUDIO bytes. Its local exclusive claim spans
// publication through the completed reply copy, including render ownership.
unsafe impl Send for Region {}
unsafe impl Sync for Region {}
impl Drop for Region {
    fn drop(&mut self) { unsafe { libc::munmap(self.pointer.as_ptr().cast(), BYTES); } }
}
#[derive(Clone)]
pub(crate) struct Endpoint(Arc<Region>);
pub(crate) struct AudioEndpoint { control: Endpoint, outstanding: bool }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Failure { Ownership, Cancelled, Expired, Wait, Extent }
impl Endpoint {
    /// Inactive preparation. The file is already session-bound by Mailbox.
    pub(crate) fn prepare(file: &File) -> io::Result<Self> {
        let file=file.try_clone()?;
        let raw=unsafe { libc::mmap(std::ptr::null_mut(), BYTES, libc::PROT_READ|libc::PROT_WRITE,
            libc::MAP_SHARED, file.as_raw_fd(), 0) };
        if raw==libc::MAP_FAILED { return Err(io::Error::last_os_error()); }
        Ok(Self(Arc::new(Region { pointer:NonNull::new(raw.cast()).unwrap(), _file:file })))
    }
    fn word(&self, offset:usize)->&AtomicU32 {
        unsafe { &*self.0.pointer.as_ptr().add(offset).cast::<AtomicU32>() }
    }
    pub(crate) fn terminal(&self)->u32 { self.word(TERMINAL).load(Ordering::Acquire) }
    fn notify(&self, offset:usize)->libc::c_long {
        self.word(offset).fetch_add(1,Ordering::Release);
        // Shared backing across Linux and Wine: never FUTEX_PRIVATE_FLAG.
        unsafe { libc::syscall(libc::SYS_futex,self.word(offset).as_ptr(),libc::FUTEX_WAKE,1) }
    }
    pub(crate) fn cancel(&self, code:u32)->[libc::c_long;2] {
        // Preserve first death/cancel/fault. Neither owner releases AUDIO here.
        let _=self.word(TERMINAL).compare_exchange(0,code.max(1),Ordering::AcqRel,Ordering::Acquire);
        [self.notify(REQUEST_WAKE), self.notify(REPLY_WAKE)]
    }
    pub(crate) fn control(&self)->Result<(),Failure> {
        if self.terminal()!=0 { return Err(Failure::Cancelled); }
        self.word(CONTROL).compare_exchange(0,1,Ordering::Release,Ordering::Relaxed)
            .map_err(|_|Failure::Ownership)?;
        self.notify(REQUEST_WAKE); Ok(())
    }
    pub(crate) fn control_pending(&self)->bool { self.word(CONTROL).load(Ordering::Acquire)!=0 }
    pub(crate) fn claim_audio(&self)->Result<AudioEndpoint,Failure> {
        if self.terminal()!=0 { return Err(Failure::Cancelled); }
        self.word(AUDIO_OWNER).compare_exchange(0,1,Ordering::AcqRel,Ordering::Acquire)
            .map_err(|_|Failure::Ownership)?;
        Ok(AudioEndpoint {control:self.clone(),outstanding:false})
    }
    #[cfg(test)]
    fn render_receive(&self, end:Instant)->Result<Vec<u8>,Failure> {
        loop {
            let observed=self.word(REQUEST_WAKE).load(Ordering::Acquire);
            if self.terminal()!=0 {return Err(Failure::Cancelled);}
            if self.word(REQUEST_FLAG).load(Ordering::Acquire)==1 {
                let n=self.word(68).load(Ordering::Relaxed) as usize;
                if n>CAPACITY {return Err(Failure::Extent);}
                let bytes=unsafe {std::slice::from_raw_parts(self.0.pointer.as_ptr().add(REQUEST),n)}.to_vec();
                self.word(REQUEST_FLAG).store(0,Ordering::Release);return Ok(bytes);
            }
            if Instant::now()>=end {return Err(Failure::Expired);}
            if !crate::completion_wait::wait_shared(self.word(REQUEST_WAKE),observed,end) {
                return Err(Failure::Expired);
            }
        }
    }
    #[cfg(test)]
    fn render_reply(&self, bytes:&[u8]) {
        assert_eq!(self.word(REPLY_FLAG).load(Ordering::Acquire),0);
        unsafe {std::ptr::copy_nonoverlapping(bytes.as_ptr(),self.0.pointer.as_ptr().add(REPLY),bytes.len());}
        self.word(132).store(bytes.len() as u32,Ordering::Relaxed);
        self.word(REPLY_FLAG).store(1,Ordering::Release);self.notify(REPLY_WAKE);
    }
}
impl AudioEndpoint {
    pub(crate) fn publish(&mut self, bytes:&[u8])->Result<(),Failure> {
        let control=&self.control;
        if control.terminal()!=0 { return Err(Failure::Cancelled); }
        if self.outstanding {return Err(Failure::Ownership);}
        if bytes.len()>CAPACITY { return Err(Failure::Extent); }
        if control.word(REQUEST_FLAG).load(Ordering::Acquire)!=0 || control.word(REPLY_FLAG).load(Ordering::Acquire)!=0 {
            return Err(Failure::Ownership);
        }
        self.outstanding=true;
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(),control.0.pointer.as_ptr().add(REQUEST),bytes.len()); }
        control.word(68).store(bytes.len() as u32,Ordering::Relaxed);
        control.word(REQUEST_FLAG).store(1,Ordering::Release);
        control.notify(REQUEST_WAKE); Ok(())
    }
    pub(crate) fn receive(&mut self, deadline:Instant, bytes:&mut [u8])->Result<usize,Failure> {
        if !self.outstanding {return Err(Failure::Ownership);}
        let control=&self.control;
        loop {
            let observed=control.word(REPLY_WAKE).load(Ordering::Acquire);
            if control.terminal()!=0 { return Err(Failure::Cancelled); }
            if Instant::now()>=deadline { control.cancel(2); return Err(Failure::Expired); }
            match control.word(REPLY_FLAG).load(Ordering::Acquire) {
                0=>{}, 1=>break, _=>{control.cancel(3);return Err(Failure::Ownership);}
            }
            if !crate::completion_wait::wait_shared(control.word(REPLY_WAKE),observed,deadline) {
                if Instant::now()>=deadline { control.cancel(2);return Err(Failure::Expired); }
                control.cancel(3);return Err(Failure::Wait);
            }
        }
        let size=control.word(132).load(Ordering::Relaxed) as usize;
        if size>CAPACITY || size>bytes.len() { control.cancel(3);return Err(Failure::Extent); }
        unsafe {std::ptr::copy_nonoverlapping(control.0.pointer.as_ptr().add(REPLY),bytes.as_mut_ptr(),size);}
        // A deschedule during copy cannot release a timed-out/cancelled slot.
        if control.terminal()!=0 {return Err(Failure::Cancelled);}
        if Instant::now()>=deadline {control.cancel(2);return Err(Failure::Expired);}
        // Complete copy, not timeout/cancel, grants the sole slot's next use.
        control.word(REPLY_FLAG).store(0,Ordering::Release);self.outstanding=false;Ok(size)
    }
}
impl Drop for AudioEndpoint {
    fn drop(&mut self) {
        if self.outstanding || self.control.terminal()!=0 {self.control.cancel(1);} // never permit reuse of host-owned bytes
        else {self.control.word(AUDIO_OWNER).store(0,Ordering::Release);}
    }
}

#[cfg(all(test,target_os="linux"))]
mod tests {
    use super::*;
    use std::{fs::OpenOptions,time::Duration};
    fn channel()->(Endpoint,std::path::PathBuf) {
        let path=std::env::temp_dir().join(format!("lvb-direct-{}-{}",std::process::id(),crate::observer::monotonic_ns()));
        let file=OpenOptions::new().read(true).write(true).create_new(true).open(&path).unwrap();
        file.set_len(BYTES as u64).unwrap();(Endpoint::prepare(&file).unwrap(),path)
    }
    #[test]
    fn ready_audio_ignores_stalled_control_and_never_allocates_on_callback() {
        let (control,path)=channel(); let render=control.clone();
        let mut callback=control.claim_audio().unwrap();
        control.control().unwrap(); // deliberately unconsumed control work
        let t=std::thread::spawn(move||{
            assert_eq!(render.render_receive(Instant::now()+Duration::from_secs(2)).unwrap(),b"request");
            render.render_reply(b"complete");
        });
        let mut out=[0;32];
        let (n,counts)=crate::allocation_test::measure(|| {
            callback.publish(b"request").unwrap();
            callback.receive(Instant::now()+Duration::from_secs(2),&mut out).unwrap()
        });assert_eq!(n,8);assert_eq!(counts,[0;3]);
        assert_eq!(&out[..8],b"complete");assert!(control.control_pending());
        t.join().unwrap();std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn death_wakes_both_waiters_and_preserves_first_fault() {
        let (control,path)=channel();let render=control.clone();
        let mut callback=control.claim_audio().unwrap();callback.publish(b"owned").unwrap();
        let ready=Arc::new(std::sync::Barrier::new(3));let render_ready=ready.clone();
        let render=std::thread::spawn(move||{
            render.render_receive(Instant::now()+Duration::from_secs(5)).unwrap();
            render_ready.wait();render.render_receive(Instant::now()+Duration::from_secs(5))
        });
        let callback_ready=ready.clone();let callback=std::thread::spawn(move||{
            callback_ready.wait();callback.receive(Instant::now()+Duration::from_secs(5),&mut [0;8])
        });
        ready.wait();std::thread::sleep(Duration::from_millis(20));
        let started=Instant::now();
        assert_eq!(control.cancel(7),[1,1]); // kernel proves both waiters were sleeping
        control.cancel(8);
        assert_eq!(control.terminal(),7);assert_eq!(render.join().unwrap(),Err(Failure::Cancelled));
        assert_eq!(callback.join().unwrap(),Err(Failure::Cancelled));
        assert!(started.elapsed()<Duration::from_millis(100));
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn expiry_does_not_release_or_reuse_outstanding_storage() {
        let (control,path)=channel();let mut a=control.claim_audio().unwrap();a.publish(b"owned").unwrap();
        assert_eq!(a.receive(Instant::now(),&mut [0;8]),Err(Failure::Expired));
        assert_eq!(control.word(REQUEST_FLAG).load(Ordering::Acquire),1);
        assert_eq!(a.publish(b"replacement"),Err(Failure::Cancelled));
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn exclusive_claim_spans_rendering_and_abandoned_exchange() {
        let (control,path)=channel();let mut a=control.claim_audio().unwrap();
        assert!(matches!(control.clone().claim_audio(),Err(Failure::Ownership)));
        let file=OpenOptions::new().read(true).write(true).open(&path).unwrap();
        let alias=Endpoint::prepare(&file).unwrap();
        assert!(matches!(alias.claim_audio(),Err(Failure::Ownership)));
        a.publish(b"first").unwrap();
        assert_eq!(control.render_receive(Instant::now()+Duration::from_secs(1)).unwrap(),b"first");
        // Both shared flags are now zero, but the Windows render still owns this exchange.
        assert_eq!(a.publish(b"second"),Err(Failure::Ownership));
        drop(a);assert_ne!(control.terminal(),0);
        assert!(matches!(control.claim_audio(),Err(Failure::Cancelled)));
        std::fs::remove_file(path).unwrap();
    }
}
