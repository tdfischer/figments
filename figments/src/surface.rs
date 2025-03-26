use crate::prelude::*;

use super::atomics::AtomicMutex;

use alloc::boxed::Box;
use alloc::vec::Vec;

use core::{marker::PhantomData, sync::atomic::AtomicBool};
use alloc::sync::Arc;
use core::fmt::{Debug, Formatter};
use ringbuf::{traits::*, HeapRb};

use crate::liber8tion::interpolate::Fract8Ops;
use core::cell::RefCell;

struct ShaderBinding<U> {
    shader: Option<Box<dyn Shader<U>>>,
    rect: Rectangle<Virtual>,
    opacity: u8,
    visible: bool
}

struct SurfaceUpdate<U> {
    shader: Option<Option<Box<dyn Shader<U>>>>,
    rect: Option<Rectangle<Virtual>>,
    opacity: Option<u8>,
    visible: Option<bool>,
    slot: usize,
}

impl<U> Debug for SurfaceUpdate<U> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SurfaceUpdate")
            .field("slot", &self.slot)
            .finish()
    }
}

impl<U> SurfaceUpdate<U> {
    fn merge(&mut self, mut other: Self) {
        if other.shader.is_some() {
            self.shader = other.shader.take()
        }
        if other.rect.is_some() {
            self.rect = other.rect.take()
        }
        if other.opacity.is_some() {
            self.opacity = other.opacity.take()
        }
        if other.visible.is_some() {
            self.visible = other.visible.take()
        }
    }
}

impl<U> Default for SurfaceUpdate<U> {
    fn default() -> Self {
        SurfaceUpdate {
            shader: None,
            rect: None,
            opacity: None,
            visible: None,
            slot: usize::MAX
        }
    }
}

/// A thread-safe [Surface] implementation where changes are buffered before they are committed in batches
pub struct BufferedSurface<U> {
    updater: Arc<UpdateQueue<U>>,
    slot: usize
}

impl<U> Visible for BufferedSurface<U> {
    fn set_opacity(&mut self, opacity: u8) {
        self.updater.push(SurfaceUpdate {
            opacity: Some(opacity),
            slot: self.slot,
            ..Default::default()
        }).unwrap();
    }

    fn set_visible(&mut self, visible: bool) {
        self.updater.push(SurfaceUpdate {
            visible: Some(visible),
            slot: self.slot,
            ..Default::default()
        }).unwrap();
    }
}

impl<U> Surface<U> for BufferedSurface<U> {
    fn clear_shader(&mut self) {
        self.updater.push(SurfaceUpdate {
            shader: Some(None),
            slot: self.slot,
            ..Default::default()
        }).unwrap();
    }

    fn set_rect(&mut self, rect: Rectangle<Virtual>) {
        self.updater.push(SurfaceUpdate {
            rect: Some(rect),
            slot: self.slot,
            ..Default::default()
        }).unwrap();
    }

    fn set_shader<T: Shader<U>>(&mut self, shader: T) {
        self.updater.push(SurfaceUpdate {
            shader: Some(Some(Box::new(shader))),
            slot: self.slot,
            ..Default::default()
        }).unwrap();
    }
}

struct UpdateQueue<U> {
    pending: AtomicMutex<HeapRb<SurfaceUpdate<U>>>,
    damaged: AtomicBool
}

impl<U> Default for UpdateQueue<U> {
    fn default() -> Self {
        Self {
            pending: AtomicMutex::new(HeapRb::new(16)),
            damaged: AtomicBool::new(false)
        }
    }
}

impl<U> UpdateQueue<U> {
    fn push(&self, update: SurfaceUpdate<U>) -> Result<(), SurfaceUpdate<U>> {
        let mut locked = self.pending.lock().unwrap();
        let mut existing_slot = None;
        for existing in locked.iter_mut() {
            if existing.slot == update.slot {
                existing_slot = Some(existing);
                break
            }
        }
        match existing_slot {
            Some(tgt) => {
                tgt.merge(update);
            }
            _ => {
                match locked.try_push(update) {
                    Ok(_) => self.damaged.store(true, core::sync::atomic::Ordering::Relaxed),
                    Err(e) => return Err(e)
                }
            }
        }

        Ok(())
    }

    fn try_take(&self) -> Option<HeapRb<SurfaceUpdate<U>>> {
        if self.damaged.load(core::sync::atomic::Ordering::Relaxed) {
            let mut updates = self.pending.lock().unwrap();
            let next = HeapRb::new(8);
            self.damaged.store(false, core::sync::atomic::Ordering::Relaxed);
            Some(core::mem::replace(updates.as_mut(), next))
        } else {
            None
        }
    }
}

#[derive(Default)]
struct ShaderChain<U> {
    bindings: Vec<ShaderBinding<U>>,
    updates: Arc<UpdateQueue<U>>
}

