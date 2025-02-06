//! Chaos module.
//!
//! Implements fault injection for simulation. Here we simply modify the event content.

/// Injects a fault into the event by appending a fault string.
pub fn inject_fault(event: &mut String) {
    event.push_str(" [FAULT INJECTED]");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_fault() {
        let mut event = String::from("Test event");
        inject_fault(&mut event);
        assert!(event.contains("FAULT INJECTED"));
    }
}
