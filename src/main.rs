#![no_std]
#![no_main]

use embedded_graphics::mono_font::iso_8859_1::FONT_6X10;
use embedded_graphics::{
    mono_font::MonoTextStyle, pixelcolor::BinaryColor, prelude::Point, text::Text, Drawable,
};
use gd32vf103xx_hal::pac::Interrupt;
use heapless::String;
extern crate panic_halt;
use ufmt::uwrite;
#[macro_use(block)]
extern crate nb;

use gd32vf103xx_hal::{
    eclic::{EclicExt, Level, LevelPriorityBits, Priority, TriggerType},
    gpio::{
        gpiob::{PB6, PB7},
        Alternate, OpenDrain,
    },
    i2c::*,
    pac,
    pac::{ECLIC, TIMER1},
    prelude::*,
    timer::{Event, Timer},
};
use sh1106::{prelude::*, Builder};
//OLED display
static mut DISPLAY: Option<
    sh1106::mode::GraphicsMode<
        I2cInterface<
            gd32vf103xx_hal::i2c::BlockingI2c<
                gd32vf103xx_hal::pac::I2C0,
                (PB6<Alternate<OpenDrain>>, PB7<Alternate<OpenDrain>>),
            >,
        >,
    >,
> = None;
static mut G_TIMER1: Option<Timer<TIMER1>> = None;

//Time
static mut TIME: u32 = 0;

fn get_millis() -> u32 {
    unsafe { TIME }
}

/*
 * Nur sicher wenn in nicht (pseudo)-Multithreaded verwendet! Auf keinen fall aus einem Interrupt
 * drauf zugreifen!
 */
fn print(msg: &str, pos: Point, font: MonoTextStyle<BinaryColor>) {
    unsafe {
        Text::new(msg, pos, font)
            .draw(DISPLAY.as_mut().unwrap())
            .unwrap();
        DISPLAY.as_mut().unwrap().flush().unwrap();
    }
}

/*
 * Nur sicher wenn in nicht (pseudo)-Multithreaded verwendet! Auf keinen fall aus einem Interrupt
 * drauf zugreifen!
 */
fn clear() {
    unsafe {
        DISPLAY.as_mut().unwrap().clear();
    }
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
        TriggerType::Level,
        Level::L0,
        Priority::P0,
    );
    unsafe { ECLIC::unmask(Interrupt::TIMER1) };
    //setup timer interrupt
    let mut _tm1 = gd32vf103xx_hal::timer::Timer::timer1(dp.TIMER1, 1.khz(), &mut rcu);
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

    let text_font: MonoTextStyle<BinaryColor> = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    /*Display*/
    unsafe {
        let mut dis: GraphicsMode<_> = Builder::new()
            .with_size(DisplaySize::Display128x64)
            .with_rotation(DisplayRotation::Rotate180)
            .connect_i2c(i2c)
            .into();

        dis.init().unwrap();
        dis.flush().unwrap();
        DISPLAY = Some(dis);
    }

    unsafe { riscv::interrupt::enable() };
    print("Init finished", Point::new(0, 12), text_font);
    loop {
        tm2.start(1.hz());
        let mut time_string = String::<10>::from("time:");
        let _ = uwrite!(time_string, "{}", get_millis() / 1000);

        clear();
        print(&time_string, Point::new(0, 24), text_font);
        block!(tm2.wait()).unwrap();
    }
}

#[allow(non_snake_case)]
#[no_mangle]
fn TIMER1() {
    unsafe {
        TIME += 1;
    }
    if let Some(ref mut timer) = unsafe { &mut G_TIMER1 } {
        timer.clear_update_interrupt_flag();
    }
}
