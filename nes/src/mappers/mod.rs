use std::{cell::RefCell, rc::Rc};

use mos6502::memory::Bus;

use crate::cartridge::Cartridge;

mod mmc1;
mod nrom;
mod mapper3;

pub(crate) fn for_cart(cart: Cartridge) -> Rc<RefCell<dyn Bus>> {
  match cart.mapper() {
    crate::cartridge::Mapper::Nrom => Rc::new(RefCell::new(nrom::NROM::new(cart))),
    crate::cartridge::Mapper::Mmc1 => Rc::new(RefCell::new(mmc1::MMC1::new(cart))),
    crate::cartridge::Mapper::Mapper3 => Rc::new(RefCell::new(mapper3::Mapper3::new(cart))),
  }
}