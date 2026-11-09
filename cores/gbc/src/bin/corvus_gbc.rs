// SPDX-FileCopyrightText: 2026 CorvusSoftware
// SPDX-License-Identifier: AGPL-3.0-only

//! Main entry point for the Corvus GBC emulator.
//!
//! This binary provides a simple SDL2 frontend to run Game Boy Color ROMs.

use std::env;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use corvus_gbc::boot;
use corvus_gbc::frontend::Frontend;

/// Target FPS for the emulator.
const TARGET_FPS: u32 = 60;
const TARGET_FRAME_TIME: Duration = Duration::from_nanos(16_666_667); // ~60Hz

/// Cycles per frame (DMG: ~70224, CGB: similar)
const CYCLES_PER_FRAME: u64 = 70224;

fn main() {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <rom_file> [--cgb|--dmg]", args[0]);
        eprintln!("  --cgb: Force CGB mode");
        eprintln!("  --dmg: Force DMG mode");
        std::process::exit(1);
    }
    
    let rom_path = &args[1];
    let mut force_cgb = false;
    let mut force_dmg = false;
    
    for arg in &args[2..] {
        match arg.as_str() {
            "--cgb" => force_cgb = true,
            "--dmg" => force_dmg = true,
            _ => {}
        }
    }
    
    // Load ROM file
    let rom_data = match fs::read(rom_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error loading ROM file '{}': {}", rom_path, e);
            std::process::exit(1);
        }
    };
    
    // Determine hardware mode
    let cgb_mode = if force_cgb {
        true
    } else if force_dmg {
        false
    } else {
        // Auto-detect from cartridge header
        use corvus_gbc::cartridge::CartridgeHeader;
        match CartridgeHeader::parse(&rom_data) {
            Ok(header) => matches!(header.cgb_support, corvus_gbc::cartridge::CgbSupport::Enhanced | corvus_gbc::cartridge::CgbSupport::Exclusive),
            Err(_) => false, // Default to DMG if header parsing fails
        }
    };
    
    // Initialize SDL2 frontend
    let mut frontend = match Frontend::new("Corvus GBC") {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error initializing SDL2: {}", e);
            std::process::exit(1);
        }
    };
    
    // Initialize machine with boot state
    let (mut machine, _boot_state) = boot::initialize_machine(rom_data, cgb_mode);
    
    println!("Starting Corvus GBC Emulator");
    println!("Mode: {}", if cgb_mode { "CGB" } else { "DMG" });
    println!("ROM: {}", Path::new(rom_path).file_name().unwrap().to_string_lossy());
    println!("Press ESC to quit");
    
    // Main emulation loop
    let mut last_time = Instant::now();
    let mut frame_count = 0u64;
    let mut total_cycles = 0u64;
    let mut prev_ly = 0u8;
    
    'main: loop {
        // Process input
        let joypad_state = frontend.process_events();
        
        if joypad_state.quit {
            break 'main;
        }
        
        // Update joypad in bus
        machine.bus_mut().joypad.update_state(joypad_state);
        
        // Run emulation until we complete a frame
        let _start_ly = machine.bus().ly();
        let mut cycles = 0u64;
        let mut steps = 0u64;
        
        loop {
            // Safety check
            if steps > CYCLES_PER_FRAME * 4 {
                break;
            }
            
            match machine.step() {
                Ok(step_cycles) => {
                    cycles += u64::from(step_cycles);
                    steps += 1;
                }
                Err(e) => {
                    eprintln!("CPU error: {:?}", e);
                    break 'main;
                }
            }
            
            let current_ly = machine.bus().ly();
            
            // Frame complete when LY wraps around
            if current_ly < prev_ly && prev_ly >= 144 {
                // Get audio samples from APU
                let audio_samples = machine.bus_mut().apu.get_audio_buffer();
                if !audio_samples.is_empty() {
                    frontend.queue_audio(audio_samples);
                }
                break;
            }
            
            prev_ly = current_ly;
        }
        
        total_cycles += cycles;
        frame_count += 1;
        
        // Update display
        let framebuffer = machine.bus().framebuffer();
        frontend.update_framebuffer(framebuffer);
        
        if let Err(e) = frontend.render() {
            eprintln!("Error rendering: {}", e);
            break 'main;
        }
        
        // Frame rate control
        let elapsed = last_time.elapsed();
        if elapsed < TARGET_FRAME_TIME {
            let sleep_time = TARGET_FRAME_TIME.checked_sub(elapsed).unwrap_or(Duration::ZERO);
            std::thread::sleep(sleep_time);
        }
        last_time = Instant::now();
        
        // Print stats every second
        if frame_count % (TARGET_FPS as u64) == 0 {
            let avg_cycles = total_cycles / frame_count;
            println!(
                "FPS: {}, Avg cycles/frame: {}, Total cycles: {}",
                TARGET_FPS, avg_cycles, total_cycles
            );
        }
    }
    
    println!("Emulation stopped. Total frames: {}, Total cycles: {}", frame_count, total_cycles);
}
