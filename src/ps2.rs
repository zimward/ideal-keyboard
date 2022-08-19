use core::{convert::Infallible, usize};

use bitvec::order::Lsb0;
use bitvec::prelude::*;
use embedded_hal::digital::v2::{InputPin, StatefulOutputPin};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer, RingBufferRead};
use riscv::{
    asm::delay,
    interrupt::{disable, enable},
};

enum OperatingMode {
    Idle,
    Transmit,
    SendParity,
    SendStop,
    Receive,
}
pub struct PS2<DATA, CLOCK>
where
    DATA: InputPin<Error = Infallible> + StatefulOutputPin<Error = Infallible>,
    CLOCK: InputPin<Error = Infallible> + StatefulOutputPin<Error = Infallible>,
{
    data: DATA,
    clock: CLOCK,
    send_buffer: BitArr!(for 64,in usize,Lsb0),
    current_bit: usize,
    operation: OperatingMode,
}
impl<DATA, CLOCK> PS2<DATA, CLOCK>
where
    DATA: InputPin<Error = Infallible> + StatefulOutputPin<Error = Infallible>,
    CLOCK: InputPin<Error = Infallible> + StatefulOutputPin<Error = Infallible>,
{
    pub fn new(data: DATA, clock: CLOCK) -> Self {
        Self {
            data,
            clock,
            send_buffer: bitarr!(usize,Lsb0;0;64),
            current_bit: 0,
            operation: OperatingMode::Idle,
        }
    }
    pub fn update<const LEN: usize>(
        &mut self,
        transmit_buffer: &mut ConstGenericRingBuffer<u8, { LEN }>,
    ) {
        //const_assert_ne!(transmit_buffer.len(), 0);
        //reset clock and check for host transmission
        if self.clock.is_set_low().unwrap() {
            let _ = self.clock.set_high();
            unsafe {
                disable();
                delay(100); //wait ~ 1µS for clock to go high
                enable();
            }
            if self.clock.is_low().unwrap() {
                //Host is going to transmit
                self.operation = OperatingMode::Receive;
            }
            return; //wait till next clock
        }
        match &self.operation {
            OperatingMode::Idle => {
                //Load next scancode if present
                //send start code after successful load
                if !self.load_scancodes(transmit_buffer) {
                    return;
                }
                self.operation = OperatingMode::Transmit;
                //Transmit Start bit
                let _ = self.data.set_low();
                let _ = self.clock.set_low();
            }
            OperatingMode::Transmit => {
                self.send_bit();
            }
            OperatingMode::SendParity => {
                let current_byte = self
                    .send_buffer
                    .get(self.current_bit - 8..self.current_bit)
                    .unwrap();
                let parity = current_byte.count_ones() % 2 == 0;
                if parity {
                    let _ = self.data.set_high();
                } else {
                    let _ = self.data.set_low();
                }
                self.operation = OperatingMode::SendStop;
                let _ = self.clock.set_low();
            }
            OperatingMode::SendStop => {
                let _ = self.data.set_high();
                let _ = self.clock.set_low();
                if self
                    .send_buffer
                    .get(self.current_bit..64)
                    .unwrap()
                    .not_any()
                    || self.current_bit == 64
                {
                    self.operation = OperatingMode::Idle;
                    self.current_bit = 0;
                }
            }
            OperatingMode::Receive => {}
        }
    }
    fn load_scancodes<const LEN: usize>(
        &mut self,
        transmit_buffer: &mut ConstGenericRingBuffer<u8, { LEN }>,
    ) -> bool {
        /*        let mut new_codes = false;
        for i in 0..8 {
            if transmit_buffer.is_empty() {
                break;
            }
            new_codes = true;
            let section = self.send_buffer.get_mut(8 * i..8 * (i + 1)).unwrap();
            let next = transmit_buffer.dequeue().unwrap();
            section.store(next);
        }
        new_codes
        */
        self.send_buffer.get_mut(0..8).unwrap().store(0b1010_1010);
        true
    }
    fn send_bit(&mut self) {
        let bit = self.send_buffer.get(self.current_bit).unwrap();
        if *bit {
            let _ = self.data.set_high();
        } else {
            let _ = self.data.set_low();
        }
        self.current_bit += 1;
        if self.current_bit % 8 == 0 {
            self.operation = OperatingMode::SendParity;
        }
        let _ = self.clock.set_low();
    }
    fn receive_bit() {}
}
