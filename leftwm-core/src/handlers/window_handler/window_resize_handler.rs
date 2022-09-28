use crate::models::WindowHandle;
use crate::Manager;
use crate::config::Config;
use crate::display_servers::DisplayServer;

impl<C, SERVER> Manager<C, SERVER> where C: Config, SERVER: DisplayServer {
    pub fn window_resize_handler(
        &mut self,
        handle: &WindowHandle,
        offset_w: i32,
        offset_h: i32,
    ) -> bool {
        if let Some(w) = self.state.windows.iter_mut().find(|w| &w.handle == handle) {
            self.process_window(w, offset_w, offset_h);
            return true;
        }
        false
    }
}
