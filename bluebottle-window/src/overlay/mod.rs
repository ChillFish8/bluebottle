use std::sync::atomic::Ordering;
use std::sync::{Arc, mpsc as sync_chan};
use std::time::Duration;

use iced::Program;
use iced_futures::futures::channel::mpsc;
use iced_futures::{Executor as _, Runtime, subscription};
use iced_graphics::compositor::{self, Compositor};
use iced_graphics::{Settings, Shell, Viewport};
use iced_program::Instance;
use iced_runtime::core::time::Instant;
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
use iced_runtime::user_interface::{self, Cache, UserInterface};
use iced_runtime::{Action, task};

use crate::error::Error;
use crate::handle::Shared;

/// How long the render thread waits for a command before re-checking for async
/// runtime output and pending animation frames.
const REDRAW_INTERVAL: Duration = Duration::from_millis(16);

/// A message from the Wayland event loop to the overlay render thread.
pub(crate) enum Command {
    /// An input event to feed to the Iced UI.
    Event(Event),
    /// The cursor moved to a logical position, or left the surface (`None`).
    Cursor(Option<Point>),
    /// The window's logical size and/or integer scale changed.
    Resize { width: u32, height: u32, scale: f64 },
}

/// Build the overlay on this (render) thread and drive it until close.
///
/// `target` provides the raw window/display handles of the overlay surface the
/// platform backend created; the renderer is built against it here. The Iced
/// program and wgpu renderer are not `Send`, so they are built on this thread
/// rather than moved across threads; only the `build` closure and the (`Send`)
/// `target` cross the boundary. The build result is reported through `ready` so
/// the spawning thread can fail fast. Running the renderer on its own thread
/// lets it block on surface presentation without stalling the platform event
/// loop (which must keep dispatching for the caller to map the main surface).
#[allow(clippy::too_many_arguments)]
pub(crate) fn run<P, F, W>(
    build: F,
    target: W,
    size: (u32, u32),
    scale: f64,
    commands: sync_chan::Receiver<Command>,
    shared: Arc<Shared>,
    ready: sync_chan::Sender<Result<(), Error>>,
) where
    F: FnOnce() -> P,
    P: Program,
    W: compositor::Window + Clone,
{
    let overlay = match IcedOverlay::new(build(), target, size.0, size.1, scale) {
        Ok(overlay) => {
            let _ = ready.send(Ok(()));
            overlay
        },
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        },
    };

    render_loop(overlay, commands, shared);
}

/// Drive the overlay: apply commands, pump the runtime, and redraw as needed.
fn render_loop<P: Program>(
    mut overlay: IcedOverlay<P>,
    commands: sync_chan::Receiver<Command>,
    shared: Arc<Shared>,
) {
    while !shared.close_requested.load(Ordering::Acquire) {
        let mut dirty = match commands.recv_timeout(REDRAW_INTERVAL) {
            Ok(command) => apply_command(&mut overlay, command),
            Err(sync_chan::RecvTimeoutError::Timeout) => false,
            Err(sync_chan::RecvTimeoutError::Disconnected) => break,
        };
        while let Ok(command) = commands.try_recv() {
            dirty |= apply_command(&mut overlay, command);
        }

        dirty |= overlay.pump();

        if overlay.should_exit() {
            shared.close_requested.store(true, Ordering::Release);
            break;
        }

        if dirty || overlay.wants_redraw() {
            overlay.draw();
        }
    }
}

/// Apply a single [`Command`], returning whether it requires a redraw.
fn apply_command<P: Program>(overlay: &mut IcedOverlay<P>, command: Command) -> bool {
    match command {
        Command::Event(event) => {
            overlay.queue_event(event);
            true
        },
        Command::Cursor(position) => {
            overlay.set_cursor(position);
            false
        },
        Command::Resize {
            width,
            height,
            scale,
        } => {
            overlay.resize(width, height, scale);
            true
        },
    }
}

/// The compositor type associated with a program's renderer.
type CompositorOf<P> = <<P as Program>::Renderer as compositor::Default>::Compositor;

/// The surface type produced by that compositor.
type SurfaceOf<P> = <CompositorOf<P> as Compositor>::Surface;

/// Upper bound on a surface dimension, clamped to stay within GPU texture limits.
///
/// iced creates the wgpu device with wgpu's default limits, whose
/// `max_texture_dimension_2d` is 8192 regardless of the hardware maximum.
const MAX_SURFACE_DIMENSION: u32 = 8192;

/// Convert a logical size at a given scale to a physical pixel size.
///
/// Clamped to `[1, MAX_SURFACE_DIMENSION]` so an oversized window cannot exceed
/// the renderer's maximum texture dimension.
fn physical_size(logical_width: u32, logical_height: u32, scale: f64) -> (u32, u32) {
    let to_physical = |logical: u32| {
        ((logical as f64) * scale)
            .round()
            .clamp(1.0, MAX_SURFACE_DIMENSION as f64) as u32
    };
    (to_physical(logical_width), to_physical(logical_height))
}

/// The runtime action type produced by a program's messages.
type ActionOf<P> = Action<<P as Program>::Message>;

/// The Iced runtime specialised for a program, sending actions over an mpsc channel.
type RuntimeOf<P> =
    Runtime<<P as Program>::Executor, mpsc::UnboundedSender<ActionOf<P>>, ActionOf<P>>;

