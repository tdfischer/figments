#![no_std]

use rgb::Rgb;
use esp_hal::{Async, Blocking};
use esp_hal::dma::DmaDescriptor;
use esp_hal::spi::master::SpiDma;
use esp_hal::dma::DmaTxBuf;
use smart_leds_trait::{SmartLedsWrite, SmartLedsWriteAsync};

pub struct DmaBuffers<T, const TX_SIZE: usize> {
    pub tx_descriptors: [DmaDescriptor; 1],
    pub tx_buffer: [T; TX_SIZE]
}

impl<T: Copy, const TX_SIZE: usize> DmaBuffers<T, TX_SIZE> {
    pub const fn new(value: T) -> Self {
        Self {
            tx_descriptors: [DmaDescriptor::EMPTY; 1],
            tx_buffer: [value; TX_SIZE]
        }
    }
}

struct SpiPixelWriter<'a> {
    idx: usize,
    data: &'a mut [u8]
}

impl<'a> SpiPixelWriter<'a> {
    const fn new(data: &'a mut [u8]) -> Self {
        Self {
            idx: 0,
            data
        }
    }

    #[inline(always)]
    fn write_byte(&mut self, mut data: u8) {
        let patterns = [0b1000_1000, 0b1000_1110, 0b11101000, 0b11101110];

        if self.idx > self.data.len() - 4 {
            return;
        }
        for _ in 0..4 {
            let bits = (data & 0b1100_0000) >> 6;
            self.data[self.idx] = patterns[bits as usize];
            self.idx += 1;
            data <<= 2;
        }
    }

    fn write<T, I>(&mut self, iterator: T) -> usize
    where
        T: IntoIterator<Item = I>,
        I: Into<Rgb<u8>> {

        for pix in iterator {
            let color = pix.into();
            self.write_byte(color.g);
            self.write_byte(color.r);
            self.write_byte(color.b);
        }

        self.idx
    }
}

pub struct Esp32Ws2812SpiDmaWriter<Spi, Buffer> {
    spi: Option<Spi>,
    spi_buf: Option<Buffer>
}

impl<Spi, Buffer> Esp32Ws2812SpiDmaWriter<Spi, Buffer> {
    pub const fn new(spi: Spi, spi_buf: Buffer) -> Self {
        Self {
            spi: Some(spi),
            spi_buf: Some(spi_buf)
        }
    }
}

impl SmartLedsWrite for Esp32Ws2812SpiDmaWriter<SpiDma<'_, Blocking>, DmaTxBuf> {
    type Error = esp_hal::spi::Error;
    
    type Color = Rgb<u8>;
    
    fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color> {

        let mut spi_buf = self.spi_buf.take().unwrap();
        let mut writer = SpiPixelWriter::new(spi_buf.as_mut_slice());

        let idx = writer.write(iterator);
        spi_buf.set_length(idx);

        let spi = self.spi.take().unwrap();
        let write_result = critical_section::with(|_| {
            spi.write(idx, spi_buf)
        });
        let result = match write_result {
            Ok(r) => r.wait(),
            Err((err, spi, buf)) => {
                self.spi = Some(spi);
                self.spi_buf = Some(buf);
                return Err(err);
            }
        };
        self.spi = Some(result.0);
        self.spi_buf = Some(result.1);

        Ok(())
    }
}


impl SmartLedsWriteAsync for Esp32Ws2812SpiDmaWriter<SpiDma<'_, Blocking>, DmaTxBuf> {
    type Error = esp_hal::spi::Error;
    
    type Color = Rgb<u8>;
    
    async fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color> {

        SmartLedsWrite::write(self, iterator)
    }
}

impl SmartLedsWriteAsync for Esp32Ws2812SpiDmaWriter<SpiDma<'_, Async>, DmaTxBuf> {
    type Error = esp_hal::spi::Error;
    
    type Color = Rgb<u8>;
    
    async fn write<T, I>(&mut self, iterator: T) -> Result<(), Self::Error>
    where
        T: IntoIterator<Item = I>,
        I: Into<Self::Color> {

        let mut spi_buf = self.spi_buf.take().unwrap();
        let mut writer = SpiPixelWriter::new(spi_buf.as_mut_slice());

        let idx = writer.write(iterator);
        spi_buf.set_length(idx);

        let spi = self.spi.take().unwrap();
        let write_result = spi.write(idx, spi_buf);
        let result = match write_result {
            Ok(mut result) => {
                result.wait_for_done().await;
                result.wait()
            },
            Err((err, spi, buf)) => {
                self.spi = Some(spi);
                self.spi_buf = Some(buf);
                return Err(err);
            }
        };
        self.spi = Some(result.0);
        self.spi_buf = Some(result.1);

        Ok(())
    }
}