use rgb::Rgb;
use core::array;
use core::ops::Index;

#[cfg(feature="micromath")]
use micromath::F32Ext;

#[derive(Debug)]
pub struct GammaCurve([u8; 256]);

impl GammaCurve {
    pub fn new(gamma: f32) -> Self {
        Self(array::from_fn(|x| {
            Self::gamma_for_value(x as u8, gamma)
        }))
    }

    fn gamma_for_value(value: u8, gamma: f32) -> u8 {
        ((value as f32 / 255f32).powf(gamma) * 255f32 + 0.5) as u8
    }
}

impl Default for GammaCurve {
    fn default() -> Self {
        Self::new(1.0)
    }
}

impl Index<usize> for GammaCurve {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

pub trait WithGamma {
    fn with_gamma(self, curve: &GammaCurve) -> Self;
}

impl WithGamma for Rgb<u8> {
    fn with_gamma(self, curve: &GammaCurve)-> Self {
        Rgb::new(curve[self.r as usize], curve[self.g as usize], curve[self.b as usize])
    }
}

impl<const SIZE: usize> WithGamma for [Rgb<u8>; SIZE] {
    fn with_gamma(self, curve: &GammaCurve) -> Self {
        array::from_fn(|x| { self[x].with_gamma(curve) })
    }
}