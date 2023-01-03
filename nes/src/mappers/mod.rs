use std::{cell::RefCell, rc::Rc};

use mos6502::memory::Bus;

use crate::cartridge::{Cartridge, Mirroring};

mod mmc1;
mod nrom;
mod cnrom;
mod mmc3;

pub trait Mapper : Bus {
  fn mirroring(&self) -> Mirroring;
  fn irq(&mut self) -> bool { false }
}

pub(crate) fn for_cart(cart: Cartridge) -> Rc<RefCell<dyn Mapper>> {
  match cart.mapper_type() {
    crate::cartridge::MapperType::Nrom => Rc::new(RefCell::new(nrom::NROM::new(cart))),
    crate::cartridge::MapperType::Mmc1 => Rc::new(RefCell::new(mmc1::MMC1::new(cart))),
    crate::cartridge::MapperType::Cnrom => Rc::new(RefCell::new(cnrom::CNROM::new(cart))),
    crate::cartridge::MapperType::Mmc3 => Rc::new(RefCell::new(mmc3::MMC3::new(cart))),
  }
}