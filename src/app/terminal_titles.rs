use std::collections::HashSet;

use super::App;
use crate::layout::PaneId;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct TerminalTitleChanges {
    pub(crate) raw_changed: bool,
    pub(crate) stripped_changed: bool,
}

impl App {
    pub(crate) fn terminal_title_sidebar_changed(&self, changes: &TerminalTitleChanges) -> bool {
        let config = &self.state.sidebar_agents;
        std::iter::once(&config.rows)
            .chain(config.rows_by_agent.values())
            .flatten()
            .flatten()
            .any(|token| match token.parts().0 {
                crate::config::AgentSidebarToken::TerminalTitle => changes.raw_changed,
                crate::config::AgentSidebarToken::TerminalTitleStripped => changes.stripped_changed,
                _ => false,
            })
    }

    pub(crate) fn sync_pending_terminal_titles(&mut self) -> TerminalTitleChanges {
        let sources = self.render_dirty.pending_terminal_title_sources();
        let changes = self.sync_terminal_titles(&sources);
        if self.terminal_title_sidebar_changed(&changes) {
            self.render_dirty.request_generic();
            self.render_notify.notify_one();
        }
        changes
    }

    pub(crate) fn terminal_title_projection_snapshot(
        &self,
        agents: Option<&[crate::detect::Agent]>,
    ) -> std::collections::HashMap<crate::terminal::TerminalId, Option<String>> {
        self.state
            .terminals
            .iter()
            .filter_map(|(terminal_id, terminal)| {
                let included = agents.is_none_or(|agents| {
                    terminal
                        .effective_known_agent()
                        .is_some_and(|agent| agents.contains(&agent))
                });
                included.then(|| (terminal_id.clone(), terminal.terminal_title_stripped()))
            })
            .collect()
    }

    pub(crate) fn reconcile_terminal_titles_after_manifest_reload(
        &mut self,
        previous: &std::collections::HashMap<crate::terminal::TerminalId, Option<String>>,
    ) -> TerminalTitleChanges {
        let mut changes = TerminalTitleChanges::default();
        let mut changed_terminals = HashSet::new();
        for (terminal_id, terminal) in &mut self.state.terminals {
            let Some(previous_stripped) = previous.get(terminal_id) else {
                continue;
            };
            if terminal.reconcile_terminal_title_projection(previous_stripped.clone()) {
                changes.stripped_changed = true;
                changed_terminals.insert(terminal_id.clone());
            }
        }
        if changed_terminals.is_empty() {
            return changes;
        }

        let mut publish = Vec::new();
        for (ws_idx, workspace) in self.state.workspaces.iter().enumerate() {
            for tab in &workspace.tabs {
                for (pane_id, pane) in &tab.panes {
                    if changed_terminals.contains(&pane.attached_terminal_id) {
                        publish.push((ws_idx, *pane_id));
                    }
                }
            }
        }
        for (ws_idx, pane_id) in publish {
            self.emit_pane_updated(ws_idx, pane_id);
        }
        if self.terminal_title_sidebar_changed(&changes) {
            self.render_dirty.request_generic();
            self.render_notify.notify_one();
        }
        changes
    }

    pub(crate) fn sync_terminal_titles(
        &mut self,
        sources: &HashSet<PaneId>,
    ) -> TerminalTitleChanges {
        if sources.is_empty() {
            return TerminalTitleChanges::default();
        }

        let mut observations = Vec::with_capacity(sources.len());
        for pane_id in sources {
            let Some((ws_idx, terminal_id)) = self
                .find_pane(*pane_id)
                .map(|(ws_idx, pane)| (ws_idx, pane.attached_terminal_id.clone()))
            else {
                continue;
            };
            let Some(runtime) = self.terminal_runtimes.get(&terminal_id) else {
                continue;
            };
            observations.push((ws_idx, *pane_id, terminal_id, runtime.terminal_title()));
        }

        let mut changes = TerminalTitleChanges::default();
        let mut publish = Vec::new();
        for (ws_idx, pane_id, terminal_id, title) in observations {
            let Some(terminal) = self.state.terminals.get_mut(&terminal_id) else {
                continue;
            };
            let change = terminal.set_terminal_title(title);
            changes.raw_changed |= change.raw_changed;
            changes.stripped_changed |= change.stripped_changed;
            if change.stripped_changed {
                publish.push((ws_idx, pane_id));
            }
        }

        for (ws_idx, pane_id) in publish {
            self.emit_pane_updated(ws_idx, pane_id);
        }

        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::detect::{Agent, AgentState};
    use crate::workspace::Workspace;

    #[tokio::test]
    async fn sync_keeps_latest_raw_title_and_emits_only_for_stripped_changes() {
        let _guard = crate::config::test_config_env_lock().lock().unwrap();
        crate::detect::manifest::reload_manifests();
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub.clone());
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.ensure_test_terminals();
        let pane_id = app.state.workspaces[0].tabs[0].root_pane;
        let terminal_id = app.state.workspaces[0].tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        let terminal = app.state.terminals.get_mut(&terminal_id).unwrap();
        terminal.detected_agent = Some(Agent::Claude);
        terminal.state = AgentState::Working;
        let runtime = crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"");
        runtime.test_process_pty_bytes("\x1b]0;◐ 修复🙂标题\x07".as_bytes());
        app.terminal_runtimes.insert(terminal_id.clone(), runtime);
        let sources = HashSet::from([pane_id]);

