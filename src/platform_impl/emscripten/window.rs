use crate::cursor::Cursor;
use crate::error::{ExternalError, NotSupportedError};
use crate::event::{Event, WindowEvent};
use crate::icon::Icon;
use crate::platform_impl::emscripten::event::mouse::mouse_callback;
use crate::platform_impl::emscripten::event::window::window_resize_callback;
use crate::platform_impl::emscripten::event_hub::EventHub;
use crate::platform_impl::emscripten::ffi::EM_CALLBACK_THREAD_CONTEXT_CALLING_THREAD;
use crate::platform_impl::emscripten::{
    bindings, em_try, ffi, BODY_NAME, DOCUMENT_NAME, WINDOW_NAME,
};
use crate::platform_impl::platform::get_hidpi_factor;
use crate::platform_impl::{ActiveEventLoop, Fullscreen, OsError};
use crate::window::{
    CursorGrabMode, ImePurpose, ResizeDirection, Theme, UserAttentionType, WindowAttributes,
    WindowButtons, WindowLevel,
};
use dpi::{LogicalSize, PhysicalPosition, PhysicalSize, Position, Size};
use std::collections::VecDeque;
use std::ffi::CString;
use std::iter::Empty;
use std::os::raw::c_char;
use std::ptr::null_mut;
use deft_emscripten_sys::emscripten_run_script;
use crate::platform_impl::emscripten::bindings::emscripten_get_element_css_size;
use crate::platform_impl::emscripten::event::keyboard::keyboard_callback;
use crate::platform_impl::emscripten::monitor::MonitorHandle;

#[allow(unused)]
#[derive(Clone, Debug, Default)]
pub struct PlatformSpecificWindowAttributes {
    pub(crate) prevent_default: bool,
    pub(crate) focusable: bool,
    pub(crate) append: bool,
}

#[derive(Clone, Default)]
pub struct PlatformSpecificWindowBuilderAttributes;

unsafe impl Send for PlatformSpecificWindowBuilderAttributes {}
unsafe impl Sync for PlatformSpecificWindowBuilderAttributes {}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowId(pub usize);

impl Into<u64> for WindowId {
    fn into(self) -> u64 {
        self.0 as u64
    }
}

impl Into<WindowId> for u64 {
    fn into(self) -> WindowId {
        WindowId(self as usize)
    }
}

impl WindowId {
    pub const fn dummy() -> Self {
        WindowId(0)
    }
}

pub struct Inner {
    id: WindowId,
}

pub struct Window {
    inner: Inner,
}

impl MonitorHandle {
    pub fn scale_factor(&self) -> f64 {
        get_hidpi_factor()
    }

    pub fn position(&self) -> PhysicalPosition<i32> {
        unreachable!()
    }

    pub fn name(&self) -> Option<String> {
        unreachable!()
    }

    pub fn refresh_rate_millihertz(&self) -> Option<u32> {
        unreachable!()
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        unreachable!()
    }

