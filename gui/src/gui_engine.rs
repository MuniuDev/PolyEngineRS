use std::sync::Arc;

use crate::window::Window;

pub struct GuiEngine {
    windows : Vec<Arc<Window>>,
}

impl GuiEngine {
    pub fn new() -> Self {
        return GuiEngine{ 
            windows: Vec::new(),
         };
    }

    pub fn register_widget(&self) {

    }

    pub fn open_window(&mut self) -> Arc<Window> {
        let window = Arc::new(Window{});
        self.windows.push(window.clone());
        return window;
    }
}