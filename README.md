# Name
The name is the result of github name suggestions and the fact that this is a
keyboard firmware (i'm bad at naming) if someone has a better suggestion feel
free to open an issue.
# Purpose
I've always been about the lack of split ortholinear mechanical Keyboard with an
LCD Display and Linux support for programming macros. So i decided to do it my
self.

# Planned Features
* USB Interface
* optional PS/2 Interface
* Option Menu to change between layouts and macro profiles
* Turing complete Macros, probably using a custom RISC VM. Macros are going to
    be stored on an EEPROM (up to 2Mbit)
* Macros programming via PS/2 (yes it's bidirectional)  if its possible using a
    linux kernel module
* On-Keyboard Macro programming and editing

# Status

* PS/2 Interface is partially working (it works in the BIOS of my test PC but
    nowhere else), electrical protocol has been confirmed to be correct using a
    logic analyzer. My current theory is that it's not responding correctly to
    the host-commands.
* Display is working. Prototype UI's are working.
* Keymatrix scanning works flawlessly
* The Lookup from keystrokes to scan-codes is only hacked in currently. Layering
    is still missing.
* USB Interface is still missing. I'm currently studing the MCU's datasheet.

