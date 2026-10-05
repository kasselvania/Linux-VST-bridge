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
const REQUEST_FLAG: usize = 64;
const REPLY_FLAG: usize = 128;
const REQUEST: usize = 256;
const REPLY: usize = 16640;
pub(crate) const CAPACITY: usize = 16384;

struct Region { pointer: NonNull<u8>, _file: File }
// Atomic flags transfer the two bounded byte regions to their single owners.
// These traits do not grant concurrent AUDIO producers or consumers.
unsafe impl Send for Region {}
unsafe impl Sync for Region {}
impl Drop for Region {
    fn drop(&mut self) { unsafe { libc::munmap(self.pointer.as_ptr().cast(), BYTES); } }
}
#[derive(Clone)]
pub(crate) struct Endpoint(Arc<Region>);
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
    fn notify(&self, offset:usize) {
        self.word(offset).fetch_add(1,Ordering::Release);
        // Shared backing across Linux and Wine: never FUTEX_PRIVATE_FLAG.
        unsafe { libc::syscall(libc::SYS_futex,self.word(offset).as_ptr(),libc::FUTEX_WAKE,1); }
    }
    pub(crate) fn cancel(&self, code:u32) {
        // Preserve first death/cancel/fault. Neither owner releases AUDIO here.
        let _=self.word(TERMINAL).compare_exchange(0,code.max(1),Ordering::AcqRel,Ordering::Acquire);
        self.notify(REQUEST_WAKE); self.notify(REPLY_WAKE);
    }
    pub(crate) fn control(&self)->Result<(),Failure> {
        if self.terminal()!=0 { return Err(Failure::Cancelled); }
        self.word(CONTROL).compare_exchange(0,1,Ordering::Release,Ordering::Relaxed)
            .map_err(|_|Failure::Ownership)?;
        self.notify(REQUEST_WAKE); Ok(())
    }
    pub(crate) fn control_pending(&self)->bool { self.word(CONTROL).load(Ordering::Acquire)!=0 }
    pub(crate) fn publish(&self, bytes:&[u8])->Result<(),Failure> {
        if self.terminal()!=0 { return Err(Failure::Cancelled); }
        if bytes.len()>CAPACITY { return Err(Failure::Extent); }
        if self.word(REQUEST_FLAG).load(Ordering::Acquire)!=0 || self.word(REPLY_FLAG).load(Ordering::Acquire)!=0 {
            return Err(Failure::Ownership);
        }
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(),self.0.pointer.as_ptr().add(REQUEST),bytes.len()); }
        self.word(68).store(bytes.len() as u32,Ordering::Relaxed);
        self.word(REQUEST_FLAG).store(1,Ordering::Release);
        self.notify(REQUEST_WAKE); Ok(())
    }
    pub(crate) fn receive(&self, deadline:Instant, bytes:&mut [u8])->Result<usize,Failure> {
        loop {
            let observed=self.word(REPLY_WAKE).load(Ordering::Acquire);
            if self.terminal()!=0 { return Err(Failure::Cancelled); }
            if Instant::now()>=deadline { self.cancel(2); return Err(Failure::Expired); }
            match self.word(REPLY_FLAG).load(Ordering::Acquire) {
                0=>{}, 1=>break, _=>{self.cancel(3);return Err(Failure::Ownership);}
            }
            if !crate::completion_wait::wait_shared(self.word(REPLY_WAKE),observed,deadline) {
                if Instant::now()>=deadline { self.cancel(2);return Err(Failure::Expired); }
                self.cancel(3);return Err(Failure::Wait);
            }
        }
        let size=self.word(132).load(Ordering::Relaxed) as usize;
        if size>CAPACITY || size>bytes.len() { self.cancel(3);return Err(Failure::Extent); }
        unsafe {std::ptr::copy_nonoverlapping(self.0.pointer.as_ptr().add(REPLY),bytes.as_mut_ptr(),size);}
        // Complete copy, not timeout/cancel, grants the sole slot's next use.
        self.word(REPLY_FLAG).store(0,Ordering::Release); Ok(size)
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
        let (callback,path)=channel(); let render=callback.clone();
        callback.control().unwrap(); // deliberately unconsumed control work
        let t=std::thread::spawn(move||{
            assert_eq!(render.render_receive(Instant::now()+Duration::from_secs(2)).unwrap(),b"request");
            render.render_reply(b"complete");
        });
        let mut out=[0;32];
        let (n,counts)=crate::allocation_test::measure(|| {
            callback.publish(b"request").unwrap();
            callback.receive(Instant::now()+Duration::from_secs(2),&mut out).unwrap()
        });assert_eq!(n,8);assert_eq!(counts,[0;3]);
        assert_eq!(&out[..8],b"complete");assert!(callback.control_pending());
        t.join().unwrap();std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn death_wakes_both_waiters_and_preserves_first_fault() {
        let (a,path)=channel();let b=a.clone();
        let t=std::thread::spawn(move||b.render_receive(Instant::now()+Duration::from_secs(5)));
        std::thread::sleep(Duration::from_millis(10));a.cancel(7);a.cancel(8);
        assert_eq!(a.terminal(),7);assert_eq!(t.join().unwrap(),Err(Failure::Cancelled));
        assert_eq!(a.receive(Instant::now()+Duration::from_secs(5),&mut [0;8]),Err(Failure::Cancelled));
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn expiry_does_not_release_or_reuse_outstanding_storage() {
        let (a,path)=channel();a.publish(b"owned").unwrap();
        assert_eq!(a.receive(Instant::now(),&mut [0;8]),Err(Failure::Expired));
        assert_eq!(a.word(REQUEST_FLAG).load(Ordering::Acquire),1);
        assert_eq!(a.publish(b"replacement"),Err(Failure::Cancelled));
        std::fs::remove_file(path).unwrap();
    }
}

