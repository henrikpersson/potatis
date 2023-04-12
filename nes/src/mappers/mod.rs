use alloc::boxed::Box;
use alloc::rc::Rc;
use core::cell::RefCell;

use mos6502::memory::Bus;

use crate::cartridge::Cartridge;
use crate::cartridge::Mirroring;
use crate::cartridge::Rom;

mod cnrom;
mod mmc1;
mod mmc3;
mod nrom;
mod uxrom;

pub trait Mapper: Bus {
  fn on_runtime_mirroring(&mut self, _: Box<dyn FnMut(&Mirroring)>) {}
  fn irq(&mut self) -> bool {
    false
  }
}

pub(crate) fn for_cart<R: Rom + 'static>(cart: Cartridge<R>) -> Rc<RefCell<dyn Mapper>> {
  match cart.mapper_type() {
    crate::cartridge::MapperType::Nrom => Rc::new(RefCell::new(nrom::NROM::new(cart))),
    crate::cartridge::MapperType::Mmc1 => Rc::new(RefCell::new(mmc1::MMC1::new(cart))),
    crate::cartridge::MapperType::Uxrom => Rc::new(RefCell::new(uxrom::UxROM::new(cart))),
    crate::cartridge::MapperType::Cnrom => Rc::new(RefCell::new(cnrom::CNROM::new(cart))),
    crate::cartridge::MapperType::Mmc3 => Rc::new(RefCell::new(mmc3::MMC3::new(cart))),
  }
}
