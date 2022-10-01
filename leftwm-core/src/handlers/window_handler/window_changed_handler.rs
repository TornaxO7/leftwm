use crate::{Config, DisplayServer, Manager, models::{WindowChange, WindowState, WindowType}};


impl<C: Config, SERVER: DisplayServer> Manager<C, SERVER> {
    pub fn window_changed_handler(&mut self, change: WindowChange) -> bool {
        let mut changed = false;
        let mut fullscreen_changed = false;
        let strut_changed = change.strut.is_some();
        let windows = self.state.windows.clone();

        if let Some(window) = windows
            .iter()
            .find(|w| w.handle == change.handle)
        {
            if let Some(ref states) = change.states {
                let change_contains = states.contains(&WindowState::Fullscreen);
                fullscreen_changed = change_contains || window.is_fullscreen();
            }
            let container = match self.find_transient_parent(&windows, window.transient) {
                Some(parent) => Some(parent.exact_xyhw()),
                None if window.r#type == WindowType::Dialog => self
                    .state
                    .workspaces
                    .iter()
                    .find(|ws| ws.tag == window.tag)
                    .map(|ws| ws.xyhw),
                _ => None,
            };

            changed = change.update(window, container);
            if window.r#type == WindowType::Dock {
                self.update_workspace_avoid_list();
                // Don't let changes from docks re-render the worker. This will result in an
                // infinite loop. Just be patient a rerender will occur.
            }
        }
        if fullscreen_changed {
            // Update `dock` windows once, so they can recieve mouse click events again.
            // This is necessary, since we exclude them from the general update loop above.
            if let Some(windows) = self
                .state
                .windows
                .iter()
                .find(|w| w.r#type == WindowType::Dock)
            {
                self.display_server.update_windows(vec![windows]);
            }

            // Reorder windows.
            self.state.sort_windows();
        }
        if strut_changed {
            self.state.update_static();
        }
        changed
    }
}
