//! `kern-fsm` — the mission finite-state machine.
//!
//! Missions move through an explicit graph of states. Illegal transitions are *rejected*
//! (they return an error), and terminal states have no outgoing edges. There is no path
//! that skips authorization: `Created -> Completed` is not a legal edge.

use core::fmt;

/// The lifecycle state of a mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissionState {
    /// Just created; nothing validated yet.
    Created,
    /// Being validated (inputs, identity, policy inputs).
    Validating,
    /// Validated and planned.
    Planned,
    /// Authorized to execute (a capability has been granted).
    Authorized,
    /// Executing governed effects.
    Executing,
    /// Verifying the effects produced.
    Verifying,
    /// Finished successfully.
    Completed,
    /// Refused by policy/capability.
    Denied,
    /// Failed during execution or verification.
    Failed,
    /// Cancelled by the operator.
    Cancelled,
    /// Exceeded its deadline.
    TimedOut,
    /// Effects were rolled back.
    RolledBack,
    /// Blocked by a real, external constraint.
    Blocked,
}

impl fmt::Display for MissionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl MissionState {
    /// True when no further transition is possible.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            MissionState::Completed
                | MissionState::Denied
                | MissionState::Failed
                | MissionState::Cancelled
                | MissionState::TimedOut
                | MissionState::RolledBack
                | MissionState::Blocked
        )
    }

    /// Whether `self -> to` is a legal edge in the mission graph.
    #[must_use]
    pub fn can_transition_to(self, to: MissionState) -> bool {
        use MissionState::{
            Authorized, Blocked, Cancelled, Completed, Created, Denied, Executing, Failed, Planned,
            RolledBack, TimedOut, Validating, Verifying,
        };
        matches!(
            (self, to),
            (Created, Validating | Denied | Cancelled)
                | (Validating, Planned | Denied | Failed | Cancelled | Blocked)
                | (Planned, Authorized | Denied | Cancelled | Blocked)
                | (Authorized, Executing | Cancelled | TimedOut)
                | (
                    Executing,
                    Verifying | Failed | TimedOut | Cancelled | RolledBack | Blocked
                )
                | (Verifying, Completed | Failed | RolledBack | Blocked)
        )
    }
}

/// Why a transition was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionError {
    /// The edge `from -> to` is not part of the mission graph.
    Illegal {
        /// Current state.
        from: MissionState,
        /// Attempted next state.
        to: MissionState,
    },
    /// The machine is already in a terminal state.
    FromTerminal {
        /// The terminal state.
        state: MissionState,
    },
}

impl fmt::Display for TransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransitionError::Illegal { from, to } => {
                write!(f, "illegal transition {from} -> {to}")
            }
            TransitionError::FromTerminal { state } => {
                write!(f, "cannot transition from terminal state {state}")
            }
        }
    }
}

impl std::error::Error for TransitionError {}

/// A mission state machine that only permits legal transitions.
#[derive(Debug, Clone)]
pub struct MissionMachine {
    state: MissionState,
    history: Vec<MissionState>,
}

impl Default for MissionMachine {
    fn default() -> Self {
        Self {
            state: MissionState::Created,
            history: vec![MissionState::Created],
        }
    }
}

impl MissionMachine {
    /// A fresh machine in `Created`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The current state.
    #[must_use]
    pub fn state(&self) -> MissionState {
        self.state
    }

    /// The ordered history of visited states.
    #[must_use]
    pub fn history(&self) -> &[MissionState] {
        &self.history
    }

    /// Attempt a transition. On success the machine advances and records history; on
    /// failure the machine is unchanged.
    pub fn transition(&mut self, to: MissionState) -> Result<MissionState, TransitionError> {
        if self.state.is_terminal() {
            return Err(TransitionError::FromTerminal { state: self.state });
        }
        if !self.state.can_transition_to(to) {
            return Err(TransitionError::Illegal {
                from: self.state,
                to,
            });
        }
        self.state = to;
        self.history.push(to);
        Ok(to)
    }
}

#[cfg(test)]
mod tests {
    use super::MissionState::{
        Authorized, Completed, Created, Executing, Planned, Validating, Verifying,
    };
    use super::*;

    #[test]
    fn created_to_completed_is_denied() {
        let mut m = MissionMachine::new();
        let err = m.transition(Completed).unwrap_err();
        assert_eq!(
            err,
            TransitionError::Illegal {
                from: Created,
                to: Completed
            }
        );
        // rejected transition left the machine untouched
        assert_eq!(m.state(), Created);
    }

    #[test]
    fn happy_path_reaches_completed() {
        let mut m = MissionMachine::new();
        for s in [
            Validating, Planned, Authorized, Executing, Verifying, Completed,
        ] {
            m.transition(s).expect("legal edge");
        }
        assert_eq!(m.state(), Completed);
        assert!(m.state().is_terminal());
        assert_eq!(m.history().len(), 7);
    }

    #[test]
    fn cannot_leave_terminal_state() {
        let mut m = MissionMachine::new();
        m.transition(MissionState::Denied).unwrap();
        let err = m.transition(Validating).unwrap_err();
        assert_eq!(
            err,
            TransitionError::FromTerminal {
                state: MissionState::Denied
            }
        );
    }

    #[test]
    fn skipping_authorization_is_illegal() {
        // Planned -> Executing must not exist (must pass through Authorized).
        assert!(!Planned.can_transition_to(Executing));
        assert!(Planned.can_transition_to(Authorized));
        assert!(Authorized.can_transition_to(Executing));
    }

    #[test]
    fn terminal_states_have_no_out_edges() {
        let all = [
            Created,
            Validating,
            Planned,
            Authorized,
            Executing,
            Verifying,
            Completed,
            MissionState::Denied,
            MissionState::Failed,
            MissionState::Cancelled,
            MissionState::TimedOut,
            MissionState::RolledBack,
            MissionState::Blocked,
        ];
        for from in all {
            for to in all {
                if from.is_terminal() {
                    assert!(
                        !from.can_transition_to(to),
                        "{from} is terminal but has edge to {to}"
                    );
                }
            }
        }
    }
}
