//! Source-file eligibility for explicit installer selection, not execution trust.
//! Imported bytes acquire private manager custody before any supervised launch.
pub fn accepted_owner(owner_uid: u32, mode: u32, operator_uid: u32) -> bool {
    owner_uid == operator_uid || (owner_uid == 0 && mode & 0o022 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_accepts_operator_or_protected_system_source() {
        assert!(accepted_owner(1000, 0o600, 1000));
        assert!(accepted_owner(0, 0o644, 1000));
        assert!(accepted_owner(0, 0o400, 1000));
        assert!(!accepted_owner(0, 0o664, 1000));
        assert!(!accepted_owner(0, 0o646, 1000));
        assert!(!accepted_owner(1001, 0o444, 1000));
    }
}
