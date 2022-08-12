use embedded_graphics::mono_font::iso_8859_1::FONT_6X10;
use embedded_graphics::prelude::{Primitive, Size};
use embedded_graphics::primitives::{PrimitiveStyleBuilder, Rectangle};
use embedded_graphics::{
    mono_font::MonoTextStyle, pixelcolor::BinaryColor, prelude::Point, text::Text, Drawable,
};
use gd32vf103xx_hal::gpio::{
    gpiob::{PB6, PB7},
    Alternate, OpenDrain,
};
use sh1106::prelude::*;
pub struct StaticGuiElement {
    pub pos: Point,
    pub size: Size,
    pub text: &'static str,
    pub invert: bool,
}

macro_rules! StaticGuiElement {
    ($Px:tt,$Py:tt,$H:tt,$T:tt) => {
        gui::StaticGuiElement {
            pos: Point::new($Px, $Py),
            size: Size::new(u32::try_from($T.chars().count()).unwrap() * 6 + 2, $H),
            text: $T,
            invert: false,
        }
    };
    ($Px:tt,$Py:tt,$H:tt,$T:tt,$O:tt) => {
        gui::StaticGuiElement {
            pos: Point::new($Px + $O, $Py),
            size: Size::new(u32::try_from($T.chars().count()).unwrap() * 6 + 2, $H),
            text: $T,
            invert: false,
        }
    };
    ($Px:expr,$Py:tt,$H:tt,$T:tt) => {
        gui::StaticGuiElement {
            pos: Point::new(i32::try_from($Px).unwrap(), $Py),
            size: Size::new(u32::try_from($T.chars().count()).unwrap() * 6 + 2, $H),
            text: $T,
            invert: false,
        }
    };
}

//Oled display
type Oled = sh1106::mode::GraphicsMode<
    I2cInterface<
        gd32vf103xx_hal::i2c::BlockingI2c<
            gd32vf103xx_hal::pac::I2C0,
            (PB6<Alternate<OpenDrain>>, PB7<Alternate<OpenDrain>>),
        >,
    >,
>;
pub fn draw_gui(disp: &mut Oled, s_gui_elem: &[StaticGuiElement]) {
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
