#![doc = "Pixel buffer types"]
use std::cell::RefCell;
use std::ops::IndexMut;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crate::liber8tion::interpolate::Fract8Ops;

use super::geometry::*;
use super::render::{HardwarePixel, PixelView, Sample, Shader, Surface, Surfaces, Visible};

struct ShaderBinding {
    shader: Option<Box<dyn Shader>>,
    rect: Rectangle<Virtual>,
    opacity: u8,
    visible: bool
}

struct SurfaceUpdate {
    shader: Option<Option<Box<dyn Shader>>>,
    rect: Option<Rectangle<Virtual>>,
    opacity: Option<u8>,
    visible: Option<bool>,
    slot: usize,
}

impl SurfaceUpdate {
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

impl Default for SurfaceUpdate {
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
pub struct BufferedSurface {
    updater: Arc<UpdateQueue>,
    slot: usize
}

impl Visible for BufferedSurface {
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

impl Surface for BufferedSurface {
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

    fn set_shader<T: Shader>(&mut self, shader: T) {
        self.updater.push(SurfaceUpdate {
            shader: Some(Some(Box::new(shader))),
            slot: self.slot,
            ..Default::default()
        });
    }
}

#[derive(Default)]
struct UpdateQueue {
    pending: Mutex<Vec<SurfaceUpdate>>,
    damaged: AtomicBool
}

impl UpdateQueue {
    fn push(&self, update: SurfaceUpdate) {
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
                locked.push(update);
                self.damaged.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
}

#[derive(Default)]
struct ShaderChain {
    bindings: Vec<ShaderBinding>,
    updates: Arc<UpdateQueue>
}

impl ShaderChain {
    pub fn is_dirty(&self) -> bool {
        self.updates.damaged.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn commit(&mut self) {
        if self.is_dirty() {
            let mut queue: Vec<SurfaceUpdate> = {
                let mut updates = self.updates.pending.lock().unwrap();
                std::mem::take(updates.as_mut())
            };
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
            self.updates.damaged.store(false, std::sync::atomic::Ordering::Relaxed);
        }
    }

    fn new_surface(&mut self, area: Rectangle<Virtual>) -> Result<BufferedSurface, ()> {
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

    fn render_to<S: Sample>(&self, output: &mut S, frame: usize) {
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
pub struct BufferedSurfacePool {
    pool: RefCell<ShaderChain>
}

impl Surfaces for BufferedSurfacePool {
    type Error = ();
    type Surface = BufferedSurface;
    fn new_surface(&mut self, area: super::geometry::Rectangle<super::geometry::Virtual>) -> Result<Self::Surface, Self::Error> {
        self.pool.borrow_mut().new_surface(area)
    }

    fn render_to<S: super::render::Sample>(&self, output: &mut S, frame: usize) {
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

    /// Iterates over all the pixels in the buffer
    fn iter_with_brightness(&self, brightness: u8) -> impl Iterator<Item = Self::Pixel> + Send;

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

    fn iter_with_brightness(&self, brightness: u8) -> impl Iterator<Item=T> + Send {
        self.iter().map(move |x| { x.scale8(brightness)})
    }
}