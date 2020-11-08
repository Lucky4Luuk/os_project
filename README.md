# os_project
An attempt at a microkernel, but honestly, it will probably fail seeing as I just got started lol.

## TODO
- [ ] Use the RDTSC reading to provide a way of more accurate sleep?
- [ ] Implement pre-emptive multitasking.
- [ ] Check if LAPIC is always mapped to the same memory location.

## In progress
- [ ] Use the HPET to generate regular interval IRQs.

## Recently completed
- [x] Read the ISO tables to properly route legacy interrupts.
- [x] Redo the IOAPIC code to support multiple IOAPICs.

## Long term goals
- [ ] Userspace
- [ ] Some sort of GUI

## Notes on various topics
My brain is a mess, so this exists to keep notes.
### ISO tables
The InterruptIndex table I have right now is basically useless.
The number seems to only be used to set up the index in the IDT and then it just gets passed as the vector. I might as well map it out myself, I reckon. So far, assigning a random number instead of whatever number I got from my old PIC code seems to work 100% fine.

### HPET
Just use it in legacy mode. It's much easier to map to IRQ that way.
