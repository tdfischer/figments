#![doc = "Pixel buffer types"]
use core::cell::RefCell;
use core::ops::IndexMut;
use core::sync::atomic::AtomicBool;
use alloc::sync::Arc;
use rgb::Rgb;
use core::fmt::{Debug, Formatter};
use ringbuf::{traits::*, HeapRb};

use crate::liber8tion::interpolate::Fract8Ops;

use super::geometry::*;
use super::render::{HardwarePixel, PixelView, Sample, Shader, Surface, Surfaces, Visible};
use super::atomics::AtomicMutex;

use alloc::boxed::Box;
use alloc::vec::Vec;

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
        });
    }

    fn set_visible(&mut self, visible: bool) {
        self.updater.push(SurfaceUpdate {
            visible: Some(visible),
            slot: self.slot,
            ..Default::default()
        })
    }
}

impl<U> Surface<U> for BufferedSurface<U> {
    fn clear_shader(&mut self) {
        self.updater.push(SurfaceUpdate {
            shader: Some(None),
            slot: self.slot,
            ..Default::default()
        });
    }

    fn set_rect(&mut self, rect: Rectangle<Virtual>) {
        self.updater.push(SurfaceUpdate {
            rect: Some(rect),
            slot: self.slot,
            ..Default::default()
        });
    }

    fn set_shader<T: Shader<U>>(&mut self, shader: T) {
        self.updater.push(SurfaceUpdate {
            shader: Some(Some(Box::new(shader))),
            slot: self.slot,
            ..Default::default()
        });
    }
}

struct UpdateQueue<U> {
    pending: AtomicMutex<HeapRb<SurfaceUpdate<U>>>,
    damaged: AtomicBool
}

impl<U> Default for UpdateQueue<U> {
    fn default() -> Self {
        Self {
            pending: AtomicMutex::new(HeapRb::new(8)),
            damaged: AtomicBool::new(false)
        }
    }
}

impl<U> UpdateQueue<U> {
    fn push(&self, update: SurfaceUpdate<U>) {
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
                locked.try_push(update).unwrap();
                self.damaged.store(true, core::sync::atomic::Ordering::Relaxed);
            }
        }
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

    fn render_to<S: Sample>(&self, output: &mut S, frame: &U) {
        for surface in &self.bindings {
            let opacity = surface.opacity;
            if opacity > 0 && surface.visible {
                if let Some(ref shader) = surface.shader {
                    let rect = surface.rect;
                    let mut sample = output.sample(&rect);
                    while let Some((virt_coords, pixel)) = sample.next() {
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
    type Error = ();
    type Surface = BufferedSurface<U>;
    type Pixel = Rgb<u8>;
    type Uniforms = U;

    fn new_surface(&mut self, area: super::geometry::Rectangle<super::geometry::Virtual>) -> Result<Self::Surface, Self::Error> {
        self.pool.borrow_mut().new_surface(area)
    }

    fn render_to<S: super::render::Sample<Pixel = Self::Pixel>>(&self, output: &mut S, frame: &U) {
        let mut b = self.pool.borrow_mut();
        b.commit();
        b.render_to(output, frame);
    }
}

/// Types that provide access to a buffer of pixels, which may or may not be hardware based
/// 
/// This trait requires [IndexMut] so you can acccess individual pixels by index
pub trait Pixbuf: IndexMut<usize, Output=Self::Pixel> + Send {
    /// The underlying hardware pixel type
    type Pixel: HardwarePixel;
    /// Creates a new Pixbuf that may or may not contain default pixel values (eg, black)
    fn new() -> Self;

    /// Blanks the pixels, usually to black
    fn blank(&mut self);

    /// Returns the number of pixels accessable through this buffer
    fn pixel_count(&self) -> usize;
}

impl<T: HardwarePixel, const PIXEL_NUM: usize> Pixbuf for [T; PIXEL_NUM] {
    type Pixel = T;
    fn new() -> Self {
        [T::default(); PIXEL_NUM]
    }

    fn pixel_count(&self) -> usize {
        self.len()
    }

    fn blank(&mut self) {
        self.fill(T::default())
    }
}