use core::{convert::Infallible, usize};

use bitvec::order::Lsb0;
use bitvec::prelude::*;
use embedded_hal::digital::v2::{InputPin, StatefulOutputPin};
use ringbuffer::{ConstGenericRingBuffer, RingBuffer, RingBufferRead, RingBufferWrite};

#[derive(PartialEq)]
enum OperatingMode {
    Idle,
    SendStart,
    Transmit,
    SendStop,
    SendStopEnd,
    Wait,
    ComInihibited,
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
        if self.clock.is_high().unwrap() && self.operation != OperatingMode::Idle {
            match self.operation {
                //Transmission
                OperatingMode::SendStart => {
                    // The Start condition needs the Data line to be low
                    // one clock before the clock goes low
                    self.operation = OperatingMode::Transmit;
                }
                OperatingMode::Transmit => {
                    self.send_bit();
                }
                OperatingMode::SendStop => {
                    let _ = self.data.set_high();
                    self.operation = OperatingMode::SendStopEnd;
                }
                OperatingMode::SendStopEnd => {
                    self.operation = OperatingMode::Wait; // Generate final edge
                }
                OperatingMode::Wait => {
                    self.current_bit += 1;
                    if self.current_bit > 4 {
                        self.operation = OperatingMode::Idle;
                    }
                    return;
                }
                //Receive
                OperatingMode::ComInihibited => {
                    //Host is still inhibiting communication
                    if self.data.is_low().unwrap() && self.data.is_set_high().unwrap() {
                        self.operation = OperatingMode::Receive;
                        self.parity = 1;
                        self.current_bit = 0;
                    } else {
                        self.operation = OperatingMode::Idle;
                        let _ = self.data.set_high();
                        let _ = self.clock.set_high();
                        return;
                    }
                }
                OperatingMode::Receive => {
                    self.receive_bit();
                }
                OperatingMode::ReceiveParity => {
                    if (self.parity == 1) == self.data.is_high().unwrap() {
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
                    self.operation = OperatingMode::Idle;
                }
                _ => {}
            }
            //Generate falling edge, causing data read
            let _ = self.clock.set_low();
        } else if self.clock.is_low().unwrap() {
            self.operation = OperatingMode::ComInihibited;
            let _ = self.data.set_high();
        } else {
            let _ = self.clock.set_high();
            let _ = self.data.set_high();
        }
        if self.operation == OperatingMode::Idle && self.load_scancodes(transmit_buffer) {
            self.operation = OperatingMode::SendStart;
            let _ = self.data.set_low();
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
        self.parity = 1;
        self.current_bit = 0;
        true
    }
    fn send_bit(&mut self) {
        // send parity
        if self.current_bit == 8 {
            if self.parity == 1 {
                let _ = self.data.set_high();
            } else {
                let _ = self.data.set_low();
            }
            self.operation = OperatingMode::SendStop;
            return;
        }
        //send data bit and compute odd parity
        let bit = self.send_buffer.get(self.current_bit).unwrap();
        if *bit {
            let _ = self.data.set_high();
            self.parity ^= 1;
        } else {
            let _ = self.data.set_low();
        }
        self.current_bit += 1;
    }
    fn receive_bit(&mut self) {
        if self.data.is_high().unwrap() {
            self.parity ^= 1;
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
