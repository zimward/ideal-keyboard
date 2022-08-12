use core::num::Wrapping;
use embedded_hal::digital::v2::{OutputPin, StatefulOutputPin};
use gd32vf103xx_hal::gpio::gpiob::PB8;
use gd32vf103xx_hal::gpio::{Active, Output, PushPull};

pub struct LedPwm {
    port: PB8<Output<PushPull>>,
    count: u8,
    thresh: u8,
}

impl LedPwm {
    pub fn new<T: Active>(port1: PB8<T>, thresh: u8) -> Self {
        Self {
            port: port1.into_push_pull_output(),
            count: 0,
            thresh,
        }
    }
    pub fn update(&mut self) {
        if self.count == 0 {
            let _ = self.port.set_low();
        }
        if self.count >= self.thresh {
            let _ = self.port.set_high();
        }
        self.count = (Wrapping(self.count) + Wrapping(1u8)).0;
    }
    pub fn set_threshold(&mut self, thresh: u8) {
        self.thresh = thresh;
    }
}
