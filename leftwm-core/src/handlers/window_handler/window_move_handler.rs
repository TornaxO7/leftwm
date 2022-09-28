use crate::{Manager, Window, Workspace};
use crate::config::Config;
use crate::display_servers::DisplayServer;
use crate::models::{Xyhw, WindowHandle};

impl<C, SERVER> Manager<C, SERVER> where C: Config, SERVER: DisplayServer {
    pub fn window_move_handler(
        &mut self,
        handle: &WindowHandle,
        offset_x: i32,
        offset_y: i32,
    ) -> bool {
        let disable_snap = &self.config.disable_window_snap();
        match self.state.windows.iter_mut().find(|w| w.handle == *handle) {
            Some(w) => {
                self.process_window(w, offset_x, offset_y);
                if !disable_snap && self.snap_to_workspace(w, &self.state.workspaces) {
                    self.state.sort_windows();
                }
                true
            }
            None => false,
        }
    }

    // Update the window for the workspace it is currently on.
    fn snap_to_workspace(&self, window: &mut Window, workspaces: &[Workspace]) -> bool {
        // Check that the workspace contains the window.
        let loc = window.calculated_xyhw();
        let (x, y) = loc.center();
    
        if let Some(workspace) = workspaces.iter().find(|ws| ws.contains_point(x, y)) {
            return self.should_snap(window, workspace, loc);
        }
        false
    }

    // To be snapable, the window must be inside the workspace AND the a side must be close to
    // the workspaces edge.
    fn should_snap(&self, window: &mut Window, workspace: &Workspace, loc: Xyhw) -> bool {
        if window.must_float() {
            return false;
        }
        // Get window sides.
        let win_left = loc.x();
        let win_right = win_left + window.width();
        let win_top = loc.y();
        let win_bottom = win_top + window.height();
        // Check for close edge.
        let dist = 10;
        let ws_left = workspace.x();
        let ws_right = workspace.x() + workspace.width();
        let ws_top = workspace.y();
        let ws_bottom = workspace.y() + workspace.height();
        if [
            win_top - ws_top,
            win_bottom - ws_bottom,
            win_left - ws_left,
            win_right - ws_right,
        ]
        .iter()
        .any(|x| x.abs() < dist)
        {
            return window.snap_to_workspace(workspace);
        }
        false
    }
}
