use std::fs;
use std::process;

struct PatchEntry {
    original_str: String,
    pattern: Vec<Option<u8>>, // None = ??, Some(u8) = byte
    replace: Vec<u8>,
}

fn main() {
    // 1. Define Patterns
    let raw_patterns = vec![
        (
            "40 53 48 83 EC ?? 8B D9 33 C9 E8 ?? ?? ?? ?? 80 3D ?? ?? ?? ?? ?? 75 ?? 8B 0D ?? ?? ?? ?? E8 ?? ?? ?? ?? C6 05 ?? ?? ?? ?? ?? 48 8B 05 ?? ?? ?? ?? 45 33 C0 8B D3 48 8B 88 ?? ?? ?? ?? 48 8B 49 ?? 48 83 C4 ?? 5B E9 ?? ?? ?? ?? CC CC CC CC CC 48 83 EC ?? 33 C9 E8 ?? ?? ?? ?? 80 3D ?? ?? ?? ?? ?? 75 ?? 8B 0D ?? ?? ?? ?? E8 ?? ?? ?? ?? C6 05 ?? ?? ?? ?? ?? 48 8B 05 ?? ?? ?? ?? 33 D2 48 8B 88 ?? ?? ?? ?? 48 8B 49 ?? 48 83 C4 ?? E9 ?? ?? ?? ?? CC CC CC CC CC CC CC CC CC CC CC CC CC 40 53 48 83 EC ?? 8B D9 33 C9 E8 ?? ?? ?? ?? 80 3D ?? ?? ?? ?? ?? 75 ?? 8B 0D ?? ?? ?? ?? E8 ?? ?? ?? ?? C6 05 ?? ?? ?? ?? ?? 48 8B 05 ?? ?? ?? ?? 45 33 C0 8B D3 48 8B 88 ?? ?? ?? ?? 48 8B 49 ?? 48 83 C4 ?? 5B E9 ?? ?? ?? ?? CC CC CC CC CC 48 83 EC ?? 33 C9 E8 ?? ?? ?? ?? 80 3D ?? ?? ?? ?? ?? 75 ?? 8B 0D ?? ?? ?? ?? E8 ?? ?? ?? ?? C6 05 ?? ?? ?? ?? ?? 48 8B 05 ?? ?? ?? ?? 33 D2 48 8B 88 ?? ?? ?? ?? 48 8B 49 ?? 48 83 C4 ?? E9 ?? ?? ?? ?? CC CC CC CC CC CC CC CC CC CC CC CC CC 48 83 EC",
            "48 B8 01 00 00 00 00 00 00 00 C3",
        ),
        (
            "40 53 48 83 EC ?? 8B D9 33 C9 E8 ?? ?? ?? ?? 80 3D ?? ?? ?? ?? ?? 75 ?? 8B 0D ?? ?? ?? ?? E8 ?? ?? ?? ?? C6 05 ?? ?? ?? ?? ?? 48 8B 05 ?? ?? ?? ?? 45 33 C0 8B D3 48 8B 88 ?? ?? ?? ?? 48 8B 49 ?? 48 83 C4 ?? 5B E9 ?? ?? ?? ?? CC CC CC CC CC 40 55 53",
            "B8 85 47 DE 63 C3",
        ),
        (
            "48 83 EC ?? 80 3D ?? ?? ?? ?? ?? 75 ?? 8B 0D ?? ?? ?? ?? E8 ?? ?? ?? ?? C6 05 ?? ?? ?? ?? ?? 48 8B 0D ?? ?? ?? ?? F6 81 ?? ?? ?? ?? ?? 74 ?? 83 B9 ?? ?? ?? ?? ?? 75 ?? E8 ?? ?? ?? ?? 33 C9 E8 ?? ?? ?? ?? 84 C0 0F 85",
            "48 B8 01 00 00 00 00 00 00 00 C3",
        ),
    ];

    let input_file = "GameAssembly.dll";
    let output_file = "GameAssembly_patched.dll";

    // 2. Pre-process Patterns (Parse strings to bytes)
    println!("Parsing patterns...");
    let mut entries = Vec::new();
    for (orig, repl) in raw_patterns {
        let pattern = parse_pattern_string(orig);
        let mut replace = parse_byte_string(repl);
        
        // Pad replacement with zeros if shorter than pattern (Same logic as C++)
        if replace.len() < pattern.len() {
            replace.resize(pattern.len(), 0);
        }

        entries.push(PatchEntry {
            original_str: orig.to_string(),
            pattern,
            replace,
        });
    }

    // 3. Read File
    println!("Loading file: {}", input_file);
    let mut data = match fs::read(input_file) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Failed to read file: {}", e);
            process::exit(1);
        }
    };

    // 4. Scan and Patch
    println!("Scanning...");
    for entry in &entries {
        // Using the optimized search
        match find_pattern_fast(&data, &entry.pattern) {
            Some(offset) => {
                println!(
                    "Found match at offset: 0x{:X} for pattern: {}...",
                    offset,
                    &entry.original_str[0..15.min(entry.original_str.len())]
                );

                // Perform the patch (Overwrite bytes)
                // Rust slice copy is heavily optimized (memcpy)
                data[offset..offset + entry.replace.len()].copy_from_slice(&entry.replace);
            }
            None => {
                println!(
                    "No match found for pattern: {}...",
                    &entry.original_str[0..15.min(entry.original_str.len())]
                );
            }
        }
    }

    // 5. Write Output
    println!("Writing to: {}", output_file);
    match fs::write(output_file, &data) {
        Ok(_) => println!("Success!"),
        Err(e) => eprintln!("Failed to write output: {}", e),
    }
}

// --- Helper Functions ---

fn parse_pattern_string(s: &str) -> Vec<Option<u8>> {
    s.split_whitespace()
        .map(|token| {
            if token == "??" {
                None
            } else {
                Some(u8::from_str_radix(token, 16).expect("Invalid Hex Pattern"))
            }
        })
        .collect()
}

fn parse_byte_string(s: &str) -> Vec<u8> {
    s.split_whitespace()
        .map(|token| u8::from_str_radix(token, 16).expect("Invalid Hex Replacement"))
        .collect()
}

// Optimized Search function
fn find_pattern_fast(data: &[u8], pattern: &[Option<u8>]) -> Option<usize> {
    if data.len() < pattern.len() {
        return None;
    }

    // Optimization: Fast scan for the first byte using standard library iterator
    // Rust's `position` on slices typically compiles to optimized SIMD/memchr calls.
    if let Some(Some(first_byte)) = pattern.first() {
        let mut offset = 0;

        // Search for the first byte
        while let Some(pos) = data[offset..].iter().position(|&b| b == *first_byte) {
            let current_idx = offset + pos;
            
            // Check bounds
            if current_idx + pattern.len() > data.len() {
                return None;
            }

            // Detailed check for the rest of the pattern
            // We use zip to compare data byte vs pattern byte
            let candidate = &data[current_idx..current_idx + pattern.len()];
            let matches = candidate.iter().zip(pattern).all(|(d, p)| {
                match p {
                    Some(val) => *d == *val, // Byte must match
                    None => true,            // Wildcard matches anything
                }
            });

            if matches {
                return Some(current_idx);
            }

            // Not a match, move forward 1 byte and try again
            offset = current_idx + 1;
        }
        None
    } else {
        // Fallback if the pattern starts with "??" (slower, but rare)
        data.windows(pattern.len()).position(|window| {
            window.iter().zip(pattern).all(|(d, p)| {
                match p {
                    Some(val) => *d == *val,
                    None => true,
                }
            })
        })
    }
}
