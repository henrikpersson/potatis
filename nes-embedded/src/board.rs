use rp2040_hal as hal;
use hal::pac as pac;

use embedded_hal::digital::v2::OutputPin;
use hal::gpio::{Pin, PushPullOutput, FunctionSpi};
use hal::gpio::bank0::{Gpio16, Gpio17};
use hal::sio::SioGpioBank0;
use hal::{Spi, spi::Enabled};
use hal::gpio::bank0::Gpio25;
use hal::gpio::PinId;

use fugit::RateExtU32;
use display_interface_spi::SPIInterface;
use embedded_hal::{blocking::delay::DelayUs, spi::MODE_0};
use st7789::ST7789;

mod all_pins {
  rp2040_hal::bsp_pins!(
      Gpio16 {
          name: lcd_dc,
          aliases: { FunctionSpi: LcdDc }
      },
      Gpio17 {
          name: lcd_cs,
          aliases: { FunctionSpi: LcdCs }
      },
      Gpio18 {
          name: spi_sclk,
          aliases: { FunctionSpi: Sclk }
      },
      Gpio19 {
          name: spi_mosi,
          aliases: { FunctionSpi: Mosi }
      },
      Gpio25 { name: led },
  );
}

pub type Screen = ST7789<
    SPIInterface<Spi<Enabled, pac::SPI0, 8>, Pin<Gpio16, PushPullOutput>, Pin<Gpio17, PushPullOutput>>,
    DummyPin,
    DummyPin
>;

pub struct Pins {
  pub led: Pin<Gpio25, <Gpio25 as PinId>::Reset>
}

pub struct Board {
  pub screen: Screen,
}

pub struct DummyPin;

impl OutputPin for DummyPin {
  type Error = ();
  fn set_high(&mut self) -> Result<(), Self::Error> {
      Ok(())
  }
  fn set_low(&mut self) -> Result<(), Self::Error> {
      Ok(())
  }
}

impl Board {
      pub fn new(
        io: pac::IO_BANK0,
        pads: pac::PADS_BANK0,
        sio: SioGpioBank0,
        spi0: pac::SPI0,
        resets: &mut pac::RESETS,
        delay: &mut impl DelayUs<u32>,
    ) -> (Self, Pins) {
      let pins = all_pins::Pins::new(io, pads, sio, resets);

      let dc = pins.lcd_dc.into_push_pull_output();
      let cs = pins.lcd_cs.into_push_pull_output();
      let sck = pins.spi_sclk.into_mode::<FunctionSpi>();
      let mosi = pins.spi_mosi.into_mode::<FunctionSpi>();

      let spi_screen = Spi::<_, _, 8>::new(spi0).init(
        resets, 
        125u32.MHz(), 
        16u32.MHz(), 
        &MODE_0
      );

      let spii_screen = SPIInterface::new(spi_screen, dc, cs);
      let mut screen = ST7789::new(
        spii_screen, 
        None,
        None,
        320, 
        240,
      );

      screen.init(delay).unwrap();
      screen
          .set_orientation(st7789::Orientation::LandscapeSwapped)
          .unwrap();

      (Self { screen }, Pins { led: pins.led })
    }
}