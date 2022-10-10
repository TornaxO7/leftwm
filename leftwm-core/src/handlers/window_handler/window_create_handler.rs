use crate::{
    child_process::exec_shell,
    layouts::Layout,
    models::{WindowState, WindowType, Xyhw},
    utils::helpers,
    Config, DisplayAction, DisplayServer, Manager, Window, Workspace, config::InsertBehavior,
};
use std::env;
use std::str::FromStr;

impl<C: Config, SERVER: DisplayServer> Manager<C, SERVER> {
    /// Process a collection of events, and apply them changes to a manager.
    /// Returns true if changes need to be rendered.
    pub fn window_created_handler(&mut self, mut window: Window, x: i32, y: i32) -> bool {
        // Setup any predifined hooks.
        self.config.setup_predefined_window(&mut window);
        let mut is_first = false;
        let mut on_same_tag = true;
        // Random value
        let mut layout: Layout = Layout::MainAndVertStack;
        self.setup_window(
            &mut window,
            (x, y),
            &mut layout,
            &mut is_first,
            &mut on_same_tag,
        );
        self.config.load_window(&mut window);
        self.insert_window(&mut window, layout);

        let follow_mouse = self.state.focus_manager.focus_new_windows
            && self.state.focus_manager.behaviour.is_sloppy()
            && self.state.focus_manager.sloppy_mouse_follows_focus
            && on_same_tag;
        // Let the DS know we are managing this window.
        let act = DisplayAction::AddedWindow(window.handle, window.floating(), follow_mouse);
        self.state.actions.push_back(act);

        // Let the DS know the correct desktop to find this window.
        if window.tag.is_some() {
            let act = DisplayAction::SetWindowTag(window.handle, window.tag);
            self.state.actions.push_back(act);
        }

        // Tell the WM the new display order of the windows.
        self.state.sort_windows(); // Is this needed??

        if (self.state.focus_manager.focus_new_windows || is_first) && on_same_tag {
            self.state.focus_window(&window.handle);
        }

        if let Some(cmd) = &self.config.on_new_window_cmd() {
            exec_shell(cmd, &mut self.children);
        }

        true
    }

    fn insert_window(&mut self, window: &mut Window, layout: Layout) {
        let mut was_fullscreen = false;
        if window.r#type == WindowType::Normal {
            let for_active_workspace =
                |x: &Window| -> bool { window.tag == x.tag && x.is_managed() };
            // Only minimize when the new window is type normal.
            if let Some(fsw) = self
                .state
                .windows
                .iter_mut()
                .find(|w| for_active_workspace(w) && w.is_fullscreen())
            {
                let act = DisplayAction::SetState(
                    fsw.handle,
                    !fsw.is_fullscreen(),
                    WindowState::Fullscreen,
                );
                self.state.actions.push_back(act);
                was_fullscreen = true;
            }
            if matches!(layout, Layout::Monocle | Layout::MainAndDeck) {
                // Extract the current windows on the same workspace.
                let mut to_reorder =
                    helpers::vec_extract(&mut self.state.windows, for_active_workspace);
                if layout == Layout::Monocle || to_reorder.is_empty() {
                    // When in monocle we want the new window to be fullscreen if the previous window was
                    // fullscreen.
                    if was_fullscreen {
                        let act = DisplayAction::SetState(
                            window.handle,
                            !window.is_fullscreen(),
                            WindowState::Fullscreen,
                        );
                        self.state.actions.push_back(act);
                    }
                    // Place the window above the other windows on the workspace.
                    to_reorder.insert(0, window.clone());
                } else {
                    // Place the window second within the other windows on the workspace.
                    to_reorder.insert(1, window.clone());
                }
                self.state.windows.append(&mut to_reorder);
                return;
            }
        }

        // If a window is a dialog, splash, or scractchpad we want it to be at the top.
        if window.r#type == WindowType::Dialog
            || window.r#type == WindowType::Splash
            || window.r#type == WindowType::Utility
            || self.is_scratchpad(window)
        {
            self.state.windows.insert(0, window.clone());
            return;
        }

        let current_index = self.state
            .focus_manager
            .window(&self.state.windows)
            .and_then(|current| {
                self.state
                    .windows
                    .iter()
                    .position(|w| w.handle == current.handle)
            })
            .unwrap_or(0);

