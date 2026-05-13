use super::UserProcess;
use super::linux_abi::LINUX_PERSONALITY_QUERY;

pub(super) fn apply_personality_request(process: &UserProcess, persona: usize) -> usize {
    let old = process.personality();
    if persona != LINUX_PERSONALITY_QUERY {
        process.set_personality(persona);
    }
    old
}
