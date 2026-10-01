use crate::controller::Binding;
use std::collections::BTreeMap;

pub const CONTROLS: [(&str, u8, u8); 3] = [("scale", 74, 0), ("arp", 73, 0), ("chordMap", 29, 2)];
#[derive(Clone, Copy)]
enum Phase {
    Query,
    Reset,
    Armed,
    Failed,
}
struct Claim {
    id: &'static str,
    cc: u8,
    baseline: u8,
    original: Option<u8>,
    phase: Phase,
    since: f32,
}
#[derive(Default)]
pub struct FeatureControls {
    claims: Vec<Claim>,
}
#[derive(Default)]
pub struct Update {
    pub messages: Vec<Vec<u8>>,
    pub action: Option<String>,
    pub warning: bool,
}
impl FeatureControls {
    pub fn owns(&self, cc: u8) -> bool {
        self.claims.iter().any(|c| c.cc == cc)
    }
    pub fn reset(&mut self) {
        self.claims.clear();
    }
    pub fn synchronize(
        &mut self,
        bindings: Option<&BTreeMap<String, Binding>>,
        now: f32,
    ) -> Update {
        let mut out = Update::default();
        let wanted = |id: &str| {
            bindings
                .and_then(|b| b.get(id))
                .is_some_and(|b| b.action != "none")
        };
        self.claims.retain(|claim| {
            if wanted(claim.id) {
                return true;
            }
            if let Some(value) = claim.original {
                out.messages.push(vec![0xb6, claim.cc, value]);
            }
            false
        });
        for (id, cc, baseline) in CONTROLS {
            if wanted(id) && !self.owns(cc) {
                self.claims.push(Claim {
                    id,
                    cc,
                    baseline,
                    original: None,
                    phase: Phase::Query,
                    since: now,
                });
                out.messages.push(vec![0xb7, cc, 0]);
            }
        }
        for claim in &mut self.claims {
            if matches!(claim.phase, Phase::Query | Phase::Reset) && now - claim.since > 2. {
                claim.phase = Phase::Failed;
                out.warning = true;
                if let Some(value) = claim.original {
                    out.messages.push(vec![0xb6, claim.cc, value]);
                }
            }
        }
        out
    }
    /// Only a confirmed baseline -> active transition is an action. Query/reset replies
    /// carry no source identifier, so they must never be treated as physical presses.
    pub fn receive(&mut self, source: &str, bytes: &[u8], now: f32) -> Update {
        let mut out = Update::default();
        if source != "daw" || bytes.len() != 3 || bytes[0] != 0xb6 {
            return out;
        }
        let Some(c) = self.claims.iter_mut().find(|c| c.cc == bytes[1]) else {
            return out;
        };
        let value = bytes[2];
        match c.phase {
            Phase::Query => {
                c.original = Some(value);
                c.phase = Phase::Reset;
                c.since = now;
                out.messages.push(vec![0xb6, c.cc, c.baseline]);
            }
            Phase::Reset if value == c.baseline => c.phase = Phase::Armed,
            Phase::Armed if value != c.baseline && (c.cc != 29 || value == 14) => {
                c.phase = Phase::Reset;
                c.since = now;
                out.action = Some(c.id.into());
                out.messages.push(vec![0xb6, c.cc, c.baseline]);
            }
            _ => {}
        }
        out
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn initialization_and_echoes_never_execute_and_release_restores_the_original_state() {
        let bindings = BTreeMap::from([("scale".into(), Binding::new("lighting", "1"))]);
        let mut f = FeatureControls::default();
        assert_eq!(
            f.synchronize(Some(&bindings), 0.).messages,
            vec![vec![0xb7, 74, 0]]
        );
        assert_eq!(
            f.receive("daw", &[0xb6, 74, 127], 0.1).messages,
            vec![vec![0xb6, 74, 0]]
        );
        assert!(f.receive("daw", &[0xb6, 74, 127], 0.2).action.is_none());
        assert!(f.receive("daw", &[0xb6, 74, 0], 0.3).action.is_none());
        assert_eq!(
            f.receive("daw", &[0xb6, 74, 127], 0.4).action.as_deref(),
            Some("scale")
        );
        assert!(f.receive("daw", &[0xb6, 74, 127], 0.5).action.is_none());
        f.receive("daw", &[0xb6, 74, 0], 0.6);
        assert_eq!(f.synchronize(None, 0.7).messages, vec![vec![0xb6, 74, 127]]);
        assert!(!f.owns(74));
    }
    #[test]
    fn chord_mode_returns_to_daw_baseline_and_missing_confirmation_disarms() {
        let bindings = BTreeMap::from([("chordMap".into(), Binding::new("sound", "1"))]);
        let mut f = FeatureControls::default();
        f.synchronize(Some(&bindings), 0.);
        f.receive("daw", &[0xb6, 29, 2], 0.1);
        f.receive("daw", &[0xb6, 29, 2], 0.2);
        let change = f.receive("daw", &[0xb6, 29, 14], 0.3);
        assert_eq!(change.action.as_deref(), Some("chordMap"));
        assert_eq!(change.messages, vec![vec![0xb6, 29, 2]]);
        assert!(f.synchronize(Some(&bindings), 3.).warning);
        f.receive("daw", &[0xb6, 29, 2], 3.1);
        assert!(f.receive("daw", &[0xb6, 29, 14], 3.2).action.is_none());
    }
}
