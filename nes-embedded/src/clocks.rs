use rp2040_hal as hal;

use fugit::RateExtU32;
use hal::{pac as pac, Watchdog, clocks::ClocksManager};
use hal::clocks::ClockSource;
use hal::Clock;

pub const XOSC_CRYSTAL_FREQ: u32 = 12_000_000;

pub fn configure_normal(
  xosc_dev: pac::XOSC,
  clocks_dev: pac::CLOCKS,
  pll_sys_dev: pac::PLL_SYS,
  pll_usb_dev: pac::PLL_USB,
  resets: &mut pac::RESETS,
  watchdog: &mut Watchdog,
) {
  hal::clocks::init_clocks_and_plls(
    XOSC_CRYSTAL_FREQ,
    xosc_dev,
    clocks_dev,
    pll_sys_dev,
    pll_usb_dev,
    resets,
    watchdog,
  )
  .ok()
  .unwrap();
}

pub fn configure_overclock(
  xosc_dev: pac::XOSC,
  clocks_dev: pac::CLOCKS,
  pll_sys_dev: pac::PLL_SYS,
  pll_usb_dev: pac::PLL_USB,
  resets: &mut pac::RESETS,
  watchdog: &mut Watchdog,
) -> ClocksManager {
  let xosc = hal::xosc::setup_xosc_blocking(
    xosc_dev, 
    XOSC_CRYSTAL_FREQ.Hz()
  ).unwrap();

  watchdog.enable_tick_generation((XOSC_CRYSTAL_FREQ / 1_000_000) as u8);

  let mut clocks = ClocksManager::new(clocks_dev);

  let pll_sys = hal::pll::setup_pll_blocking(
    pll_sys_dev,
    xosc.operating_frequency(),
    hal::pll::PLLConfig {
      vco_freq: fugit::HertzU32::MHz(1500),
      refdiv: 1,
      post_div1: 3,
      post_div2: 2,
    },
    &mut clocks,
    resets,
  )
  .unwrap();

  let pll_usb = hal::pll::setup_pll_blocking(
    pll_usb_dev,
    xosc.operating_frequency(),
    hal::pll::common_configs::PLL_USB_48MHZ,
    &mut clocks,
    resets,
  )
  .unwrap();

  clocks
    .system_clock
    .configure_clock(&pll_sys, pll_sys.get_freq())
    .unwrap();

  clocks.init_default(&xosc, &pll_sys, &pll_usb).unwrap();

  clocks
}