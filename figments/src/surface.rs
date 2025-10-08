use crate::prelude::*;

use super::atomics::AtomicMutex;

use alloc::boxed::Box;
use alloc::vec::Vec;

use core::{ops::{Deref, DerefMut}, sync::atomic::AtomicBool};
use alloc::sync::Arc;
use core::fmt::{Debug, Formatter};
use ringbuf::{traits::*, HeapRb};

use core::cell::RefCell;

impl<U, Space: CoordinateSpace, Pixel: PixelFormat> Debug for ShaderBinding<U, Space, Pixel> where Rectangle<Space>: Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ShaderBinding")
            .field("rect", &self.rect)
            .field("opacity", &self.opacity)
            .field("shader", &match self.shader {
                None => "None",
                Some(_) => "Some(...)"
            })
            .field("visible", &self.visible)
            .finish()
    }
}

struct ShaderBinding<U, Space: CoordinateSpace, Pixel: PixelFormat> {
    shader: Option<Box<dyn Shader<U, Space, Pixel>>>,
    rect: Rectangle<Space>,
    opacity: u8,
    visible: bool,
    offset: Coordinates<Space>
}

struct SurfaceUpdate<U, Space: CoordinateSpace, Pixel: PixelFormat> {
    #[allow(clippy::type_complexity)]
    shader: Option<Option<Box<dyn Shader<U, Space, Pixel>>>>,
    rect: Option<Rectangle<Space>>,
    opacity: Option<u8>,
    visible: Option<bool>,
    offset: Option<Coordinates<Space>>,
    slot: usize,
}

impl<U, Space: CoordinateSpace, Pixel: PixelFormat> Debug for SurfaceUpdate<U, Space, Pixel> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SurfaceUpdate")
            .field("slot", &self.slot)
            .finish()
    }
}

impl<U, Space: CoordinateSpace, Pixel: PixelFormat> SurfaceUpdate<U, Space, Pixel> {
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
        if other.offset.is_some() {
            self.offset = other.offset.take()
        }
    }
}

impl<U, Space: CoordinateSpace, Pixel: PixelFormat> Default for SurfaceUpdate<U, Space, Pixel> {
    fn default() -> Self {
        SurfaceUpdate {
            shader: None,
            rect: None,
            opacity: None,
            visible: None,
            offset: None,
            slot: usize::MAX
        }
    }
}

/// A thread-safe [Surface] implementation where changes are buffered before they are committed in batches
pub struct BufferedSurface<U, Space: CoordinateSpace, Pixel: PixelFormat> {
    updater: Arc<UpdateQueue<U, Space, Pixel>>,
    slot: usize
}

impl<U, Space: CoordinateSpace, Pixel: PixelFormat> Debug for BufferedSurface<U, Space, Pixel> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BufferedSurface").field("updater", &self.updater).field("slot", &self.slot).finish()
    }
}


impl<U, Space: CoordinateSpace, Pixel: PixelFormat> Surface for BufferedSurface<U, Space, Pixel> {
    type Uniforms = U;
    type CoordinateSpace = Space;
    type Pixel = Pixel;

    fn clear_shader(&mut self) {
        self.updater.push(SurfaceUpdate {
            shader: Some(None),
            slot: self.slot,
            ..Default::default()
        }).unwrap();
    }

    fn set_rect(&mut self, rect: Rectangle<Space>) {
        self.updater.push(SurfaceUpdate {
            rect: Some(rect),
            slot: self.slot,
            ..Default::default()
        }).unwrap();
    }

    fn set_shader<T: Shader<U, Space, Pixel> + 'static>(&mut self, shader: T) {
        self.updater.push(SurfaceUpdate {
            shader: Some(Some(Box::new(shader))),
            slot: self.slot,
            ..Default::default()
        }).unwrap();
    }

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
    
    fn set_offset(&mut self, offset: Coordinates<Self::CoordinateSpace>) {
        self.updater.push(SurfaceUpdate {
            offset: Some(offset),
            slot: self.slot,
            ..Default::default()
        }).unwrap();
    }
}

impl<U, Space: CoordinateSpace, Pixel: PixelFormat> Debug for UpdateQueue<U, Space, Pixel> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("UpdateQueue").finish()
    }
}

struct UpdateQueue<U, Space: CoordinateSpace, Pixel: PixelFormat> {
    pending: AtomicMutex<HeapRb<SurfaceUpdate<U, Space, Pixel>>>,
    damaged: AtomicBool
}

