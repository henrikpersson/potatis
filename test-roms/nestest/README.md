From https://www.nesdev.org/wiki/Emulator_tests

nestest.nes:
fairly thoroughly tests CPU operation. This is the best test to start with when getting a CPU emulator working for the first time. Start execution at $C000 and compare execution with a known good log (created using Nintendulator, an emulator chosen by the test's author because its CPU was verified to function correctly, aside from some minor details of the power-up state).

instr_test-v5:
Tests official and unofficial CPU instructions and lists which ones failed. It will work even if emulator has no PPU and only supports NROM, writing a copy of output to $6000 (see readme). This more thoroughly tests instructions, but can't help you figure out what's wrong beyond what instruction(s) are failing, so it's better for testing mature CPU emulators.
