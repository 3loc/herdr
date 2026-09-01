//! Delivery of queued notes when an agent finishes a turn.

use std::time::Duration;

use bytes::Bytes;

use crate::app::actions::PaneStateUpdate;
use crate::app::App;
use crate::detect::AgentState;
use crate::layout::PaneId;

const QUEUED_NOTE_SUBMIT_DELAY: Duration = Duration::from_millis(300);

fn completes_a_turn(update: &PaneStateUpdate) -> bool {
    // Startup and agent acquisition can settle to idle without completing a turn.
    update.state == AgentState::Idle
        && update.previous_state != AgentState::Idle
        && !update.suppress_completion
}

impl App {
    pub(crate) fn flush_note_queues(&mut self, updates: &[PaneStateUpdate]) {
        for update in updates.iter().filter(|update| completes_a_turn(update)) {
            self.send_next_queued_note(update.ws_idx, update.pane_id);
        }
    }

    /// Send and remove the oldest queued note when the pane can accept it.
    pub(crate) fn send_next_queued_note(
        &mut self,
        ws_idx: usize,
        pane_id: PaneId,
    ) -> Option<String> {
        let terminal_id = self
            .state
            .workspaces
            .get(ws_idx)
            .and_then(|workspace| workspace.terminal_id(pane_id))
            .cloned()?;
        let terminal = self.state.terminals.get(&terminal_id)?;
        if !terminal.has_notes() {
            return None;
        }
        if terminal.state == AgentState::Blocked || terminal.managed_agent_launch_pending() {
            return None;
        }
        let expected_agent = terminal.effective_known_agent()?;
        let text = terminal.notes.first()?.clone();

        let runtime = self.lookup_runtime_sender(ws_idx, pane_id)?;
        if !crate::app::agents::runtime_hosts_agent(runtime, expected_agent) {
            return None;
        }

        let (encoded, enter) = crate::app::api_helpers::encode_api_submission_parts(runtime, &text);
        if runtime.try_send_bytes(Bytes::from(encoded)).is_err() {
            return None;
        }
        runtime.send_bytes_after(Bytes::from(enter), QUEUED_NOTE_SUBMIT_DELAY);

        let terminal = self.state.terminals.get_mut(&terminal_id)?;
        let sent = terminal.remove_note(0);
        self.state.mark_session_dirty();
        sent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::EffectivePresentation;
    use std::collections::HashMap;

    fn update(previous_state: AgentState, state: AgentState) -> PaneStateUpdate {
        PaneStateUpdate {
            pane_id: PaneId::alloc(),
            ws_idx: 0,
            previous_agent_label: None,
            previous_known_agent: None,
            previous_state,
            previous_seen: true,
            previous_presentation: EffectivePresentation {
                title: None,
                display_agent: None,
                state_labels: HashMap::new(),
            },
            agent_label: None,
            known_agent: None,
            state,
            seen: true,
            presentation: EffectivePresentation {
                title: None,
                display_agent: None,
                state_labels: HashMap::new(),
            },
            agent_name_changed: false,
            agent_released: false,
            agent_release_status: None,
            suppress_completion: false,
        }
    }

    #[test]
    fn finishing_a_turn_delivers() {
        assert!(completes_a_turn(&update(
            AgentState::Working,
            AgentState::Idle
        )));
        assert!(completes_a_turn(&update(
            AgentState::Blocked,
            AgentState::Idle
        )));
    }

    #[test]
    fn staying_idle_does_not_redeliver() {
        assert!(!completes_a_turn(&update(
            AgentState::Idle,
            AgentState::Idle
        )));
    }

    #[test]
    fn starting_work_does_not_deliver() {
        assert!(!completes_a_turn(&update(
            AgentState::Idle,
            AgentState::Working
        )));
    }

    #[test]
    fn suppressed_completion_does_not_deliver() {
        let mut suppressed = update(AgentState::Working, AgentState::Idle);
        suppressed.suppress_completion = true;
        assert!(!completes_a_turn(&suppressed));
    }
}
