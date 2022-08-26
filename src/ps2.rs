use core::{convert::Infallible, usize};

use bitvec::order::Lsb0;
use bitvec::prelude::*;
use embedded_hal::digital::v2::{InputPin, StatefulOutputPin};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer, RingBufferRead, RingBufferWrite};

#[derive(PartialEq)]
enum OperatingMode {
    Idle,
    Transmit,
    SendParity,
    SendStop,
    ReceiveStart,
    Receive,
    ReceiveParity,
    SendAck,
    SendNack,
}
pub struct PS2<DATA, CLOCK>
where
    DATA: InputPin<Error = Infallible> + StatefulOutputPin<Error = Infallible>,
    CLOCK: InputPin<Error = Infallible> + StatefulOutputPin<Error = Infallible>,
{
    data: DATA,
    clock: CLOCK,
    send_buffer: BitArr!(for 8,in u8,Lsb0),
    receive_buffer: BitArr!(for 8,in u8,Lsb0),
    current_bit: usize,
    parity: u8,
    operation: OperatingMode,
}
impl<DATA, CLOCK> PS2<DATA, CLOCK>
where
    DATA: InputPin<Error = Infallible> + StatefulOutputPin<Error = Infallible>,
    CLOCK: InputPin<Error = Infallible> + StatefulOutputPin<Error = Infallible>,
{
    pub fn new(mut data: DATA, mut clock: CLOCK) -> Self {
        let _ = data.set_high();
        let _ = clock.set_high();
        Self {
            data,
            clock,
            send_buffer: bitarr!(u8,Lsb0;0;8),
            receive_buffer: bitarr!(u8,Lsb0;0;8),
            current_bit: 0,
            parity: 1,
            operation: OperatingMode::Idle,
        }
    }
    pub fn update<const LEN: usize>(
        &mut self,
        transmit_buffer: &mut ConstGenericRingBuffer<u8, { LEN }>,
        receive_buffer: &mut ConstGenericRingBuffer<u8, { LEN }>,
    ) {
        if self.clock.is_set_low().unwrap() {
            let _ = self.clock.set_high();
            match &self.operation {
                OperatingMode::Transmit => {
                    self.send_bit();
                }
                OperatingMode::SendParity => {
                    if self.parity == 1 {
                        let _ = self.data.set_high();
                    } else {
                        let _ = self.data.set_low();
                    }
                    self.operation = OperatingMode::SendStop;
                }
                OperatingMode::SendStop => {
                    let _ = self.data.set_high();
                }
                _ => {}
            }
        } else if self.clock.is_low().unwrap() {
            //host is going to transmit
            let _ = self.data.set_high(); //release data line
            self.operation = OperatingMode::ReceiveStart;
        } else if self.operation != OperatingMode::Idle {
            match self.operation {
                OperatingMode::SendStop => {
                    self.operation = OperatingMode::Idle;
                }
                OperatingMode::ReceiveStart => {
                    if self.data.is_low().unwrap() {
                        self.operation = OperatingMode::Receive;
                        self.current_bit = 0;
                    } else {
                        self.operation = OperatingMode::Idle;
                    }
                }
                OperatingMode::Receive => {
                    self.receive_bit();
                }
                OperatingMode::ReceiveParity => {
                    let parity = self.data.is_high().unwrap();
                    let expected_parity = self.receive_buffer.count_ones() % 2 == 0;
                    if parity == expected_parity {
                        self.operation = OperatingMode::SendAck;
                        receive_buffer.push(self.receive_buffer.as_raw_slice()[0]);
                    } else {
                        self.operation = OperatingMode::SendNack;
                    }
                }
                OperatingMode::SendAck => {
                    let _ = self.data.set_low();
                    self.operation = OperatingMode::Idle;
                }
                OperatingMode::SendNack => {
                    let _ = self.data.set_high();
                    self.operation = OperatingMode::ReceiveStart;
                }
                _ => {}
            }
            let _ = self.clock.set_low(); //generate falling edge
                                          //causing data to be read
        } else if self.load_scancodes(transmit_buffer) {
            let _ = self.data.set_low();
            let _ = self.clock.set_low();
            self.operation = OperatingMode::Transmit;
            self.current_bit = 0;
            self.parity = 1;
        }
    }
    fn load_scancodes<const LEN: usize>(
        &mut self,
        transmit_buffer: &mut ConstGenericRingBuffer<u8, { LEN }>,
    ) -> bool {
        if transmit_buffer.is_empty() {
            return false;
        }
        self.send_buffer.store(transmit_buffer.dequeue().unwrap());
        true
    }
    fn send_bit(&mut self) {
        let bit = self.send_buffer.get(self.current_bit).unwrap();
        if *bit {
            let _ = self.data.set_high();
            self.parity ^= 1;
        } else {
            let _ = self.data.set_low();
        }
        self.current_bit += 1;
        if self.current_bit == 8 {
            self.operation = OperatingMode::SendParity;
        }
    }
    fn receive_bit(&mut self) {
        if self.data.is_high().unwrap() {
            self.receive_buffer.set(self.current_bit, true);
        } else {
            self.receive_buffer.set(self.current_bit, false);
        }
        self.current_bit += 1;
        if self.current_bit == 8 {
            self.operation = OperatingMode::ReceiveParity;
        }
    }
}