    pub fn video_modes(&self) -> Empty<VideoModeHandle> {
        unreachable!()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VideoModeHandle;

impl VideoModeHandle {
    pub fn size(&self) -> PhysicalSize<u32> {
        unreachable!();
    }

    pub fn bit_depth(&self) -> u16 {
        unreachable!();
    }

    pub fn refresh_rate_millihertz(&self) -> u32 {
        unreachable!();
    }

    pub fn monitor(&self) -> MonitorHandle {
        unreachable!();
    }
}

impl Window {
    pub fn new(
        _active_event_loop: &ActiveEventLoop,
        _attribs: WindowAttributes,
    ) -> Result<Window, crate::error::OsError> {
        unsafe {
            let script = CString::new(include_bytes!("init-input.js")).unwrap();
            emscripten_run_script(script.as_ptr());
        }
        let window = Window { inner: Inner { id: WindowId::dummy() } };

        // TODO: set up more event callbacks
        let user_data = null_mut();
        unsafe {
            let mut width = 0.0;
            let mut height = 0.0;
            bindings::emscripten_get_element_css_size(
                BODY_NAME.as_ptr() as *const c_char,
                &mut width as *mut _,
                &mut height as *mut _,
            );
            // println!("window size: {:?}", (width, height));
            let size = LogicalSize::new(width, height).to_physical(get_hidpi_factor());
            let event = Event::WindowEvent {
                window_id: crate::window::WindowId(WindowId(0)),
                event: WindowEvent::Resized(size),
            };
            EventHub::send_event(event);

            em_try(bindings::emscripten_set_resize_callback_on_thread(
                WINDOW_NAME.as_ptr() as *const c_char,
                user_data,
                false,
                Some(window_resize_callback),
                EM_CALLBACK_THREAD_CONTEXT_CALLING_THREAD as u32,
            ))
            .unwrap();
            em_try(bindings::emscripten_set_mousemove_callback_on_thread(
                DOCUMENT_NAME.as_ptr() as *const c_char,
                user_data,
                false,
                Some(mouse_callback),
                ffi::EM_CALLBACK_THREAD_CONTEXT_CALLING_THREAD,
            ))
            .map_err(|e| OsError(format!("emscripten error: {}", e)))
            .unwrap();
            em_try(bindings::emscripten_set_mousedown_callback_on_thread(
                DOCUMENT_NAME.as_ptr() as *const c_char,
                user_data,
                false,
                Some(mouse_callback),
                ffi::EM_CALLBACK_THREAD_CONTEXT_CALLING_THREAD,
            ))
            .map_err(|e| OsError(format!("emscripten error: {}", e)))
            .unwrap();
            em_try(bindings::emscripten_set_mouseup_callback_on_thread(
                DOCUMENT_NAME.as_ptr() as *const c_char,
                user_data,
                false,
                Some(mouse_callback),
                ffi::EM_CALLBACK_THREAD_CONTEXT_CALLING_THREAD,
            ))
            .map_err(|e| OsError(format!("emscripten error: {}", e)))
            .unwrap();

            em_try(bindings::emscripten_set_keydown_callback_on_thread(
                DOCUMENT_NAME.as_ptr() as *const c_char,
                user_data,
                false,
                Some(keyboard_callback),
                ffi::EM_CALLBACK_THREAD_CONTEXT_CALLING_THREAD as u32,
            ))
            .map_err(|e| OsError(format!("emscripten error: {}", e)))
            .unwrap();
            em_try(bindings::emscripten_set_keyup_callback_on_thread(
                DOCUMENT_NAME.as_ptr() as *const c_char,
                user_data,
                false,
                Some(keyboard_callback),
                ffi::EM_CALLBACK_THREAD_CONTEXT_CALLING_THREAD as u32,
            ))
            .map_err(|e| OsError(format!("emscripten error: {}", e)))
            .unwrap();
        }
        //TODO support fullscreen attr
        Ok(window)
    }

    pub(crate) fn maybe_queue_on_main(&self, f: impl FnOnce(&Inner) + Send + 'static) {
        f(&self.inner);
    }

    pub(crate) fn maybe_wait_on_main<R: Send>(&self, f: impl FnOnce(&Inner) -> R + Send) -> R {
        f(&self.inner)
    }

    pub(crate) fn prevent_default(&self) -> bool {
        //TODO impl
        false
    }

    pub(crate) fn set_prevent_default(&self, _prevent_default: bool) {
        //TODO impl
    }

    #[cfg(feature = "rwh_06")]
    #[inline]
    pub fn raw_window_handle_rwh_06(&self) -> Result<rwh_06::RawWindowHandle, rwh_06::HandleError> {
        Err(rwh_06::HandleError::Unavailable)
    }

    #[cfg(feature = "rwh_06")]
    #[inline]
    pub(crate) fn raw_display_handle_rwh_06(
        &self,
    ) -> Result<rwh_06::RawDisplayHandle, rwh_06::HandleError> {
        Ok(rwh_06::RawDisplayHandle::Web(rwh_06::WebDisplayHandle::new()))
    }
}

impl Inner {
    pub fn set_title(&self, _title: &str) {}

    pub fn set_transparent(&self, _transparent: bool) {}

    pub fn set_blur(&self, _blur: bool) {}

    pub fn set_visible(&self, _visible: bool) {
        // Intentionally a no-op
    }

    #[inline]
    pub fn is_visible(&self) -> Option<bool> {
        None
    }

    pub fn request_redraw(&self) {
        EventHub::send_event(Event::WindowEvent {
            window_id: crate::window::WindowId(self.id),
            event: WindowEvent::RedrawRequested,
        });
    }

    pub fn pre_present_notify(&self) {}

    pub fn pointer_position(&self) -> Option<PhysicalPosition<i32>> {
        None
    }

    pub fn outer_position(&self) -> Result<PhysicalPosition<i32>, NotSupportedError> {
        Err(NotSupportedError::new())
    }

    pub fn inner_position(&self) -> Result<PhysicalPosition<i32>, NotSupportedError> {
        // Note: the canvas element has no window decorations, so this is equal to `outer_position`.
        self.outer_position()
    }

    pub fn set_outer_position(&self, _position: Position) {
        //TODO
    }

    #[inline]
    pub fn inner_size(&self) -> PhysicalSize<u32> {
        unsafe {
            let mut width = 0.0;
            let mut height = 0.0;
            emscripten_get_element_css_size(
                BODY_NAME.as_ptr() as *const c_char,
                &mut width as *mut _,
                &mut height as *mut _,
            );
            LogicalSize::new(width as u32, height as u32).to_physical(get_hidpi_factor())
        }
    }

    #[inline]
    pub fn outer_size(&self) -> PhysicalSize<u32> {
        // Note: the canvas element has no window decorations, so this is equal to `inner_size`.
        self.inner_size()
    }

    #[inline]
    pub fn request_inner_size(&self, _size: Size) -> Option<PhysicalSize<u32>> {
        None
    }

    #[inline]
    pub fn set_min_inner_size(&self, _dimensions: Option<Size>) {
        //TODO impl
    }

    #[inline]
    pub fn set_max_inner_size(&self, _dimensions: Option<Size>) {
        //TODO impl
    }

    #[inline]
    pub fn resize_increments(&self) -> Option<PhysicalSize<u32>> {
        None
    }

    #[inline]
    pub fn set_resize_increments(&self, _increments: Option<Size>) {
        // Intentionally a no-op: users can't resize canvas elements
    }

    #[inline]
    pub fn set_resizable(&self, _resizable: bool) {
        // Intentionally a no-op: users can't resize canvas elements
    }

    pub fn is_resizable(&self) -> bool {
        true
    }

    #[inline]
    pub fn set_enabled_buttons(&self, _buttons: WindowButtons) {}

    #[inline]
    pub fn enabled_buttons(&self) -> WindowButtons {
        WindowButtons::all()
    }

    #[inline]
    pub fn scale_factor(&self) -> f64 {
        get_hidpi_factor()
    }

    #[inline]
    pub fn set_cursor(&self, _cursor: Cursor) {
        //TODO impl
    }

    #[inline]
    pub fn set_cursor_position(&self, _position: Position) -> Result<(), ExternalError> {
        Err(ExternalError::NotSupported(NotSupportedError::new()))
    }

    #[inline]
    pub fn set_cursor_grab(&self, _mode: CursorGrabMode) -> Result<(), ExternalError> {
        //TODO impl
        Err(ExternalError::NotSupported(NotSupportedError::new()))
    }

    #[inline]
    pub fn set_cursor_visible(&self, _visible: bool) {
        //TODO impl
    }

    #[inline]
    pub fn drag_window(&self) -> Result<(), ExternalError> {
        Err(ExternalError::NotSupported(NotSupportedError::new()))
    }

    #[inline]
    pub fn drag_resize_window(&self, _direction: ResizeDirection) -> Result<(), ExternalError> {
        Err(ExternalError::NotSupported(NotSupportedError::new()))
    }

    #[inline]
    pub fn show_window_menu(&self, _position: Position) {}

    #[inline]
    pub fn set_cursor_hittest(&self, _hittest: bool) -> Result<(), ExternalError> {
        Err(ExternalError::NotSupported(NotSupportedError::new()))
    }

    #[inline]
    pub fn set_minimized(&self, _minimized: bool) {
        // Intentionally a no-op, as canvases cannot be 'minimized'
    }

    #[inline]
    pub fn is_minimized(&self) -> Option<bool> {
        // Canvas cannot be 'minimized'
        Some(false)
    }

    #[inline]
    pub fn set_maximized(&self, _maximized: bool) {
        // Intentionally a no-op, as canvases cannot be 'maximized'
    }

    #[inline]
    pub fn is_maximized(&self) -> bool {
        // Canvas cannot be 'maximized'
        false
    }

    #[inline]
    pub(crate) fn fullscreen(&self) -> Option<Fullscreen> {
        //TODO impl
        None
    }

    #[inline]
    pub(crate) fn set_fullscreen(&self, _fullscreen: Option<Fullscreen>) {
        //TODO impl
    }

    #[inline]
    pub fn set_decorations(&self, _decorations: bool) {
        // Intentionally a no-op, no canvas decorations
    }

    pub fn is_decorated(&self) -> bool {
        true
    }

    #[inline]
    pub fn set_window_level(&self, _level: WindowLevel) {
        // Intentionally a no-op, no window ordering
    }

    #[inline]
    pub fn set_window_icon(&self, _window_icon: Option<Icon>) {
        // Currently an intentional no-op
    }

    #[inline]
    pub fn set_ime_cursor_area(&self, position: Position, size: Size) {
        // Currently a no-op as it does not seem there is good support for this on web
        let hidpi_factor = get_hidpi_factor();
        let position = position.to_logical::<i32>(hidpi_factor);
        let size = size.to_logical::<i32>(hidpi_factor);
        unsafe {
            let code = format!("Module.winit.setImeCursor({}, {}, {}, {})", position.x, position.y, size.width, size.height);
            let script = CString::new(code).unwrap();
            emscripten_run_script(script.as_ptr());
        }
    }

    #[inline]
    pub fn set_ime_allowed(&self, allowed: bool) {
        unsafe {
            let code = if allowed {
                "Module.winit.allowIme(true)"
            } else {
                "Module.winit.allowIme(false)"
            };
            let script = CString::new(code).unwrap();
            emscripten_run_script(script.as_ptr());
        }
    }

    #[inline]
    pub fn set_ime_purpose(&self, _purpose: ImePurpose) {
        // Currently not implemented
    }

    #[inline]
    pub fn commit_ime(&self) {}

    #[inline]
    pub fn focus_window(&self) {
        // let _ = self.canvas.borrow().raw().focus();
    }

    #[inline]
    pub fn request_user_attention(&self, _request_type: Option<UserAttentionType>) {
        // Currently an intentional no-op
    }

    #[inline]
    pub fn current_monitor(&self) -> Option<MonitorHandle> {
        None
    }

    #[inline]
    pub fn available_monitors(&self) -> VecDeque<MonitorHandle> {
        VecDeque::new()
    }

    #[inline]
    pub fn primary_monitor(&self) -> Option<MonitorHandle> {
        None
    }

    #[inline]
    pub fn id(&self) -> WindowId {
        self.id
    }

    #[cfg(feature = "rwh_04")]
    #[inline]
    pub fn raw_window_handle_rwh_04(&self) -> rwh_04::RawWindowHandle {
        let mut window_handle = rwh_04::WebHandle::empty();
        window_handle.id = self.id.0;
        rwh_04::RawWindowHandle::Web(window_handle)
    }

    #[cfg(feature = "rwh_05")]
    #[inline]
    pub fn raw_window_handle_rwh_05(&self) -> rwh_05::RawWindowHandle {
        let mut window_handle = rwh_05::WebWindowHandle::empty();
        window_handle.id = self.id.0 as u32;
        rwh_05::RawWindowHandle::Web(window_handle)
    }

    #[cfg(feature = "rwh_05")]
    #[inline]
    pub fn raw_display_handle_rwh_05(&self) -> rwh_05::RawDisplayHandle {
        rwh_05::RawDisplayHandle::Web(rwh_05::WebDisplayHandle::empty())
    }

    #[inline]
    pub fn set_theme(&self, _theme: Option<Theme>) {}

    #[inline]
    pub fn theme(&self) -> Option<Theme> {
        //TODO impl
        None
    }

    pub fn set_content_protected(&self, _protected: bool) {}

    #[inline]
    pub fn has_focus(&self) -> bool {
        //TODO impl
        true
    }

    pub fn title(&self) -> String {
        String::new()
    }

    pub fn reset_dead_keys(&self) {
        // Not supported
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        //TODO release event bindings?
    }
}