impl<U, Space: CoordinateSpace, Pixel: PixelFormat> Default for UpdateQueue<U, Space, Pixel> {
    fn default() -> Self {
        Self {
            pending: AtomicMutex::new(HeapRb::new(128)),
            damaged: AtomicBool::new(false)
        }
    }
}

impl<U, Space: CoordinateSpace, Pixel: PixelFormat> UpdateQueue<U, Space, Pixel> {
    fn push(&self, update: SurfaceUpdate<U, Space, Pixel>) -> Result<(), SurfaceUpdate<U, Space, Pixel>> {
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

    fn try_take(&self) -> Option<HeapRb<SurfaceUpdate<U, Space, Pixel>>> {
        if self.damaged.load(core::sync::atomic::Ordering::Relaxed) {
            let mut updates = self.pending.lock().unwrap();
            let next = HeapRb::new(updates.capacity().into());
            self.damaged.store(false, core::sync::atomic::Ordering::Relaxed);
            Some(core::mem::replace(updates.as_mut(), next))
        } else {
            None
        }
    }
}

#[derive(Default)]
struct ShaderChain<U, Space: CoordinateSpace, Pixel: PixelFormat> {
    bindings: Vec<ShaderBinding<U, Space, Pixel>>,
    updates: Arc<UpdateQueue<U, Space, Pixel>>
}

impl<U, Space: CoordinateSpace, Pixel: PixelFormat> Debug for ShaderChain<U, Space, Pixel> where Space: Debug, Space::Data: Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ShaderChain").field("bindings", &self.bindings).field("updates", &self.updates).finish()
    }
}

impl<U: 'static, Space: CoordinateSpace, Pixel: PixelFormat> ShaderChain<U, Space, Pixel> {
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

    fn new_surface(&mut self, area: Rectangle<Space>) -> Result<BufferedSurface<U, Space, Pixel>, ()> {
        let next_slot = self.bindings.len();
        self.bindings.push(ShaderBinding {
            opacity: 255,
            shader: None,
            rect: area,
            visible: true,
            offset: Coordinates::top_left()
        });

        Ok(BufferedSurface {
            updater: Arc::clone(&self.updates),
            slot: next_slot
        })
    }
}

/// A thread-safe [Surfaces] implementation where changes are buffered before they are committed in batches
#[derive(Default)]
pub struct BufferedSurfacePool<U, Space: CoordinateSpace, Pixel: PixelFormat> {
    pool: RefCell<ShaderChain<U, Space, Pixel>>
}

impl<U: 'static, Space: CoordinateSpace, Pixel: PixelFormat> BufferedSurfacePool<U, Space, Pixel> {
    pub fn commit(&self) {
        let mut b = self.pool.borrow_mut();
        b.commit();
    }
}

impl<U: 'static, Space: CoordinateSpace, Pixel: PixelFormat + 'static> Surfaces<Space> for BufferedSurfacePool<U, Space, Pixel> {
    type Error = ();
    type Surface = BufferedSurface<U, Space, Pixel>;
    
    fn new_surface(&mut self, area: Rectangle<<Self::Surface as Surface>::CoordinateSpace>) -> Result<Self::Surface, Self::Error> {
        self.pool.borrow_mut().new_surface(area)
    }
    
    fn render_to<'a, S>(&self, output: &mut S, uniforms: &<Self::Surface as Surface>::Uniforms)
        where 
            S: Sample<'a, Space>,
            S::Output: PixelBlend<<Self::Surface as Surface>::Pixel> {
        let mut b = self.pool.borrow_mut();
        b.commit();
        for surface in &b.bindings {
            let opacity = surface.opacity;
            if opacity > 0 && surface.visible {
                if let Some(ref shader) = surface.shader {
                    let rect = &surface.rect;
                    for (virt_coords, output_pixel) in output.sample(rect) {
                        let adjusted = virt_coords + surface.offset;
                        let shader_pixel = shader.draw(&adjusted, uniforms);
                        *output_pixel = output_pixel.blend_pixel(shader_pixel, opacity);
                    }
                }
            }
        }
    }
}

/// Types that can provide [Surface]s and render their surfaces to a [Sample]-able type
pub trait Surfaces<Space: CoordinateSpace> {
    /// The underlying surface type created by this backend
    type Surface: Surface<CoordinateSpace = Space>;

    /// Error type for operations
    type Error;