        assert_eq!(
            app.sync_terminal_titles(&sources),
            TerminalTitleChanges {
                raw_changed: true,
                stripped_changed: true,
            }
        );
        let pane = app.pane_info(0, pane_id).unwrap();
        assert_eq!(pane.terminal_title.as_deref(), Some("◐ 修复🙂标题"));
        assert_eq!(pane.terminal_title_stripped.as_deref(), Some("修复🙂标题"));
        assert_eq!(pane.title, None);
        assert_eq!(pane.agent_status, crate::api::schema::AgentStatus::Working);
        assert_eq!(pane.revision, 1);
        let agent = app.collect_agent_infos().pop().unwrap();
        assert_eq!(agent.terminal_title.as_deref(), Some("◐ 修复🙂标题"));
        assert_eq!(agent.terminal_title_stripped.as_deref(), Some("修复🙂标题"));

        app.terminal_runtimes
            .get(&terminal_id)
            .unwrap()
            .test_process_pty_bytes("\x1b]2;◓ 修复🙂标题\x1b\\".as_bytes());
        assert_eq!(
            app.sync_terminal_titles(&sources),
            TerminalTitleChanges {
                raw_changed: true,
                stripped_changed: false,
            }
        );
        let pane = app.pane_info(0, pane_id).unwrap();
        assert_eq!(pane.terminal_title.as_deref(), Some("◓ 修复🙂标题"));
        assert_eq!(pane.terminal_title_stripped.as_deref(), Some("修复🙂标题"));
        assert_eq!(pane.revision, 1);
        assert_eq!(pane_updated_events(&event_hub), 1);

        app.terminal_runtimes
            .get(&terminal_id)
            .unwrap()
            .test_process_pty_bytes(b"\x1b]0;Done reviewing\x07");
        assert!(app.sync_terminal_titles(&sources).stripped_changed);
        assert_eq!(pane_updated_events(&event_hub), 2);

