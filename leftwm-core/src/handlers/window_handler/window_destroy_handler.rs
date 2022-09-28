use crate::{
    models::WindowHandle,
    Config, DisplayAction, DisplayServer, Manager,
};

impl<C: Config, SERVER: DisplayServer> Manager<C, SERVER> {
    /// Process a collection of events, and apply them changes to a manager.
    /// Returns true if changes need to be rendered.
    pub fn window_destroyed_handler(&mut self, handle: &WindowHandle) -> bool {
        // Find the next or previous window on the workspace.
        let new_handle = self.get_next_or_previous_handle(handle);
        // If there is a parent we would want to focus it.
        let (transient, floating, visible) =
            match self.state.windows.iter().find(|w| &w.handle == handle) {
                Some(window) => (window.transient, window.floating(), window.visible()),
                None => return false,
            };
        self.state
            .focus_manager
            .tags_last_window
            .retain(|_, h| h != handle);
        self.state.windows.retain(|w| &w.handle != handle);

        // Make sure the workspaces do not draw on the docks.
        self.update_workspace_avoid_list();

        let focused = self.state.focus_manager.window_history.get(0);
        // Make sure focus is recalculated if we closed the currently focused window
        if focused == Some(&Some(*handle)) {
            if self.state.focus_manager.behaviour.is_sloppy()
                && self.state.focus_manager.sloppy_mouse_follows_focus
            {
                let act = DisplayAction::FocusWindowUnderCursor;
                self.state.actions.push_back(act);
            } else if let Some(parent) = self
                .find_transient_parent(&self.state.windows, transient)
                .map(|p| p.handle)
            {
                self.state.focus_window(&parent);
            } else if let Some(handle) = new_handle {
                self.state.focus_window(&handle);
            } else {
                let act = DisplayAction::Unfocus(Some(*handle), floating);
                self.state.actions.push_back(act);
                self.state.focus_manager.window_history.push_front(None);
            }
        }

        // Only update windows if this window is visible.
        visible
    }
}