    /// Creates a new surface if possible over the given area
    fn new_surface(&mut self, area: Rectangle<Space>) -> Result<Self::Surface, Self::Error>;

    fn render_to<'a, S>(&self, output: &mut S, uniforms: &<Self::Surface as Surface>::Uniforms)
        where 
            S: Sample<'a, Space>,
            S::Output: PixelBlend<<Self::Surface as Surface>::Pixel>;
}

/// Builder pattern API for creating surfaces
pub struct SurfaceBuilder<'a, S: Surface<Uniforms = U, Pixel = Pixel>, SS: Surfaces<S::CoordinateSpace, Surface = S>, SF: Shader<U, <SS::Surface as Surface>::CoordinateSpace, Pixel>, U, Pixel: PixelFormat> {
    surfaces: &'a mut SS,
    rect: Option<Rectangle<<SS::Surface as Surface>::CoordinateSpace>>,
    opacity: Option<u8>,
    shader: Option<SF>,
    visible: Option<bool>
}

impl<'a, S: Surface<Uniforms = U, Pixel = Pixel>, SS: Surfaces<S::CoordinateSpace, Surface = S>, SF: Shader<U, S::CoordinateSpace, S::Pixel> + 'static, U, Pixel: PixelFormat> SurfaceBuilder<'a, S, SS, SF, U, Pixel> {
    /// Starts building a surface
    pub fn build(surfaces: &'a mut SS) -> Self {
        Self {
            surfaces,
            opacity: None,
            shader: None,
            rect: None,
            visible: None
        }
    }

    /// Sets the initial opacity
    pub fn opacity(mut self, opacity: u8) -> Self {
        self.opacity = Some(opacity);
        self
    }

    /// Sets the initial size of the surface
    pub fn rect(mut self, rect: Rectangle<<SS::Surface as Surface>::CoordinateSpace>) -> Self {
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
    pub fn finish(self) -> Result<SS::Surface, SS::Error> {
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
pub trait Surface {
    /// The type of uniform data that is supported by this shader
    type Uniforms;

    /// The coordiante space over which this shader can operate
    type CoordinateSpace: CoordinateSpace;

    /// The format of the pixels produced by this shader
    type Pixel: PixelFormat;

    /// Sets the shader for this surface
    fn set_shader<T: Shader<Self::Uniforms, Self::CoordinateSpace, Self::Pixel> + 'static>(&mut self, shader: T);

    /// Clears the shader
    fn clear_shader(&mut self);

    /// Changes the size of the surface
    fn set_rect(&mut self, rect: Rectangle<Self::CoordinateSpace>);

    /// Sets the opacity of this surface, where 0 is completely transparent and 255 is completely opaque
    fn set_opacity(&mut self, opacity: u8);

    /// Sets the visibility of the surface without adjusting the stored opacity
    fn set_visible(&mut self, visible: bool);

    fn set_offset(&mut self, offset: Coordinates<Self::CoordinateSpace>);
}

impl<T: DerefMut<Target = S>, S: Surface> Surface for [T] {
    type Uniforms = S::Uniforms;

    type CoordinateSpace = S::CoordinateSpace;

    type Pixel = S::Pixel;

    fn set_shader<SH: Shader<Self::Uniforms, Self::CoordinateSpace, Self::Pixel> + 'static>(&mut self, _shader: SH) {
        unimplemented!();
    }

    fn clear_shader(&mut self) {
        self.iter_mut().for_each(|f| { f.clear_shader(); });
    }

    fn set_rect(&mut self, rect: Rectangle<Self::CoordinateSpace>) {
        self.iter_mut().for_each(|f| { f.set_rect(rect); });
    }

    fn set_opacity(&mut self, opacity: u8) {
        self.iter_mut().for_each(|f| { f.set_opacity(opacity); });
    }

    fn set_visible(&mut self, visible: bool) {
        self.iter_mut().for_each(|f| { f.set_visible(visible); });
    }
    
    fn set_offset(&mut self, offset: Coordinates<Self::CoordinateSpace>) {
        self.iter_mut().for_each(|f| { f.set_offset(offset); });
    }
}

impl<U, Space: CoordinateSpace, Pixel: PixelFormat> Shader<U, Space, Pixel> for Box<dyn Shader<U, Space, Pixel>> {
    fn draw(&self, surface_coords: &Coordinates<Space>, uniforms: &U) -> Pixel {
        self.as_ref().draw(surface_coords, uniforms)
    }
}