/// An Iced program rendered into a platform surface via its wgpu compositor.
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
    runtime: RuntimeOf<P>,
    receiver: mpsc::UnboundedReceiver<ActionOf<P>>,
    redraw_request: window::RedrawRequest,
    exit: bool,
}

impl<P: Program> IcedOverlay<P> {
    /// Build the renderer/compositor for `program` on the given overlay surface.
    pub(crate) fn new<W>(
        program: P,
        target: W,
        width: u32,
        height: u32,
        scale: f64,
    ) -> Result<Self, Error>
    where
        W: compositor::Window + Clone,
    {
        let mut compositor = pollster::block_on(CompositorOf::<P>::new(
            Settings::default(),
            target.clone(),
            target.clone(),
            Shell::headless(),
        ))
        .map_err(|err| Error::RendererInit {
            message: err.to_string(),
        })?;

        let (physical_width, physical_height) = physical_size(width, height, scale);
        let renderer = compositor.create_renderer();
        let surface = compositor.create_surface(target, physical_width, physical_height);

        let executor =
            P::Executor::new().map_err(|source| Error::Executor { source })?;
        let (sender, receiver) = mpsc::unbounded();
        let mut runtime = RuntimeOf::<P>::new(executor, sender);

        let (instance, boot_task) = Instance::new(program);
        let window_id = window::Id::unique();
        let default_theme = <P::Theme as theme::Base>::default(theme::Mode::default());
        let viewport = Viewport::with_physical_size(
            Size::new(physical_width, physical_height),
            scale as f32,
        );

        if let Some(stream) = task::into_stream(boot_task) {
            runtime.run(stream);
        }

        let mut overlay = Self {
            instance,
            window_id,
            compositor,
            renderer,
            surface,
            default_theme,
            cache: Some(Cache::default()),
            viewport,
            width: physical_width,
            height: physical_height,
            scale,
            events: Vec::new(),
            cursor: mouse::Cursor::Unavailable,
            runtime,
            receiver,
            redraw_request: window::RedrawRequest::Wait,
            exit: false,
        };

        overlay.sync_subscriptions();
        Ok(overlay)
    }

    /// Apply a message to the program state, running any resulting [`Task`].
    ///
    /// [`Task`]: iced_runtime::Task
    fn apply_message(&mut self, message: P::Message) {
        let task = self.instance.update(message);
        if let Some(stream) = task::into_stream(task) {
            self.runtime.run(stream);
        }
    }

    /// Recompute the program's [`Subscription`] and reconcile it in the runtime.
    ///
    /// [`Subscription`]: iced_futures::Subscription
    fn sync_subscriptions(&mut self) {
        let subscription = self.runtime.enter(|| self.instance.subscription());
        self.runtime
            .track(subscription::into_recipes(subscription.map(Action::Output)));
    }
}

impl<P: Program> IcedOverlay<P> {
    fn resize(&mut self, logical_width: u32, logical_height: u32, scale: f64) {
        let (width, height) = physical_size(logical_width, logical_height, scale);
        if self.width == width && self.height == height && self.scale == scale {
            return;
        }

        self.width = width;
        self.height = height;
        self.scale = scale;
        self.viewport =
            Viewport::with_physical_size(Size::new(width, height), scale as f32);
        self.compositor
            .configure_surface(&mut self.surface, width, height);
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

    fn pump(&mut self) -> bool {
        let mut applied = false;

        while let Ok(Some(action)) = self.receiver.try_next() {
            match action {
                Action::Output(message) => {
                    self.apply_message(message);
                    applied = true;
                },
                Action::Exit => self.exit = true,
                // Clipboard, widget operations, window/system actions, etc. are
                // not yet supported for an overlay; ignore them for now.
                _ => {},
            }
        }

        if applied {
            self.sync_subscriptions();
        }

        applied
    }

    fn wants_redraw(&self) -> bool {
        match self.redraw_request {
            window::RedrawRequest::NextFrame => true,
            window::RedrawRequest::At(instant) => Instant::now() >= instant,
            window::RedrawRequest::Wait => false,
        }
    }

    fn should_exit(&self) -> bool {
        self.exit
    }

    fn draw(&mut self) {
        let bounds = self.viewport.logical_size();
        let mut cache = self.cache.take().unwrap_or_default();

        // Run the update pass (even with no input events) so animation redraw
        // requests are refreshed, then apply any produced messages.
        let events = std::mem::take(&mut self.events);
        let mut messages = Vec::new();
        {
            let mut clipboard = clipboard::Null;
            let mut ui = UserInterface::build(
                self.instance.view(self.window_id),
                bounds,
                cache,
                &mut self.renderer,
            );
            let (state, _statuses) = ui.update(
                &events,
                self.cursor,
                &mut self.renderer,
                &mut clipboard,
                &mut messages,
            );
            self.redraw_request = match state {
                user_interface::State::Updated { redraw_request, .. } => redraw_request,
                user_interface::State::Outdated => window::RedrawRequest::NextFrame,
            };
            cache = ui.into_cache();
        }

        let applied = !messages.is_empty();
        for message in messages {
            self.apply_message(message);
        }
        if applied {
            self.sync_subscriptions();
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
                // The frame we tried to present was dropped; ask the loop to
                // repaint so the overlay does not stay blank until the next
                // input or animation frame happens to arrive.
                self.redraw_request = window::RedrawRequest::NextFrame;
            },
            Err(err) => {
                tracing::warn!("overlay present failed: {err}");
            },
        }
    }
}
