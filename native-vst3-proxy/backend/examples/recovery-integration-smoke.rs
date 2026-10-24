// Runs the transport half without compiling unrelated Linux-only unit tests.
// The SDK callback/fence regression is bridge-instance-restart-real in CTest.
unsafe extern "C" {
    fn lvb_test_recovery_smoke();
}
fn main() {
    assert_eq!(ap2_backend::ap2_abi_version(), 1);
    unsafe {
        lvb_test_recovery_smoke();
    }
}
