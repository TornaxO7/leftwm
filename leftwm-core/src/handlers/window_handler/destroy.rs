use crate::{Config, DisplayServer, Manager, models::{WindowHandle, WindowType}, Window, utils::helpers, State, DisplayAction};

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
        update_workspace_avoid_list(&mut self.state);

        let focused = self.state.focus_manager.window_history.get(0);
        // Make sure focus is recalculated if we closed the currently focused window
        if focused == Some(&Some(*handle)) {
            if self.state.focus_manager.behaviour.is_sloppy()
                && self.state.focus_manager.sloppy_mouse_follows_focus
            {
                let act = DisplayAction::FocusWindowUnderCursor;
                self.state.actions.push_back(act);
            } else if let Some(parent) =
                find_transient_parent(&self.state.windows, transient).map(|p| p.handle)
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

    /// Find the next or previous window on the currently focused workspace.
    /// May return `None` if no other window is present.
    pub fn get_next_or_previous_handle(&mut self, handle: &WindowHandle) -> Option<WindowHandle> {
        let focused_workspace = self.state.focus_manager.workspace(&self.state.workspaces)?;
        let on_focused_workspace = |x: &Window| -> bool { focused_workspace.is_managed(x) };
        let mut windows_on_workspace =
            helpers::vec_extract(&mut self.state.windows, on_focused_workspace);
        let is_handle = |x: &Window| -> bool { &x.handle == handle };
        let new_handle = helpers::relative_find(&windows_on_workspace, is_handle, 1, false)
            .or_else(|| helpers::relative_find(&windows_on_workspace, is_handle, -1, false))
            .map(|w| w.handle);
        self.state.windows.append(&mut windows_on_workspace);
        new_handle
    }
}

fn update_workspace_avoid_list(state: &mut State) {
    let mut avoid = vec![];
    state
        .windows
        .iter()
        .filter(|w| w.r#type == WindowType::Dock)
        .filter_map(|w| w.strut.map(|strut| (w.handle, strut)))
        .for_each(|(handle, to_avoid)| {
            tracing::debug!("AVOID STRUT:[{:?}] {:?}", handle, to_avoid);
            avoid.push(to_avoid);
        });
    for ws in &mut state.workspaces {
        let struts = avoid
            .clone()
            .into_iter()
            .filter(|s| {
                let (x, y) = s.center();
                ws.contains_point(x, y)
            })
            .collect();
        ws.avoid = struts;
        ws.update_avoided_areas();
    }
}

fn find_transient_parent(windows: &[Window], transient: Option<WindowHandle>) -> Option<&Window> {
    let mut transient = transient?;
    loop {
        transient = if let Some(found) = windows
            .iter()
            .find(|x| x.handle == transient)
            .and_then(|x| x.transient)
        {
            found
        } else {
            return windows.iter().find(|x| x.handle == transient);
        };
    }
}
