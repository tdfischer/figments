use crate::prelude::*;

use spin::Mutex;
use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::sync::Arc;

use core::{marker::PhantomData, ops::DerefMut};
use core::fmt::{Debug, Formatter};
use ringbuf::{StaticRb, traits::*};
use portable_atomic::AtomicBool;

impl<U, Space: CoordinateSpace, Pixel> Debug for ShaderBinding<U, Space, Pixel> where Rectangle<Space>: Debug {
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
struct ShaderBinding<U, Space: CoordinateSpace, Pixel> {
    shader: Option<Box<dyn Shader<U, Space, Pixel>>>,
    rect: Rectangle<Space>,
    opacity: u8,
    visible: bool,
    offset: Coordinates<Space>
}

struct SurfaceUpdate<U, Space: CoordinateSpace, Pixel> {
    #[allow(clippy::type_complexity)]
    shader: Option<Option<Box<dyn Shader<U, Space, Pixel>>>>,
    rect: Option<Rectangle<Space>>,
    opacity: Option<u8>,
    visible: Option<bool>,
    offset: Option<Coordinates<Space>>,
    slot: usize,
}

type UpdateRB<U, Space, Pixel> = StaticRb<SurfaceUpdate<U, Space, Pixel>, 32>;

impl<U, Space: CoordinateSpace, Pixel> Debug for SurfaceUpdate<U, Space, Pixel> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SurfaceUpdate")
            .field("slot", &self.slot)
            .finish()
    }
}

impl<U, Space: CoordinateSpace, Pixel> SurfaceUpdate<U, Space, Pixel> {
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

impl<U, Space: CoordinateSpace, Pixel> Default for SurfaceUpdate<U, Space, Pixel> {
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
pub struct BufferedSurface<U, Space: CoordinateSpace, Pixel> {
    updater: Arc<UpdateQueue<U, Space, Pixel>>,
    slot: usize
}

impl<U, Space: CoordinateSpace, Pixel> Debug for BufferedSurface<U, Space, Pixel> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BufferedSurface").field("updater", &self.updater).field("slot", &self.slot).finish()
    }
}


impl<U, Space: CoordinateSpace, Pixel> Surface for BufferedSurface<U, Space, Pixel> {
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

impl<U, Space: CoordinateSpace, Pixel> Debug for UpdateQueue<U, Space, Pixel> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("UpdateQueue").finish()
    }
}

struct UpdateQueue<U, Space: CoordinateSpace, Pixel> {
    pending: Mutex<UpdateRB<U, Space, Pixel>>,
    damaged: AtomicBool
}

impl<U, Space: CoordinateSpace, Pixel> Default for UpdateQueue<U, Space, Pixel> {
    fn default() -> Self {
        Self {
            pending: Mutex::new(Default::default()),
            damaged: AtomicBool::new(false)
        }
    }
}

impl<U, Space: CoordinateSpace, Pixel> UpdateQueue<U, Space, Pixel> {
    fn push(&self, update: SurfaceUpdate<U, Space, Pixel>) -> Result<(), SurfaceUpdate<U, Space, Pixel>> {
        let mut locked = self.pending.lock();
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

    fn try_take(&self) -> Option<UpdateRB<U, Space, Pixel>> {
        if self.damaged.load(core::sync::atomic::Ordering::Acquire) {
            let mut updates = self.pending.lock();
            self.damaged.store(false, core::sync::atomic::Ordering::Relaxed);
            Some(core::mem::take(updates.as_mut()))
        } else {
            None
        }
    }
}

#[derive(Default)]
struct ShaderChain<U, Space: CoordinateSpace, Pixel> {
    bindings: Vec<ShaderBinding<U, Space, Pixel>>,
    updates: Arc<UpdateQueue<U, Space, Pixel>>
}

impl<U, Space: CoordinateSpace, Pixel> Debug for ShaderChain<U, Space, Pixel> where Space: Debug, Space::Data: Debug {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ShaderChain").field("bindings", &self.bindings).field("updates", &self.updates).finish()
    }
}

impl<U: 'static, Space: CoordinateSpace, Pixel> ShaderChain<U, Space, Pixel> {
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
pub struct BufferedSurfacePool<U, Space: CoordinateSpace, Pixel> {
    pool: ShaderChain<U, Space, Pixel>
}

impl<U: 'static, Space: CoordinateSpace, Pixel> BufferedSurfacePool<U, Space, Pixel> {
    /// Commits the queue of pending surface changes
    pub fn commit(&mut self) {
        self.pool.commit();
    }
}

impl<U: 'static, Space: CoordinateSpace, Pixel: Copy + Fract8Ops + 'static + Copy> Surfaces for BufferedSurfacePool<U, Space, Pixel> {
    type Error = ();
    type Surface = BufferedSurface<U, Space, Pixel>;
    
    fn new_surface(&mut self, area: Rectangle<<Self::Surface as Surface>::CoordinateSpace>) -> Result<Self::Surface, Self::Error> {
        self.pool.new_surface(area)
    }
}

impl<U: 'static, Space: CoordinateSpace + core::fmt::Debug, Pixel: 'static + Debug, HwPixel: AdditivePixelSink<Pixel> + 'static> RenderSource<U, Space, Pixel, HwPixel> for BufferedSurfacePool<U, Space, Pixel> where Space::Data: core::fmt::Debug {
    fn render_to<'a, S>(&self, output: &mut S, uniforms: &U)
        where 
            S: Sample<'a, Space, Output = HwPixel> + ?Sized {
        for surface in &self.pool.bindings {
            let opacity = surface.opacity;
            if opacity > 0 && surface.visible {
                if let Some(ref shader) = surface.shader {
                    let rect = &surface.rect;
                    for (virt_coords, output_pixel) in output.sample(rect) {
                        let adjusted = virt_coords + surface.offset;
                        let shader_pixel = shader.draw(&adjusted, uniforms);
                        output_pixel.add(shader_pixel, opacity);
                    }
                }
            }
        }
    }
}

