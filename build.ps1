function build {
    cargo build -Z build-std=core --target avr-atmega328p.json --release
    avr-size -C .\target\avr-atmega328p\release\nano_rl.elf
}

function upload {
    avr-objcopy -O ihex -R .eeprom .\target\avr-atmega328p\release\nano_rl.elf .\target\avr-atmega328p\release\nano_rl.hex
    avrdude -CC:\Users\Stuart\AppData\Local\Arduino15\packages\arduino\tools\avrdude\6.3.0-arduino17/etc/avrdude.conf -v -patmega328p -carduino -PCOM4 -b57600 -D -Uflash:w:.\target\avr-atmega328p\release\nano_rl.hex:i 
}

function dump {
    avr-objdump -d .\target\avr-atmega328p\release\nano_rl.elf
}

function coms {
    d:/Programs/PuTTY/PuTTY.exe -serial com4
}