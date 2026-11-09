// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! DMG and CGB boot ROM emulation.
//!
//! Provides the initial register state and memory contents that a real
//! Game Boy would have after executing its boot ROM.

use crate::cpu::Registers;
use crate::machine::Machine;

/// Result of boot process: initialized registers and memory state.
#[derive(Clone, Debug)]
pub struct BootState {
    pub registers: Registers,
    pub ime: bool,
    pub halted: bool,
    /// Memory initialization: (address, value) pairs to write
    pub memory_init: Vec<(u16, u8)>,
}

impl BootState {
    /// Create initial state for DMG boot.
    pub fn dmg() -> Self {
        let mut registers = Registers::default();
        registers.set_af(0x01B0);
        registers.set_bc(0x0013);
        registers.set_de(0x00D8);
        registers.set_hl(0x014D);
        registers.sp = 0xFFFE;
        registers.pc = 0x0100;
        
        let mut memory_init = vec![
            // Timer registers
            (0xFF05, 0x00), // TIMA
            (0xFF06, 0x00), // TMA
            (0xFF07, 0x00), // TAC
            // APU registers
            (0xFF10, 0x80), // NR10
            (0xFF11, 0xBF), // NR11
            (0xFF12, 0xF3), // NR12
            (0xFF14, 0xBF), // NR14
            (0xFF16, 0x3F), // NR21
            (0xFF17, 0x00), // NR22
            (0xFF19, 0xBF), // NR24
            (0xFF1A, 0x7F), // NR30
            (0xFF1B, 0xFF), // NR31
            (0xFF1C, 0x9F), // NR32
            (0xFF1E, 0xBF), // NR33
            (0xFF20, 0xFF), // NR41
            (0xFF21, 0x00), // NR42
            (0xFF22, 0x00), // NR43
            (0xFF23, 0xBF), // NR44
            (0xFF24, 0x77), // NR50
            (0xFF25, 0xF3), // NR51
            (0xFF26, 0xF1), // NR52 (power on)
            // LCD registers
            (0xFF40, 0x91), // LCDC
            (0xFF42, 0x00), // SCY
            (0xFF43, 0x00), // SCX
            (0xFF45, 0x00), // LYC
            (0xFF47, 0xFC), // BGP
            (0xFF48, 0xFF), // OBP0
            (0xFF49, 0xFF), // OBP1
            (0xFF4A, 0x00), // WY
            (0xFF4B, 0x00), // WX
            // Interrupt enable
            (0xFFFF, 0x00), // IE
        ];
        
        // Initialize WRAM with DMG boot pattern
        for i in 0xC000..0xDFFF {
            let val = match (i % 0x1000) / 0x100 {
                0 | 2 => 0x00,
                1 | 3 => 0xFF,
                _ => 0x00,
            };
            memory_init.push((i, val));
        }
        
        // Initialize HRAM
        for i in 0xFF80..0xFFFF {
            memory_init.push((i, 0x00));
        }
        
        // Initialize OAM
        for i in 0xFE00..0xFE9F {
            memory_init.push((i, 0x00));
        }
        
        Self {
            registers,
            ime: true,
            halted: false,
            memory_init,
        }
    }
    
    /// Create initial state for CGB boot.
    pub fn cgb() -> Self {
        let mut registers = Registers::default();
        registers.set_af(0x01B0);
        registers.set_bc(0x0013);
        registers.set_de(0x00D8);
        registers.set_hl(0x014D);
        registers.sp = 0xFFFE;
        registers.pc = 0x0100;
        
        let mut memory_init = vec![
            // Timer registers
            (0xFF05, 0x00), // TIMA
            (0xFF06, 0x00), // TMA
            (0xFF07, 0x00), // TAC
            // APU registers
            (0xFF10, 0x80), // NR10
            (0xFF11, 0xBF), // NR11
            (0xFF12, 0xF3), // NR12
            (0xFF14, 0xBF), // NR14
            (0xFF16, 0x3F), // NR21
            (0xFF17, 0x00), // NR22
            (0xFF19, 0xBF), // NR24
            (0xFF1A, 0x7F), // NR30
            (0xFF1B, 0xFF), // NR31
            (0xFF1C, 0x9F), // NR32
            (0xFF1E, 0xBF), // NR33
            (0xFF20, 0xFF), // NR41
            (0xFF21, 0x00), // NR42
            (0xFF22, 0x00), // NR43
            (0xFF23, 0xBF), // NR44
            (0xFF24, 0x77), // NR50
            (0xFF25, 0xF3), // NR51
            (0xFF26, 0xF1), // NR52 (power on)
            // LCD registers
            (0xFF40, 0x91), // LCDC
            (0xFF41, 0x85), // STAT
            (0xFF42, 0x00), // SCY
            (0xFF43, 0x00), // SCX
            (0xFF45, 0x00), // LYC
            (0xFF47, 0xFC), // BGP
            (0xFF48, 0xFF), // OBP0
            (0xFF49, 0xFF), // OBP1
            (0xFF4A, 0x00), // WY
            (0xFF4B, 0x00), // WX
            // CGB-specific registers
            (0xFF4F, 0x00), // VBK
            (0xFF50, 0x01), // Set to non-zero to skip boot
            (0xFF51, 0x00), // HDMA1
            (0xFF52, 0x00), // HDMA2
            (0xFF53, 0x00), // HDMA3
            (0xFF54, 0x00), // HDMA4
            (0xFF55, 0x00), // HDMA5
            (0xFF56, 0x00), // RP (infrared)
            (0xFF68, 0x00), // BCPS
            (0xFF69, 0x00), // BCPD
            (0xFF6A, 0x00), // OCPS
            (0xFF6B, 0x00), // OCPD
            (0xFF70, 0x00), // SVBK
            // Interrupt enable
            (0xFFFF, 0x00), // IE
        ];
        
        // Initialize WRAM (CGB has 8 banks)
        for i in 0xC000..0xE000 {
            memory_init.push((i, 0x00));
        }
        
        // Initialize VRAM (CGB has 2 banks)
        for i in 0x8000..0xA000 {
            memory_init.push((i, 0x00));
        }
        
        // Initialize HRAM
        for i in 0xFF80..0xFFFF {
            memory_init.push((i, 0x00));
        }
        
        // Initialize OAM
        for i in 0xFE00..0xFE9F {
            memory_init.push((i, 0x00));
        }
        
        Self {
            registers,
            ime: true,
            halted: false,
            memory_init,
        }
    }
}

/// Initialize a machine with boot state.
pub fn initialize_machine(cartridge_data: Vec<u8>, cgb_mode: bool) -> (Machine, BootState) {
    use crate::cartridge::Cartridge;
    
    let boot_state = if cgb_mode {
        BootState::cgb()
    } else {
        BootState::dmg()
    };
    
    let mut machine = Machine::new(Cartridge::load(cartridge_data).unwrap());
    
    // Set CPU registers from boot state
    let cpu = machine.cpu_mut();
    cpu.registers = boot_state.registers;
    cpu.ime = boot_state.ime;
    cpu.halted = boot_state.halted;
    
    // Initialize memory
    let bus = machine.bus_mut();
    for (addr, value) in &boot_state.memory_init {
        let _ = bus.write(*addr, *value);
    }
    
    (machine, boot_state)
}
