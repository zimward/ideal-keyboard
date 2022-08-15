use bitvec::prelude::*;
use embedded_hal::digital::v2::{InputPin, OutputPin};

#[macro_export]
macro_rules! pp_output {
    ($($pin:expr),+) => {
        [$($pin.into_push_pull_output().downgrade()),+]
    };
}
#[macro_export]
macro_rules! pd_input {
    ($($pin:expr),+) => {
        [$($pin.into_pull_down_input().downgrade()),+]
    };
}

pub struct KeyMatrix<A, D, const AC: usize, const DC: usize>
where
    A: OutputPin,
    D: InputPin,
{
    address: [A; AC],
    data: [D; DC],
    offset: u8,
}

impl<A, D, const AC: usize, const DC: usize> KeyMatrix<A, D, AC, DC>
where
    A: OutputPin,
    D: InputPin,
{
    pub fn new(address_pins: [A; AC], data_pins: [D; DC], offset: u8) -> Self {
        Self {
            address: address_pins,
            data: data_pins,
            offset,
        }
    }
    pub fn scan(&mut self, keybuffer: &mut BitArr!(for 192)) {
        #[cfg(debug_assertions)]
        {
            assert!(
                keybuffer.len() > (self.offset as usize + self.address.len() * self.data.len()) * 2
            );
        }
        let address_len = self.address.len();
        for (i, addr) in self.address.iter_mut().enumerate() {
            let _ = addr.set_high();
            for (j, dat) in self.data.iter().enumerate() {
                match dat.is_high() {
                    Err(_) => {}
                    Ok(r) => {
                        if r {
                            let key = (self.offset as usize + i * address_len + j) * 2;
                            let bits = keybuffer.get_mut(key..=(key + 1)).unwrap();
                            if bits.not_any() {
                                bits.set(0, true); //key bit
                                bits.set(1, true); //change bit
                            }
                        } else {
                            let key = (self.offset as usize + i * address_len + j) * 2;
                            let bits = keybuffer.get_mut(key..=(key + 1)).unwrap();
                            if bits.any() {
                                bits.set(0, false);
                                bits.set(1, true);
                            }
                        }
                    }
                }
            }
            let _ = addr.set_low();
        }
    }
}
