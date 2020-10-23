# NanoRL

A Rust port of [CoreRL](https://www.roguelikeeducation.org/2.html) running on an Arduino Nano. A video can be found [here](https://youtu.be/srZrWjXdAHU) recorded on glorious potato-cam.

## Hardware

I ran this on an Arduino Nano clone with a CH430 USB-Serial chip. The display was a generic 128x64 SSD1306 OLED display over the I2C bus. For input I used four generic push-buttons, with the 328P's internal pullups enabled.

## Peripheral implementation

I only implemented the peripherals I needed to get things working (though with a complete register definition), plus the USART which I used when debugging, but is not used in the final thing. The register abstraction is complete overkill for this.

## Compiler, and Building

I used the x86-64 MSVC 2020-08-28 Nightly compiler to build this. The binary was built with the following command:

    cargo build -Z build-std=core --target avr-atmega328p.json --release

After that it was the same commands used by the Arduino IDE to upload:

    avr-objcopy -O ihex -R .eeprom .\target\avr-atmega328p\release\nano_rl.elf .\target\avr-atmega328p\release\nano_rl.hex
    avrdude -C<path/to/>/avrdude.conf -v -patmega328p -carduino -PCOM4 -b57600 -D -Uflash:w:.\target\avr-atmega328p\release\nano_rl.hex:i

## License

As the TWI and delay_microseconds implementations are based on the Arduino library those files are specifically licensed under LGPL 2.1.