        app.terminal_runtimes
            .get(&terminal_id)
            .unwrap()
            .test_process_pty_bytes(b"\x1b]0;\x07");
        assert!(app.sync_terminal_titles(&sources).stripped_changed);
        let pane = app.pane_info(0, pane_id).unwrap();
        assert_eq!(pane.terminal_title, None);
        assert_eq!(pane.terminal_title_stripped, None);
        assert_eq!(pane.revision, 3);
        assert_eq!(pane_updated_events(&event_hub), 3);
    }

    #[tokio::test]
    async fn agent_identity_reconciles_existing_title_projection() {
        let _guard = crate::config::test_config_env_lock().lock().unwrap();
        crate::detect::manifest::reload_manifests();
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub.clone());
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.ensure_test_terminals();
        let pane_id = app.state.workspaces[0].tabs[0].root_pane;
        let terminal_id = app.state.workspaces[0].tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        let terminal = app.state.terminals.get_mut(&terminal_id).unwrap();
        terminal.set_terminal_title(Some("◐ task".into()));
        let revision = terminal.revision;
        assert_eq!(
            terminal.terminal_title_stripped().as_deref(),
            Some("◐ task")
        );

        app.handle_internal_event(crate::events::AppEvent::AgentProcessDetected {
            pane_id,
            agent: Agent::Claude,
            observed_at: std::time::Instant::now(),
        });

        let terminal = app.state.terminals.get(&terminal_id).unwrap();
        assert_eq!(terminal.terminal_title.as_deref(), Some("◐ task"));
        assert_eq!(terminal.terminal_title_stripped().as_deref(), Some("task"));
        assert_eq!(terminal.revision, revision + 1);
        assert_eq!(pane_updated_events(&event_hub), 1);
    }

    #[tokio::test]
    async fn manifest_reload_reconciles_existing_title_projection() {
        const CHILD_ENV: &str = "HERDR_TEST_TITLE_MANIFEST_RELOAD_CHILD";
        if std::env::var_os(CHILD_ENV).is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "app::terminal_titles::tests::manifest_reload_reconciles_existing_title_projection",
                    "--nocapture",
                ])
                .env(CHILD_ENV, "1")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "isolated manifest reload test failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let _guard = crate::config::test_config_env_lock().lock().unwrap();
        let old_config = std::env::var_os("XDG_CONFIG_HOME");
        let old_state = std::env::var_os("XDG_STATE_HOME");
        let base = std::env::temp_dir().join(format!(
            "herdr-title-manifest-reload-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::env::set_var("XDG_CONFIG_HOME", base.join("config"));
        std::env::set_var("XDG_STATE_HOME", base.join("state"));
        crate::detect::manifest::reload_manifests();

        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub.clone());
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.ensure_test_terminals();
        let pane_id = app.state.workspaces[0].tabs[0].root_pane;
        let terminal_id = app.state.workspaces[0].tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        let terminal = app.state.terminals.get_mut(&terminal_id).unwrap();
        terminal.detected_agent = Some(Agent::Claude);
        terminal.set_terminal_title(Some("◆ task".into()));
        let revision = terminal.revision;
        assert_eq!(
            terminal.terminal_title_stripped().as_deref(),
            Some("◆ task")
        );

        let override_path = base.join("config/herdr-dev/agent-detection/claude.toml");
        std::fs::create_dir_all(override_path.parent().unwrap()).unwrap();
        std::fs::write(
            &override_path,
            r#"
id = "claude"
min_engine_version = 4
terminal_title_activity_regex = '^◆$'

[[rules]]
id = "idle"
state = "idle"
contains = ["ready"]
"#,
        )
        .unwrap();

        let response = app.handle_api_request(crate::api::schema::Request {
            id: "reload-title-manifest".into(),
            method: crate::api::schema::Method::ServerReloadAgentManifests(
                crate::api::schema::EmptyParams::default(),
            ),
        });
        let response: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(response["result"]["type"], "agent_manifest_reload");
        let terminal = app.state.terminals.get(&terminal_id).unwrap();
        assert_eq!(terminal.terminal_title.as_deref(), Some("◆ task"));
        assert_eq!(terminal.terminal_title_stripped().as_deref(), Some("task"));
        assert_eq!(terminal.revision, revision + 1);
        assert_eq!(pane_updated_events(&event_hub), 1);

        match old_config {
            Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_state {
            Some(value) => std::env::set_var("XDG_STATE_HOME", value),
            None => std::env::remove_var("XDG_STATE_HOME"),
        }
        crate::detect::manifest::reload_manifests();
        let _ = std::fs::remove_dir_all(base);
    }

    #[tokio::test]
    async fn syncing_pending_titles_preserves_sidebar_render_impact() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub);
        app.state.workspaces = vec![Workspace::test_new("one")];
        app.state.active = Some(0);
        app.state.ensure_test_terminals();
        app.state.sidebar_agents.rows = vec![vec![
            crate::config::AgentSidebarToken::TerminalTitleStripped,
        ]];
        let pane_id = app.state.workspaces[0].tabs[0].root_pane;
        let terminal_id = app.state.workspaces[0]
            .terminal_id(pane_id)
            .unwrap()
            .clone();
        let runtime = crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"");
        runtime.test_process_pty_bytes(b"\x1b]0;building\x07");
        app.terminal_runtimes.insert(terminal_id, runtime);
        app.render_dirty.request_terminal_title(pane_id);

        let changes = app.sync_pending_terminal_titles();

        assert!(changes.stripped_changed);
        let render_request = app.render_dirty.take();
        assert!(render_request.generic);
    }

    #[test]
    fn sidebar_redraws_only_for_the_configured_title_form() {
        let event_hub = crate::api::EventHub::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(&Config::default(), true, None, api_rx, event_hub);
        app.state.sidebar_agents.rows = vec![vec![crate::config::AgentSidebarToken::Agent]];
        app.state.sidebar_agents.rows_by_agent.insert(
            "claude".into(),
            vec![vec![
                crate::config::AgentSidebarToken::TerminalTitleStripped,
            ]],
        );

        let spinner_only = TerminalTitleChanges {
            raw_changed: true,
            ..TerminalTitleChanges::default()
        };
        assert!(!app.terminal_title_sidebar_changed(&spinner_only));
        assert!(app.terminal_title_sidebar_changed(&TerminalTitleChanges {
            stripped_changed: true,
            ..TerminalTitleChanges::default()
        }));

        app.state.sidebar_agents.rows_by_agent.insert(
            "claude".into(),
            vec![vec![crate::config::AgentSidebarToken::TerminalTitle]],
        );
        assert!(app.terminal_title_sidebar_changed(&spinner_only));
    }

    fn pane_updated_events(event_hub: &crate::api::EventHub) -> usize {
        event_hub
            .events_after(0)
            .iter()
            .filter(|(_, event)| event.event == crate::api::schema::EventKind::PaneUpdated)
            .count()
    }
}
