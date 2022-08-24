use crate::ps2::PS2;
use crate::{keyboard_layouts::SCANCODE_LOOKUP, sprintln};
use bitvec::prelude::*;
use core::convert::Infallible;
use embedded_hal::digital::v2::{InputPin, OutputPin, StatefulOutputPin};
use ringbuffer::{ConstGenericRingBuffer, RingBufferWrite};
use riscv::asm::delay;

pub struct Keyboard<M, const MC: usize, Ps2Data, Ps2Clock>
where
    M: ScanableMatrix,
    Ps2Data: InputPin<Error = Infallible> + StatefulOutputPin<Error = Infallible>,
    Ps2Clock: InputPin<Error = Infallible> + StatefulOutputPin<Error = Infallible>,
{
    matricies: [M; MC],
    ps2_interface: PS2<Ps2Data, Ps2Clock>,
    key_buffer: BitArr!(for 192),
    scancode_buffer: ConstGenericRingBuffer<u8, 32>,
    command_buffer: ConstGenericRingBuffer<u8, 32>,
}

impl<M, const MC: usize, Ps2Data, Ps2Clock> Keyboard<M, MC, Ps2Data, Ps2Clock>
where
    M: ScanableMatrix,
    Ps2Data: InputPin<Error = Infallible> + StatefulOutputPin<Error = Infallible>,
    Ps2Clock: InputPin<Error = Infallible> + StatefulOutputPin<Error = Infallible>,
{
    pub fn new(matricies: [M; MC], ps2_data: Ps2Data, ps2_clock: Ps2Clock) -> Self {
        Self {
            matricies,
            key_buffer: bitarr!(usize,Lsb0;0;192),
            scancode_buffer: ConstGenericRingBuffer::new(),
            command_buffer: ConstGenericRingBuffer::new(),
            ps2_interface: PS2::new(ps2_data, ps2_clock),
        }
    }
    pub fn scan(&mut self) {
        for matrix in &mut self.matricies {
            matrix.scan(&mut self.key_buffer);
        }
    }
    pub fn process_keystrokes(&mut self) {
        for i in (0..self.key_buffer.len()).step_by(2) {
            let val = self.key_buffer.get_mut(i..=i + 1).unwrap();
            // read change bit
            if *val.get(1).unwrap() {
                let key = i / 2;
                if key >= SCANCODE_LOOKUP.len() {
                    continue;
                }
                if *val.get(0).unwrap() {
                    for code in SCANCODE_LOOKUP[key] {
                        self.scancode_buffer.push(*code);
                    }
                    sprintln!("Key {} pressed", key);
                } else {
                    if SCANCODE_LOOKUP[key].len() > 1 {
                        //Extended code
                        self.scancode_buffer.push(0xE0);
                        self.scancode_buffer.push(0xF0);
                        for code in &SCANCODE_LOOKUP[key][1..] {
                            self.scancode_buffer.push(*code);
                        }
                    } else {
                    }
                    self.scancode_buffer.push(0xF0); //break code
                    self.scancode_buffer.push(SCANCODE_LOOKUP[key][0]);
                    sprintln!("Key {} released", key);
                }
                val.set(1, false);
            }
        }
    }
    pub fn update_interface(&mut self) {
        self.ps2_interface
            .update(&mut self.scancode_buffer, &mut self.command_buffer);
    }
}

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

pub trait ScanableMatrix {
    fn scan(&mut self, keybuffer: &mut BitArr!(for 192));
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
}
impl<A, D, const AC: usize, const DC: usize> ScanableMatrix for KeyMatrix<A, D, AC, DC>
where
    A: OutputPin,
    D: InputPin,
{
    fn scan(&mut self, keybuffer: &mut BitArr!(for 192)) {
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
                        let key = (self.offset as usize + i * address_len + j) * 2;
                        let bits = keybuffer.get_mut(key..=(key + 1)).unwrap();
                        if r {
                            //key wasn't already pressed
                            if !*bits.get(0).unwrap() {
                                bits.set(0, true); //key bit
                                bits.set(1, true); //change bit
                            }
                        } else if *bits.get(0).unwrap() {
                            //key was pressed before
                            bits.set(0, false);
                            bits.set(1, true);
                        }
                    }
                }
            }
            let _ = addr.set_low();
            unsafe {
                delay(100); //wait for diode capacitance to discharge ~1µS should be sufficient, if
                            //not a keypress will register as the current and following key.
            }
        }
    }
}
