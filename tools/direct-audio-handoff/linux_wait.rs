//! Original Linux x86-64 ELF entry for the pinned Wine Unix-call boundary.
//! No allocator, libc, TLS, exception, filesystem or process dependency.
#![no_std]
use core::arch::asm;
#[repr(C)]
pub struct Arguments {
    address: u64,
    sec: i64,
    nsec: i64,
    expected: u32,
    operation: u32,
    result: i64,
}
#[unsafe(no_mangle)]
unsafe extern "C" fn lvb_linux_futex_v1(raw: *mut Arguments) -> i32 {
    let args = unsafe { &mut *raw };
    let result: i64;
    match args.operation {
        0 => unsafe {
            asm!("syscall", inlateout("rax") 228i64 => result,
                 in("rdi") 1u64, in("rsi") (&mut args.sec as *mut i64) as u64,
                 lateout("rcx") _, lateout("r11") _, options(nostack));
        },
        1 => unsafe {
            asm!("syscall", inlateout("rax") 202i64 => result,
                 in("rdi") args.address, in("rsi") 9u64, in("rdx") args.expected as u64,
                 in("r10") (&args.sec as *const i64) as u64, in("r8") 0u64,
                 in("r9") 0xffffffffu64, lateout("rcx") _, lateout("r11") _, options(nostack));
        },
        2 => unsafe {
            asm!("syscall", inlateout("rax") 202i64 => result,
                 in("rdi") args.address, in("rsi") 1u64, in("rdx") 1u64,
                 in("r10") 0u64, in("r8") 0u64, in("r9") 0u64,
                 lateout("rcx") _, lateout("r11") _, options(nostack));
        },
        _ => { args.result = -22; return 0; }
    }
    args.result = result;
    0
}
#[unsafe(no_mangle)]
pub static __wine_unix_call_funcs: [unsafe extern "C" fn(*mut Arguments) -> i32; 1] = [lvb_linux_futex_v1];

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    unsafe { asm!("ud2", options(noreturn)) }
}
