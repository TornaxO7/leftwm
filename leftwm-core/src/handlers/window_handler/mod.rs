use crate::{
    models::{WindowHandle, WindowType},
    utils::helpers,
    Config, DisplayServer, Manager, Window,
};

mod window_changed_handler;
mod window_create_handler;
mod window_destroy_handler;
mod window_move_handler;
mod window_resize_handler;

impl<C: Config, SERVER: DisplayServer> Manager<C, SERVER> {
    fn find_transient_parent(
        &self,
        windows: &[Window],
        transient: Option<WindowHandle>,
    ) -> Option<&Window> {
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

    fn is_scratchpad(&mut self, window: &Window) -> bool {
        self.state
            .active_scratchpads
            .iter()
            .any(|(_, id)| id.iter().any(|id| window.pid == Some(*id)))
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

    fn process_window(&self, window: &mut Window, offset_x: i32, offset_y: i32) {
        let mut offset = window.get_floating_offsets().unwrap_or_default();
        let start = window.start_loc.unwrap_or_default();
        offset.set_x(start.x() + offset_x);
        offset.set_y(start.y() + offset_y);
        window.set_floating_offsets(Some(offset));
    }

    fn update_workspace_avoid_list(&mut self) {
        let mut avoid = vec![];
        self.state
            .windows
            .iter()
            .filter(|w| w.r#type == WindowType::Dock)
            .filter_map(|w| w.strut.map(|strut| (w.handle, strut)))
            .for_each(|(handle, to_avoid)| {
                tracing::debug!("AVOID STRUT:[{:?}] {:?}", handle, to_avoid);
                avoid.push(to_avoid);
            });
        for ws in &mut self.state.workspaces {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::InsertBehavior;
    use crate::models::Screen;
    use crate::Manager;

    #[test]
    fn insert_behavior_bottom_add_window_at_the_end_of_the_stack() {
        let mut manager = Manager::new_test(vec![]);
        manager.state.insert_behavior = InsertBehavior::Bottom;

        manager.screen_create_handler(Screen::default());
        manager.window_create_handler(
            Window::new(WindowHandle::MockHandle(1), None, None),
            -1,
            -1,
        );
        manager.window_create_handler(
            Window::new(WindowHandle::MockHandle(2), None, None),
            -1,
            -1,
        );

        let expected = vec![WindowHandle::MockHandle(1), WindowHandle::MockHandle(2)];

        let actual: Vec<WindowHandle> = manager.state.windows.iter().map(|w| w.handle).collect();

        assert_eq!(actual, expected);
    }

    #[test]
    fn insert_behavior_top_add_window_at_the_top_of_the_stack() {
        let mut manager = Manager::new_test(vec![]);
        manager.state.insert_behavior = InsertBehavior::Top;

        manager.screen_create_handler(Screen::default());
        manager.window_create_handler(
            Window::new(WindowHandle::MockHandle(1), None, None),
            -1,
            -1,
        );
        manager.window_create_handler(
            Window::new(WindowHandle::MockHandle(2), None, None),
            -1,
            -1,
        );

        let expected = vec![WindowHandle::MockHandle(2), WindowHandle::MockHandle(1)];
        let actual: Vec<WindowHandle> = manager.state.windows.iter().map(|w| w.handle).collect();

        assert_eq!(actual, expected);
    }

    #[test]
    fn insert_behavior_after_current_add_window_after_the_current_window() {
        let mut manager = Manager::new_test(vec![]);
        manager.state.insert_behavior = InsertBehavior::AfterCurrent;

        manager.screen_create_handler(Screen::default());
        manager.window_create_handler(
            Window::new(WindowHandle::MockHandle(1), None, None),
            -1,
            -1,
        );
        manager.window_create_handler(
            Window::new(WindowHandle::MockHandle(2), None, None),
            -1,
            -1,
        );
        manager.window_create_handler(
            Window::new(WindowHandle::MockHandle(3), None, None),
            -1,
            -1,
        );

        let expected = vec![
            WindowHandle::MockHandle(1),
            WindowHandle::MockHandle(3),
            WindowHandle::MockHandle(2),
        ];
        let actual: Vec<WindowHandle> = manager.state.windows.iter().map(|w| w.handle).collect();

        assert_eq!(actual, expected);
    }

    #[test]
    fn insert_behavior_before_current_add_window_before_the_current_window() {
        let mut manager = Manager::new_test(vec![]);
        manager.state.insert_behavior = InsertBehavior::BeforeCurrent;

        manager.screen_create_handler(Screen::default());
        manager.window_create_handler(
            Window::new(WindowHandle::MockHandle(1), None, None),
            -1,
            -1,
        );
        manager.window_create_handler(
            Window::new(WindowHandle::MockHandle(2), None, None),
            -1,
            -1,
        );

        manager.window_create_handler(
            Window::new(WindowHandle::MockHandle(3), None, None),
            -1,
            -1,
        );

        let expected = vec![
            WindowHandle::MockHandle(2),
            WindowHandle::MockHandle(3),
            WindowHandle::MockHandle(1),
        ];
        let actual: Vec<WindowHandle> = manager.state.windows.iter().map(|w| w.handle).collect();

        assert_eq!(actual, expected);
    }
}