        // Past special cases we just insert the window based on the configured insert behavior
        match self.state.insert_behavior {
            InsertBehavior::Top => self.state.windows.insert(0, window.clone()),
            InsertBehavior::Bottom => self.state.windows.push(window.clone()),
            InsertBehavior::AfterCurrent if current_index < self.state.windows.len() => {
                self.state.windows.insert(current_index + 1, window.clone());
            }
            InsertBehavior::AfterCurrent | InsertBehavior::BeforeCurrent => {
                self.state.windows.insert(current_index, window.clone());
            }
        }
    }

    fn setup_window(
        &mut self,
        window: &mut Window,
        xy: (i32, i32),
        layout: &mut Layout,
        is_first: &mut bool,
        on_same_tag: &mut bool,
    ) {
        // When adding a window we add to the workspace under the cursor, This isn't necessarily the
        // focused workspace. If the workspace is empty, it might not have received focus. This is so
        // the workspace that has windows on its is still active not the empty workspace.
        let ws: Option<&Workspace> = self
            .state
            .workspaces
            .iter()
            .find(|ws| {
                ws.xyhw.contains_point(xy.0, xy.1) && self.state.focus_manager.behaviour.is_sloppy()
            })
            .or_else(|| self.state.focus_manager.workspace(&self.state.workspaces)); // Backup plan.

        if let Some(ws) = ws {
            // Setup basic variables.
            let for_active_workspace = |x: &Window| -> bool { ws.tag == x.tag && x.is_managed() };
            *is_first = !self.state.windows.iter().any(|w| for_active_workspace(w));
            // May have been set by a predefined tag.
            if window.tag.is_none() {
                window.tag = self
                    .find_terminal(window.pid)
                    .map_or_else(|| ws.tag, |terminal| terminal.tag);
            }
            *on_same_tag = ws.tag == window.tag;
            *layout = ws.layout;

            // Setup a scratchpad window.
            if let Some((scratchpad_name, _)) = self
                .state
                .active_scratchpads
                .iter()
                .find(|(_, id)| id.iter().any(|id| Some(*id) == window.pid))
            {
                window.set_floating(true);
                if let Some(s) = self
                    .state
                    .scratchpads
                    .iter()
                    .find(|s| *scratchpad_name == s.name)
                {
                    let new_float_exact = s.xyhw(&ws.xyhw);
                    window.normal = ws.xyhw;
                    window.set_floating_exact(new_float_exact);
                    return;
                }
            }

            // Setup a child window.
            if let Some(parent) = self.find_transient_parent(&self.state.windows, window.transient)
            {
                // This is currently for vlc, this probably will need to be more general if another
                // case comes up where we don't want to move the window.
                if window.r#type != WindowType::Utility {
                    self.set_relative_floating(window, ws, parent.exact_xyhw());
                    return;
                }
            }

            // Setup window based on type.
            match window.r#type {
                WindowType::Normal => {
                    window.apply_margin_multiplier(ws.margin_multiplier);
                    if window.floating() {
                        self.set_relative_floating(window, ws, ws.xyhw);
                    }
                }
                WindowType::Dialog => {
                    if window.can_resize() {
                        window.set_floating(true);
                        let new_float_exact = ws.center_halfed();
                        window.normal = ws.xyhw;
                        window.set_floating_exact(new_float_exact);
                    } else {
                        self.set_relative_floating(window, ws, ws.xyhw);
                    }
                }
                WindowType::Splash => self.set_relative_floating(window, ws, ws.xyhw),
                _ => {}
            }
            return;
        }

        // Tell the WM to reevaluate the stacking order, so the new window is put in the correct layer
        self.state.sort_windows();

        // Setup a window is workspace is `None`. This shouldn't really happen.
        window.tag = Some(1);
        if self.is_scratchpad(window) {
            if let Some(scratchpad_tag) = self.state.tags.get_hidden_by_label("NSP") {
                window.tag(&scratchpad_tag.id);
                window.set_floating(true);
            }
        }
    }

    fn find_terminal(&self, pid: Option<u32>) -> Option<&Window> {
        // Get $SHELL, e.g. /bin/zsh
        let shell_path = env::var("SHELL").ok()?;
        // Remove /bin/
        let shell = shell_path.split('/').last()?;
        // Try and find the shell that launched this app, if such a thing exists.
        let is_terminal = |pid: u32| -> Option<bool> {
            let parent = std::fs::read(format!("/proc/{}/comm", pid)).ok()?;
            let parent_bytes = parent.split(|&c| c == b' ').next()?;
            let parent_str = std::str::from_utf8(parent_bytes).ok()?.strip_suffix('\n')?;
            Some(parent_str == shell)
        };

        let get_parent = |pid: u32| -> Option<u32> {
            let stat = std::fs::read(format!("/proc/{}/stat", pid)).ok()?;
            let ppid_bytes = stat.split(|&c| c == b' ').nth(3)?;
            let ppid_str = std::str::from_utf8(ppid_bytes).ok()?;
            let ppid_u32 = u32::from_str(ppid_str).ok()?;
            Some(ppid_u32)
        };

        let pid = pid?;
        let shell_id = get_parent(pid)?;
        if is_terminal(shell_id)? {
            let terminal = get_parent(shell_id)?;
            return self.state.windows.iter().find(|w| w.pid == Some(terminal));
        }

        None
    }

    fn set_relative_floating(&self, window: &mut Window, ws: &Workspace, outer: Xyhw) {
        window.set_floating(true);
        window.normal = ws.xyhw;
        let xyhw = window.requested.map_or_else(
            || ws.center_halfed(),
            |mut requested| {
                if ws.xyhw.contains_xyhw(&requested) {
                    requested
                } else {
                    requested.center_relative(outer, window.border);
                    if ws.xyhw.contains_xyhw(&requested) {
                        requested
                    } else {
                        ws.center_halfed()
                    }
                }
            },
        );
        window.set_floating_exact(xyhw);
    }
}
