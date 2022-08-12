#![no_std]
#![no_main]

use core::num::Wrapping;
use embedded_graphics::prelude::Point;
use embedded_graphics::prelude::Size;
use gd32vf103xx_hal::pac::Interrupt;
//use heapless::String;
extern crate panic_halt;
//use ufmt::uwrite;
#[macro_use(block)]
extern crate nb;

use gd32vf103xx_hal::{
    eclic::{EclicExt, Level, LevelPriorityBits, Priority, TriggerType},
    i2c::*,
    pac,
    pac::{ECLIC, TIMER1},
    prelude::*,
    timer::{Event, Timer},
};
use pin_defs::LedPwm;
use sh1106::{prelude::*, Builder};

#[macro_use]
mod gui;
mod pin_defs;
use gui::draw_gui;

static mut G_TIMER1: Option<Timer<TIMER1>> = None;
//Time
static mut TIME: u32 = 0;
//LEDPWM
static mut LED_PWM: Option<LedPwm> = None;

//Zeit überlauf nach ~119,3 Stunden
fn get_millis() -> u32 {
    //Operation ist sicher, da eine Kopie erstellt wird und eine differenz von 1ms nicht
    //dramatisch ist
    unsafe { TIME }
}

#[riscv_rt::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let mut rcu = dp
        .RCU
        .configure()
        .ext_hf_clock(8.mhz())
        .sysclk(108.mhz())
        .freeze();
    let gpiob = dp.GPIOB.split(&mut rcu);
    let mut afio = dp.AFIO.constrain(&mut rcu);

    //ECLIC setup
    ECLIC::reset();
    ECLIC::set_threshold_level(Level::L0);
    //3 Bits level 1 Bit priority
    ECLIC::set_level_priority_bits(LevelPriorityBits::L3P1);

    //Setup of interrupts
    ECLIC::setup(
        Interrupt::TIMER1,
        TriggerType::RisingEdge,
        Level::L7,
        Priority::P0,
    );
    unsafe { ECLIC::unmask(Interrupt::TIMER1) };
    unsafe {
        LED_PWM = Some(LedPwm::new(gpiob.pb8, 0));
    }

    //set up timer interrupt
    let mut _tm1 = gd32vf103xx_hal::timer::Timer::timer1(dp.TIMER1, 10.khz(), &mut rcu);
    _tm1.listen(Event::Update);
    unsafe { G_TIMER1 = Some(_tm1) };

    let mut tm2 = gd32vf103xx_hal::timer::Timer::timer2(dp.TIMER2, 1.hz(), &mut rcu);

    /*I2C0 interface*/
    let scl = gpiob.pb6.into_alternate_open_drain();
    let sda = gpiob.pb7.into_alternate_open_drain();
    let i2c = BlockingI2c::i2c0(
        dp.I2C0,
        (scl, sda),
        &mut afio,
        Mode::Standard {
            frequency: 100.khz().into(),
        },
        &mut rcu,
        998,
        1,
        998,
        998,
    );

    /*Display*/
    let mut disp: GraphicsMode<_> = Builder::new()
        .with_size(DisplaySize::Display128x64)
        .with_rotation(DisplayRotation::Rotate180)
        .connect_i2c(i2c)
        .into();

    disp.init().unwrap();
    disp.flush().unwrap();

    let mut index: usize = 0;
    let tab1 = StaticGuiElement!(0, 0, 9, "Macro");
    let tab2 = StaticGuiElement!(tab1.size.width + 5, 0, 9, "DVORAK");
    let tab3 = StaticGuiElement!(
        u32::try_from(tab2.pos.x).unwrap() + tab2.size.width + 5,
        0,
        9,
        "Menü"
    );
    let mut static_gui_elem = [tab1, tab2, tab3];
    unsafe { riscv::interrupt::enable() };
    let mut t: u8 = 0;
    loop {
        tm2.start(1.hz());
        for mut tab in static_gui_elem.as_mut() {
            tab.invert = false;
        }
        static_gui_elem[index].invert = true;
        index += 1;
        index %= static_gui_elem.len();
        draw_gui(&mut disp, &static_gui_elem);
        t = (Wrapping(t) + Wrapping(20u8)).0;
        unsafe {
            LED_PWM.as_mut().unwrap().set_threshold(t);
        }
        block!(tm2.wait()).unwrap();
    }
}

#[allow(non_snake_case)]
#[no_mangle]
fn TIMER1() {
    if let Some(ref mut timer) = unsafe { &mut G_TIMER1 } {
        timer.clear_update_interrupt_flag();
    }
    unsafe {
        TIME = (Wrapping(TIME) + Wrapping(1u32)).0;
        LED_PWM.as_mut().unwrap().update();
    }
}