/// Types that can provide [Surface]s and render their surfaces to a [Sample]-able type
pub trait Surfaces {
    /// The underlying surface type created by this backend
    type Surface: Surface;

    /// Error type for operations
    type Error;

    /// Creates a new surface if possible over the given area
    fn new_surface(&mut self, area: Rectangle<<Self::Surface as Surface>::CoordinateSpace>) -> Result<Self::Surface, Self::Error>;
}

/// Builder pattern API for creating surfaces
pub struct SurfaceBuilder<'a, S: Surface<Uniforms = U, Pixel = Pixel>, SS: Surfaces<Surface = S>, SF: Shader<U, <SS::Surface as Surface>::CoordinateSpace, Pixel>, U, Pixel> {
    surfaces: &'a mut SS,
    rect: Option<Rectangle<<SS::Surface as Surface>::CoordinateSpace>>,
    opacity: Option<u8>,
    shader: Option<SF>,
    visible: Option<bool>
}

impl<'a, S: Surface<Uniforms = U, Pixel = Pixel>, SS: Surfaces<Surface = S>, SF: Shader<U, S::CoordinateSpace, S::Pixel> + 'static, U, Pixel> SurfaceBuilder<'a, S, SS, SF, U, Pixel> {
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
    type Pixel;

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

    /// Sets the scroll offset of the surface without adjusting shader coordinates
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

impl<U, Space: CoordinateSpace, Pixel> Shader<U, Space, Pixel> for Box<dyn Shader<U, Space, Pixel>> {
    fn draw(&self, surface_coords: &Coordinates<Space>, uniforms: &U) -> Pixel {
        self.as_ref().draw(surface_coords, uniforms)
    }
}

pub struct NullBufferPool<U, Space: CoordinateSpace, P>(NullSurface<U, Space, P>);
pub struct NullSurface<U, Space: CoordinateSpace, P>(PhantomData<Space>, PhantomData<U>, PhantomData<P>);

impl<U: Default, Space: CoordinateSpace, P> Default for NullSurface<U, Space, P> {
    fn default() -> Self {
        Self(Default::default(), Default::default(), Default::default())
    }
}

impl<U: Default, Space: CoordinateSpace, P> Default for NullBufferPool<U, Space, P> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<U, Space: CoordinateSpace, P> core::fmt::Debug for NullBufferPool<U, Space, P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("NullBufferPool").finish()
    }
}

impl<U: Default + core::fmt::Debug, Space: CoordinateSpace, P> core::fmt::Debug for NullSurface<U, Space, P> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("NullSurface").finish()
    }
}

impl<U, Space: CoordinateSpace, P> Surface for NullSurface<U, Space, P> {
    type Uniforms = U;

    type CoordinateSpace = Space;

    type Pixel = P;

    fn set_shader<T: Shader<Self::Uniforms, Self::CoordinateSpace, Self::Pixel> + 'static>(&mut self, shader: T) {}

    fn clear_shader(&mut self) {}

    fn set_rect(&mut self, rect: Rectangle<Self::CoordinateSpace>) {}

    fn set_opacity(&mut self, opacity: u8) {}

    fn set_visible(&mut self, visible: bool) {}

    fn set_offset(&mut self, offset: Coordinates<Self::CoordinateSpace>) {}
}

impl<U: Default, Space: CoordinateSpace, P> Surfaces for NullBufferPool<U, Space, P> {
    type Surface = NullSurface<U, Space, P>;

    type Error = ();

    fn new_surface(&mut self, area: Rectangle<Space>) -> Result<Self::Surface, Self::Error> {
        Ok(NullSurface::default())
    }
}

impl<U: Default, Space: CoordinateSpace, P> RenderSource<U, Space, P, P> for NullBufferPool<U, Space, P> {
    fn render_to<'a, Smp>(&'a self, output: &'a mut Smp, uniforms: &U)
        where 
            Smp: Sample<'a, Space> + ?Sized {}
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::mappings::linear::LinearSpace;
    #[test]
    fn test_shaderchain() {
        let mut c: ShaderChain<(), LinearSpace, Rgb<u8>> = Default::default();
        assert_eq!(c.bindings.len(), 0, "Blank ShaderChain should come with zero bindings");
        c.commit();
        let mut sfc = c.new_surface(Rectangle::everything()).expect("Could not construct a new surface");
        assert_eq!(c.bindings.len(), 1, "Creating a new ShaderChain should create a new binding");
        // New surfaces should default to visible
        assert_eq!(c.bindings[0].visible, true);
        sfc.set_visible(false);
        sfc.set_opacity(128);
        // Surfaces require a commit before changes are visible on the bindings
        assert_eq!(c.bindings[0].visible, true);
        assert_eq!(c.bindings[0].opacity, 255);
        c.commit();
        // Committing changes should update bindings
        assert_eq!(c.bindings[0].visible, false);
        assert_eq!(c.bindings[0].opacity, 128);
    }

    #[test]
    fn test_bufferpool() {
        let mut pool: BufferedSurfacePool<(), LinearSpace, Rgb<u8>> = Default::default();
        let mut sfc = pool.new_surface(Rectangle::everything());
        pool.commit();
        let mut pixbuf = [Rgb::default(); 1];
        pool.render_to(&mut pixbuf[..], &());
    }
}