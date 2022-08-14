TTY:=$(shell ls /dev/ttyUSB*)

.PHONY : all, flash, clean
all :
	cargo build --release

target/riscv32imac-unknown-none-elf/release/keyboard_firmware : all
	make all

target/riscv32imac-unknown-none-elf/release/keyboard_firmware.bin : target/riscv32imac-unknown-none-elf/release/keyboard_firmware
	riscv32-unknown-elf-objcopy -O binary target/riscv32imac-unknown-none-elf/release/keyboard_firmware target/riscv32imac-unknown-none-elf/release/keyboard_firmware.bin

flash: target/riscv32imac-unknown-none-elf/release/keyboard_firmware.bin
	stm32flash -w target/riscv32imac-unknown-none-elf/release/keyboard_firmware.bin -v -g 0x0 $(TTY)
clean:
	rm -rf target
