#![no_std]
#![no_main]

use core::num::Wrapping;
use embedded_graphics::mono_font::iso_8859_1::FONT_6X10;
use embedded_graphics::prelude::{Primitive, Size};
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::{
    mono_font::MonoTextStyle, pixelcolor::BinaryColor, prelude::Point, text::Text, Drawable,
};
use embedded_hal::digital::v2::OutputPin;
use gd32vf103xx_hal::pac::Interrupt;
//use heapless::String;
extern crate panic_halt;
//use ufmt::uwrite;
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
use pin_defs::LedPwm;
use sh1106::{prelude::*, Builder};

mod pin_defs;

//Oled display
type Oled = sh1106::mode::GraphicsMode<
    I2cInterface<
        gd32vf103xx_hal::i2c::BlockingI2c<
            gd32vf103xx_hal::pac::I2C0,
            (PB6<Alternate<OpenDrain>>, PB7<Alternate<OpenDrain>>),
        >,
    >,
>;
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
pub struct StaticGuiElement {
    pos: Point,
    size: Size,
    text: &'static str,
    invert: bool,
}

macro_rules! StaticGuiElement {
    ($Px:tt,$Py:tt,$H:tt,$T:tt) => {
        StaticGuiElement {
            pos: Point::new($Px, $Py),
            size: Size::new(u32::try_from($T.chars().count()).unwrap() * 6 + 2, $H),
            text: $T,
            invert: false,
        }
    };
    ($Px:tt,$Py:tt,$H:tt,$T:tt,$O:tt) => {
        StaticGuiElement {
            pos: Point::new($Px + $O, $Py),
            size: Size::new(u32::try_from($T.chars().count()).unwrap() * 6 + 2, $H),
            text: $T,
            invert: false,
        }
    };
    ($Px:expr,$Py:tt,$H:tt,$T:tt) => {
        StaticGuiElement {
            pos: Point::new(i32::try_from($Px).unwrap(), $Py),
            size: Size::new(u32::try_from($T.chars().count()).unwrap() * 6 + 2, $H),
            text: $T,
            invert: false,
        }
    };
}

fn draw_gui(disp: &mut Oled, s_gui_elem: &[StaticGuiElement]) {
    //clear display
    disp.clear();
    //Rechteckfarben
    let rect_style_on = PrimitiveStyleBuilder::new()
        .stroke_color(BinaryColor::On)
        .fill_color(BinaryColor::On)
        .build();
    let rect_style_off = PrimitiveStyleBuilder::new()
        .stroke_color(BinaryColor::Off)
        .fill_color(BinaryColor::Off)
        .build();
    //fonts
    let font_off = MonoTextStyle::new(&FONT_6X10, BinaryColor::Off);
    let font_on = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    //rendering von textblöcken
    for s_elem in s_gui_elem {
        Rectangle::new(s_elem.pos, s_elem.size)
            .into_styled(match s_elem.invert {
                true => rect_style_on,
                false => rect_style_off,
            })
            .draw(disp)
            .unwrap();
        Text::new(
            s_elem.text,
            Point::new(s_elem.pos.x + 1, s_elem.pos.y + 7),
            match s_elem.invert {
                true => font_off,
                false => font_on,
            },
        )
        .draw(disp)
        .unwrap();
    }

    //flush changes to display
    disp.flush().unwrap();
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
