#![no_std]
#![no_main]

use core::num::Wrapping;
use embedded_graphics::prelude::Point;
use embedded_graphics::prelude::Size;
use gd32vf103xx_hal::pac::Interrupt;
use heapless::String;
extern crate panic_halt;
#[macro_use(block)]
extern crate nb;

use gd32vf103xx_hal::{
    eclic::{EclicExt, Level, LevelPriorityBits, Priority, TriggerType},
    gpio::{
        gpiob::{PB0, PB1},
        Input, OpenDrain, Output, PullDown, PushPull, Pxx,
    },
    i2c::*,
    pac,
    pac::{ECLIC, TIMER1, TIMER2, TIMER3},
    prelude::*,
    timer::{Event, Timer},
};
//use ringbuffer::ConstGenericRingBuffer;
use sh1106::{prelude::*, Builder};

#[macro_use]
mod gui;
mod keyboard;
mod keyboard_layouts;
mod pin_defs;
mod ps2;
mod stdout;
use keyboard::*;
use pin_defs::*;

static mut G_TIMER2: Option<Timer<TIMER2>> = None;
static mut G_TIMER1: Option<Timer<TIMER1>> = None;
static mut G_TIMER3: Option<Timer<TIMER3>> = None;
//Time
static mut TIME: u32 = 0;
//LEDPWM
static mut LED_PWM: Option<LedPwm> = None;
type KB = Keyboard<
    keyboard::KeyMatrix<Pxx<Output<PushPull>>, Pxx<Input<PullDown>>, 7_usize, 7_usize>,
    1_usize,
    PB0<Output<OpenDrain>>,
    PB1<Output<OpenDrain>>,
>;
static mut KEYBOARD: Option<KB> = None;
//Time overflow after ~119,3h
#[allow(dead_code)]
fn get_millis() -> u32 {
    unsafe { TIME }
}

#[allow(non_snake_case)]
#[riscv_rt::entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let mut rcu = dp
        .RCU
        .configure()
        .ext_hf_clock(8.mhz())
        .sysclk(108.mhz())
        .freeze();

    {
        setup_interrupts();
        let mut _tm2 = Timer::timer2(dp.TIMER2, 30.khz(), &mut rcu);
        _tm2.listen(Event::Update);
        unsafe { G_TIMER2 = Some(_tm2) };

        /*let mut _tm1 = Timer::timer1(dp.TIMER1, 10.khz(), &mut rcu);
        _tm1.listen(Event::Update);
        unsafe { G_TIMER1 = Some(_tm1) };*/

        let mut _tm3 = Timer::timer3(dp.TIMER3, 1.khz(), &mut rcu);
        _tm3.listen(Event::Update);
        unsafe { G_TIMER3 = Some(_tm3) };
    }
    let mut tm4 = Timer::timer4(dp.TIMER4, 1.khz(), &mut rcu);

    let gpioa = dp.GPIOA.split(&mut rcu);
    let gpiob = dp.GPIOB.split(&mut rcu);
    let mut afio = dp.AFIO.constrain(&mut rcu);

    unsafe {
        LED_PWM = Some(LedPwm::new(gpiob.pb8, 220));
    }
    unsafe {
        let right_kb = KeyMatrix::new(
            pp_output!(gpioa.pa0, gpioa.pa1, gpioa.pa2, gpioa.pa3, gpioa.pa4, gpioa.pa5, gpioa.pa6),
            pd_input!(
                gpiob.pb9, gpiob.pb10, gpiob.pb11, gpiob.pb12, gpiob.pb13, gpiob.pb14, gpiob.pb15
            ),
            0,
        );
        KEYBOARD = Some(Keyboard::new(
            [right_kb],
            gpiob.pb0.into_open_drain_output(),
            gpiob.pb1.into_open_drain_output(),
        ));
    }

    crate::stdout::configure(
        dp.USART0,
        gpioa.pa9,
        gpioa.pa10,
        115_200.bps(),
        &mut afio,
        &mut rcu,
    );
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

    let tab1 = StaticGuiElement!(0, 0, 9, String::<16>::from("Macro"));
    let tab2 = StaticGuiElement!(tab1.size.width + 5, 0, 9, String::<16>::from("DVORAK"));
    let tab3 = StaticGuiElement!(
        u32::try_from(tab2.pos.x).unwrap() + tab2.size.width + 5,
        0,
        9,
        String::<16>::from("Menü")
    );
    let _static_gui_elem = [tab1, tab2, tab3];
    unsafe { riscv::interrupt::enable() };
    tm4.start(1.khz());
    let mut last = get_millis();
    loop {
        //draw_gui(&mut disp, &static_gui_elem);
        let start = get_millis();
        unsafe {
            KEYBOARD.as_mut().unwrap().process_keystrokes();
        }
        if last + 1_000 <= get_millis() {
            sprintln!("Processing Time:{}", get_millis() - start);
            last = get_millis();
        }
        block!(tm4.wait()).unwrap();
    }
}

fn setup_interrupts() {
    //ECLIC setup
    ECLIC::reset();
    ECLIC::set_threshold_level(Level::L0);
    //2 Bits level 2 Bit priority
    ECLIC::set_level_priority_bits(LevelPriorityBits::L2P2);

    //Setup of interrupts
    /*ECLIC::setup(
        Interrupt::TIMER1,
        TriggerType::RisingEdge,
        Level::L0,
        Priority::P0,
    );*/
    ECLIC::setup(
        Interrupt::TIMER2,
        TriggerType::RisingEdge,
        Level::L0,
        Priority::P0,
    );
    ECLIC::setup(
        Interrupt::TIMER3,
        TriggerType::RisingEdge,
        Level::L0,
        Priority::P1,
    );

    //  unsafe { ECLIC::unmask(Interrupt::TIMER1) };
    unsafe { ECLIC::unmask(Interrupt::TIMER2) };
    unsafe { ECLIC::unmask(Interrupt::TIMER3) };
}

#[allow(non_snake_case)]
#[no_mangle]
fn TIMER2() {
    if let Some(ref mut timer) = unsafe { &mut G_TIMER2 } {
        timer.clear_update_interrupt_flag();
    }
    unsafe {
        KEYBOARD.as_mut().unwrap().update_interface();
        LED_PWM.as_mut().unwrap().update();
    }
}
/*
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
}*/

#[allow(non_snake_case)]
#[no_mangle]
fn TIMER3() {
    if let Some(ref mut timer) = unsafe { &mut G_TIMER3 } {
        timer.clear_update_interrupt_flag();
    }
    unsafe {
        TIME = (Wrapping(TIME) + Wrapping(1u32)).0;
        KEYBOARD.as_mut().unwrap().scan();
    }
}