impl<U: 'static> ShaderChain<U> {
    pub fn commit(&mut self) {
        if let Some(mut queue) = self.updates.try_take() {
            for update in queue.iter_mut() {
                let target_slot = &mut self.bindings[update.slot];
                if let Some(shader) = update.shader.take() {
                    target_slot.shader = shader;
                }
                if let Some(opacity) = update.opacity.take() {
                    target_slot.opacity = opacity;
                }
                if let Some(rect) = update.rect.take() {
                    target_slot.rect = rect;
                }
                if let Some(visible) = update.visible.take() {
                    target_slot.visible = visible;
                }
            }
        }
    }

    fn new_surface(&mut self, area: Rectangle<Virtual>) -> Result<BufferedSurface<U>, ()> {
        let next_slot = self.bindings.len();
        self.bindings.push(ShaderBinding {
            opacity: 255,
            shader: None,
            rect: area,
            visible: true
        });

        Ok(BufferedSurface {
            updater: Arc::clone(&self.updates),
            slot: next_slot
        })
    }

    fn render_to<'a, S: Sample<'a>>(&self, output: &mut S, frame: &U) {
        for surface in &self.bindings {
            let opacity = surface.opacity;
            if opacity > 0 && surface.visible {
                if let Some(ref shader) = surface.shader {
                    let rect = surface.rect;
                    for (virt_coords, pixel) in output.sample(&rect) {
                        let shader_pixel = shader.draw(&virt_coords, frame);
                        if shader_pixel.r > 0 || shader_pixel.g > 0 || shader_pixel.b > 0 {
                            *pixel = pixel.blend8(shader_pixel.into(), opacity);
                        }
                    }
                }
            }
        }
    }
}

/// A thread-safe [Surfaces] implementation where changes are buffered before they are committed in batches
#[derive(Default)]
pub struct BufferedSurfacePool<U> {
    pool: RefCell<ShaderChain<U>>
}

impl<U: 'static> Surfaces for BufferedSurfacePool<U> {
    type Uniforms = U;
    type Error = ();
    type Surface = BufferedSurface<U>;

    fn new_surface(&mut self, area: super::geometry::Rectangle<super::geometry::Virtual>) -> Result<Self::Surface, Self::Error> {
        self.pool.borrow_mut().new_surface(area)
    }
}

impl<U: 'static> Renderable<U> for BufferedSurfacePool<U> {
    fn render_to<'a, S: Sample<'a>>(&self, output: &mut S, frame: &U) {
        let mut b = self.pool.borrow_mut();
        b.commit();
        b.render_to(output, frame);
    }
}

/// Types that can provide [Surface]s and render their surfaces to a [Sample]-able type
pub trait Surfaces: Send + Renderable<Self::Uniforms> {
    /// The type of uniforms supported during rendering
    type Uniforms;

    /// The underlying surface type created by this backend
    type Surface: Surface<Self::Uniforms>;

    /// Error type for operations
    type Error: Debug;

    /// Creates a new surface if possible over the given area
    fn new_surface(&mut self, area: Rectangle<Virtual>) -> Result<Self::Surface, Self::Error>;
}

/// Helper trait for allowing some [Surface] properties to be set when they are in a slice or array 
pub trait Visible {
    /// Sets the opacity of this surface, where 0 is completely transparent and 255 is completely opaque
    fn set_opacity(&mut self, opacity: u8);

    /// Sets the visibility of the surface without adjusting the stored opacity
    fn set_visible(&mut self, visible: bool);
}

impl<T: Visible> Visible for [T] {
    fn set_opacity(&mut self, opacity: u8) {
        for v in self.iter_mut() {
            v.set_opacity(opacity);
        }
    }

    fn set_visible(&mut self, visible: bool) {
        for v in self.iter_mut() {
            v.set_visible(visible);
        }
    }
}

/// Builder pattern API for creating surfaces
pub struct SurfaceBuilder<'a, S: Surface<U>, SS: Surfaces<Surface = S>, SF: Shader<U>, U> {
    surfaces: &'a mut SS,
    rect: Option<Rectangle<Virtual>>,
    opacity: Option<u8>,
    shader: Option<SF>,
    visible: Option<bool>,
    _uniform: PhantomData<U>
}

impl<'a, S: Surface<U>, SS: Surfaces<Surface = S>, SF: Shader<U>, U> SurfaceBuilder<'a, S, SS, SF, U> {
    /// Starts building a surface
    pub fn build(surfaces: &'a mut SS) -> Self {
        Self {
            surfaces,
            opacity: None,
            shader: None,
            rect: None,
            visible: None,
            _uniform: PhantomData
        }
    }

    /// Sets the initial opacity
    pub fn opacity(mut self, opacity: u8) -> Self {
        self.opacity = Some(opacity);
        self
    }

    /// Sets the initial size of the surface
    pub fn rect(mut self, rect: Rectangle<Virtual>) -> Self {
        self.rect = Some(rect);
        self
    }

    /// Sets the shader attached to the surface
    pub fn shader(mut self, shader: SF) -> Self {
        self.shader = Some(shader);
        self
    }

    /// Sets the initial visibility of the surface
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = Some(visible);
        self
    }

    /// Constructs the surface
    pub fn finish(self) -> Result<S, SS::Error> {
        let sfc = self.surfaces.new_surface(match self.rect {
            None => Rectangle::everything(),
            Some(r) => r
        });

        match sfc {
            Ok(mut s) => {
                if self.opacity.is_some() {
                    s.set_opacity(self.opacity.unwrap());
                }
                if self.shader.is_some() {
                    s.set_shader(self.shader.unwrap());
                }
                if self.visible.is_some() {
                    s.set_visible(self.visible.unwrap());
                }

                Ok(s)
            },
            err => err
        }
    }
}

/// A rectangular set of pixels that can be drawn on with a [Shader]
pub trait Surface<U>: Send + Visible {
    /// Sets the shader for this surface
    fn set_shader<T: Shader<U>>(&mut self, shader: T);

    /// Clears the shader
    fn clear_shader(&mut self);

    /// Changes the size of the surface
    fn set_rect(&mut self, rect: Rectangle<Virtual>);
}