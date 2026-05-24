pub(crate) mod input;

use std::ffi::c_void;
use std::ptr::NonNull;

use iced::Program;
use iced_graphics::compositor::{self, Compositor};
use iced_graphics::{Settings, Shell, Viewport};
use iced_program::Instance;
use iced_runtime::core::{
    Color,
    Event,
    Point,
    Size,
    clipboard,
    mouse,
    renderer,
    theme,
    window,
};
use iced_runtime::user_interface::{Cache, UserInterface};
use raw_window_handle::{
    DisplayHandle,
    HandleError,
    HasDisplayHandle,
    HasWindowHandle,
    RawDisplayHandle,
    RawWindowHandle,
    WaylandDisplayHandle,
    WaylandWindowHandle,
    WindowHandle,
};

use crate::error::Error;

/// The compositor type associated with a program's renderer.
type CompositorOf<P> = <<P as Program>::Renderer as compositor::Default>::Compositor;

/// The surface type produced by that compositor.
type SurfaceOf<P> = <CompositorOf<P> as Compositor>::Surface;

/// A renderable overlay decoupled from the concrete program type.
///
/// [`crate::wayland`] holds this as a trait object so its `State` can stay
/// non-generic while the Iced program type is erased behind the boundary.
pub(crate) trait Overlay {
    /// Resize the surface and viewport to `width`x`height` physical pixels.
    fn resize(&mut self, width: u32, height: u32, scale: f64);

    /// Queue an input event to be processed on the next [`Overlay::draw`].
    fn queue_event(&mut self, event: Event);

    /// Update the cursor position (in logical coordinates), or clear it.
    fn set_cursor(&mut self, position: Option<Point>);

    /// Process queued events and render the current Iced UI into the surface.
    fn draw(&mut self);
}

/// Raw Wayland handles for the overlay surface, in `raw-window-handle` form.
#[derive(Clone, Copy)]
struct RawSurface {
    display: NonNull<c_void>,
    surface: NonNull<c_void>,
}

// SAFETY: the pointers reference long-lived `wl_display`/`wl_surface` objects;
// libwayland access is internally synchronised.
unsafe impl Send for RawSurface {}
unsafe impl Sync for RawSurface {}

impl HasDisplayHandle for RawSurface {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        let raw = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(self.display));
        // SAFETY: the display outlives every handle borrowed from it.
        Ok(unsafe { DisplayHandle::borrow_raw(raw) })
    }
}

impl HasWindowHandle for RawSurface {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let raw = RawWindowHandle::Wayland(WaylandWindowHandle::new(self.surface));
        // SAFETY: the surface outlives every handle borrowed from it.
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

/// An Iced program rendered into a Wayland surface via its wgpu compositor.
pub(crate) struct IcedOverlay<P: Program> {
    instance: Instance<P>,
    window_id: window::Id,
    compositor: CompositorOf<P>,
    renderer: P::Renderer,
    surface: SurfaceOf<P>,
    default_theme: P::Theme,
    cache: Option<Cache>,
    viewport: Viewport,
    width: u32,
    height: u32,
    scale: f64,
    events: Vec<Event>,
    cursor: mouse::Cursor,
}

impl<P: Program> IcedOverlay<P> {
    /// Build the renderer/compositor for `program` on the given overlay surface.
    pub(crate) fn new(
        program: P,
        display: NonNull<c_void>,
        surface: NonNull<c_void>,
        width: u32,
        height: u32,
        scale: f64,
    ) -> Result<Self, Error> {
        let raw = RawSurface { display, surface };

        let mut compositor = pollster::block_on(CompositorOf::<P>::new(
            Settings::default(),
            raw,
            raw,
            Shell::headless(),
        ))
        .map_err(|err| Error::RendererInit {
            message: err.to_string(),
        })?;

        let renderer = compositor.create_renderer();
        let surface = compositor.create_surface(raw, width.max(1), height.max(1));

        let (instance, _boot_task) = Instance::new(program);
        let window_id = window::Id::unique();
        let default_theme = <P::Theme as theme::Base>::default(theme::Mode::default());
        let viewport =
            Viewport::with_physical_size(Size::new(width, height), scale as f32);

        // TODO(phase 5): drive `_boot_task` through the Iced runtime.

        Ok(Self {
            instance,
            window_id,
            compositor,
            renderer,
            surface,
            default_theme,
            cache: Some(Cache::default()),
            viewport,
            width,
            height,
            scale,
            events: Vec::new(),
            cursor: mouse::Cursor::Unavailable,
        })
    }
}

impl<P: Program> Overlay for IcedOverlay<P> {
    fn resize(&mut self, width: u32, height: u32, scale: f64) {
        if self.width == width && self.height == height && self.scale == scale {
            return;
        }

        self.width = width;
        self.height = height;
        self.scale = scale;
        self.viewport =
            Viewport::with_physical_size(Size::new(width, height), scale as f32);
        self.compositor.configure_surface(
            &mut self.surface,
            width.max(1),
            height.max(1),
        );
    }

    fn queue_event(&mut self, event: Event) {
        self.events.push(event);
    }

    fn set_cursor(&mut self, position: Option<Point>) {
        self.cursor = match position {
            Some(position) => mouse::Cursor::Available(position),
            None => mouse::Cursor::Unavailable,
        };
    }

    fn draw(&mut self) {
        let bounds = self.viewport.logical_size();
        let mut cache = self.cache.take().unwrap_or_default();

        // Process queued input events; apply any resulting messages to the
        // program state. TODO(phase 5): drive the returned `Task`s through the
        // Iced runtime instead of dropping them.
        let events = std::mem::take(&mut self.events);
        if !events.is_empty() {
            let mut messages = Vec::new();
            let mut clipboard = clipboard::Null;
            let mut ui = UserInterface::build(
                self.instance.view(self.window_id),
                bounds,
                cache,
                &mut self.renderer,
            );
            let _ = ui.update(
                &events,
                self.cursor,
                &mut self.renderer,
                &mut clipboard,
                &mut messages,
            );
            cache = ui.into_cache();

            for message in messages {
                let _task = self.instance.update(message);
            }
        }

        let theme = self.instance.theme(self.window_id);
        let theme = theme.as_ref().unwrap_or(&self.default_theme);
        let text_color = self.instance.style(theme).text_color;

        let mut ui = UserInterface::build(
            self.instance.view(self.window_id),
            bounds,
            cache,
            &mut self.renderer,
        );
        ui.draw(
            &mut self.renderer,
            theme,
            &renderer::Style { text_color },
            self.cursor,
        );

        self.cache = Some(ui.into_cache());

        match self.compositor.present(
            &mut self.renderer,
            &mut self.surface,
            &self.viewport,
            Color::TRANSPARENT,
            || {},
        ) {
            Ok(()) => {},
            Err(compositor::SurfaceError::Outdated | compositor::SurfaceError::Lost) => {
                self.compositor.configure_surface(
                    &mut self.surface,
                    self.width.max(1),
                    self.height.max(1),
                );
            },
            Err(err) => {
                tracing::warn!("overlay present failed: {err}");
            },
        }
    }
}
