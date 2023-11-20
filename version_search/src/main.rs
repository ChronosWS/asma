use std::fs::File;
use std::io::BufReader;
use std::io::{Read, Write};

fn main() {
    let file = std::fs::File::open(r#"E:\Games\Servers\12cf7eb4-4f8a-48aa-902c-ebe73fc28026\ShooterGame\Binaries\Win64\ArkAscendedServer.exe"#).expect("Failed to open file");

    let mut reader = BufReader::new(file);

    let target_bytes = [
        0x41, 0x00, 0x72, 0x00, 0x6B, 0x00, 0x56, 0x00, 0x65, 0x00, 0x72, 0x00, 0x73, 0x00, 0x69,
        0x00, 0x6F, 0x00, 0x6E, 0x00, 0x00, 0x00,
    ];

    // TODO: Increment position here
    fn read_to_byte(reader: &mut BufReader<File>, needle: u8) -> bool {
        loop {
            let mut actual_byte = [0u8];
            if reader.read_exact(&mut actual_byte).is_ok() {
                if actual_byte[0] == needle {
                    return true;
                }
            } else {
                return false;
            }
        }
    }

    let start_time = std::time::Instant::now();
    let mut current_position = 0usize;

    let mut bytes_read = Vec::new();
    loop {
        bytes_read.clear();
        if read_to_byte(&mut reader, target_bytes[0]) {
            let result = target_bytes[1..]
                .iter()
                .enumerate()
                .find_map(|(index, &needle)| {
                    let mut actual_byte = [0u8];
                    current_position += 1;
                    if reader.read_exact(&mut actual_byte).is_ok() && actual_byte[0] == needle {
                        bytes_read.push(actual_byte[0]);
                        None
                    } else {
                        Some(index)
                    }
                });
            match result {
                Some(_) => {}
                None => {
                    println!("Found at offset {}", current_position);
                    bytes_read.iter().for_each(|v| {
                        let _ = write!(std::io::stdout(), "{:02x} ", v);
                    });
                    break;
                }
            }
        } else {
            println!("End of file");
            return;
        }
        current_position = current_position + 1;
    }

    let mut version = String::new();
    let mut buf = [0u8; 2];
    while reader.read_exact(&mut buf).is_ok() {
        let unicode_val = u16::from_le_bytes(buf);
        if unicode_val == 0 {
            break;
        }
        if let Some(char) = char::from_u32(unicode_val as u32) {
            version.push(char);
        } else {
            println!("ERROR: Failed to convert character");
            break;
        }
    }

    let end_time = std::time::Instant::now();
    println!();
    println!("Duration: {:.2}", (end_time - start_time).as_secs_f32());
    println!("Version: {}", version);
}
