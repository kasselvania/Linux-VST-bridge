//! First-party bounded DSP dependency for post-freeze unfamiliar fixtures.
//! The generated plug-ins load this exact generation before activation.
const GENERATION: u32 = match u32::from_str_radix(env!("LVB_BETA_GENERATION"), 16) {
    Ok(value) => value,
    Err(_) => panic!("Exact unfamiliar generation required"),
};

#[unsafe(no_mangle)]
pub extern "C" fn lvb_companion_generation() -> u32 {
    GENERATION
}

#[unsafe(no_mangle)]
pub extern "C" fn lvb_companion_gain(gain: f64, signal: f64) -> f64 {
    gain * signal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dependency_performs_the_fixture_gain_operation() {
        assert_eq!(lvb_companion_generation(), GENERATION);
        assert_eq!(lvb_companion_gain(0.75, 0.75), 0.5625);
        assert_eq!(lvb_companion_gain(0.25, -0.375), -0.09375);
        assert_eq!(lvb_companion_gain(0.0, 1.0), 0.0);
    }
}
