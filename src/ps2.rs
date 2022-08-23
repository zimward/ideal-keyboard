use core::{convert::Infallible, usize};

use bitvec::order::Lsb0;
use bitvec::prelude::*;
use embedded_hal::digital::v2::{InputPin, StatefulOutputPin};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer, RingBufferRead, RingBufferWrite};

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
            receive_buffer: bitarr!(u8,Lsb0;0,8),
            current_bit: 0,
            operation: OperatingMode::Idle,
        }
    }
    pub fn update<const LEN: usize>(
        &mut self,
        transmit_buffer: &mut ConstGenericRingBuffer<u8, { LEN }>,
        receive_buffer: &mut ConstGenericRingBuffer<u8, { LEN }>,
    ) {
        //const_assert_ne!(transmit_buffer.len(), 0);
        //reset clock and check for host transmission
        if self.clock.is_set_low().unwrap() {
            let _ = self.clock.set_high();
            return; //wait till next clock
        }
        if self.clock.is_low().unwrap() {
            //Host is going to transmit
            let _ = self.data.set_high(); //release data
            self.operation = OperatingMode::ReceiveStart;
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
                if self.current_bit == 8 {
                    self.operation = OperatingMode::Idle;
                    self.current_bit = 0;
                }
            }
            OperatingMode::ReceiveStart => {
                if self.data.is_low().unwrap() {
                    if self.clock.is_high().unwrap() {
                        self.operation = OperatingMode::Receive;
                        let _ = self.clock.set_low();
                        self.current_bit = 0;
                    }
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
                let _ = self.clock.set_low();
            }
            OperatingMode::SendAck => {
                self.send_ack(true);
            }
            OperatingMode::SendNack => {
                self.send_ack(false);
            }
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
        } else {
            let _ = self.data.set_low();
        }
        self.current_bit += 1;
        if self.current_bit % 8 == 0 {
            self.operation = OperatingMode::SendParity;
        }
        let _ = self.clock.set_low();
    }
    fn receive_bit(&mut self) {
        if self.data.is_high().unwrap() {
            self.receive_buffer.set(self.current_bit, true);
        } else {
            self.receive_buffer.set(self.current_bit, false);
        }
        let _ = self.clock.set_low();
        self.current_bit += 1;
        if self.current_bit == 8 {
            self.operation = OperatingMode::ReceiveParity;
        }
    }
    fn send_ack(&mut self, ack: bool) {
        if ack {
            let _ = self.data.set_low();
            self.operation = OperatingMode::Idle;
        } else {
            let _ = self.data.set_high();
            self.current_bit = 0;
            self.operation = OperatingMode::Receive;
        }
        let _ = self.clock.set_low();
    }
}
