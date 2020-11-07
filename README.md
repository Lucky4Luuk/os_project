# os_project
An attempt at a microkernel, but honestly, it will probably fail seeing as I just got started lol.

## TODO
- [ ] Use the RDTSC reading to provide a way of more accurate sleep?
- [ ] Use the HPET to generate regular interval IRQs.
- [ ] Implement pre-emptive multitasking.
- [ ] Check if LAPIC is always mapped to the same memory location.

## In progress
- [ ] Read the ISO tables to properly route legacy interrupts.

## Recently completed
- [x] Redo the IOAPIC code to support multiple IOAPICs.

## Long term goals
- [ ] Userspace
- [ ] Some sort of GUI
