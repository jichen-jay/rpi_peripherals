use rppal::i2c::I2c as RppalI2c;
use std::error::Error;
use std::thread;
use std::time::{Duration, Instant};

// Common LCD I2C addresses
const COMMON_ADDRESSES: [u8; 2] = [0x27, 0x3F];

pub struct SimpleI2cTransmitter {
    i2c: RppalI2c,
    address: u8,
}

impl SimpleI2cTransmitter {
    pub fn new(mut i2c: RppalI2c, address: u8) -> Result<Self, Box<dyn Error>> {
        // Set the slave address for this I2C instance
        i2c.set_slave_address(address as u16)?;
        Ok(SimpleI2cTransmitter { i2c, address })
    }

    /// Send single byte with detailed error logging
    fn send_byte(&mut self, data: u8, description: &str) -> Result<(), Box<dyn Error>> {
        print!("📡 TX: 0x{:02X} {} ", data, description);
        
        match self.i2c.write(&[data]) {
            Ok(_) => {
                println!("✅ ACK - PCF8574 responded!");
                Ok(())
            },
            Err(e) => {
                println!("❌ Error: {}", e);
                // Don't fail completely, continue for scope analysis
                Ok(())
            }
        }
    }

    /// Send "Happy Birthday" message and measure timing
    pub fn send_message(&mut self, message_number: u8) -> Result<Duration, Box<dyn Error>> {
        println!("\n🎉 MESSAGE {} - Sending 'Happy Birthday'", message_number);
        let start_time = Instant::now();
        
        // Start marker
        self.send_byte(0xFF, "START")?;
        thread::sleep(Duration::from_millis(50));
        
        // Send each character
        let text = "Happy Birthday";
        for ch in text.chars() {
            let ascii = ch as u8;
            self.send_byte(ascii, &format!("'{}'", ch))?;
            thread::sleep(Duration::from_millis(50)); // 50ms between characters
        }
        
        // End marker
        self.send_byte(0x00, "END")?;
        
        let transmission_time = start_time.elapsed();
        println!("✅ Message {} complete in {:.1}ms\n", message_number, transmission_time.as_millis());
        
        Ok(transmission_time)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("🚀 Dynamic Rhythm I2C 'Happy Birthday' Transmitter");
    println!("🎵 Pattern: Send → Wait(same duration) → Send → Wait → repeat for 2s");
    println!("⚠️  Make sure to run with: sudo ./your_program");
    println!("");
    println!("🔧 Oscilloscope Setup:");
    println!("   - SDA: GPIO 2 (Pin 3)");
    println!("   - SCL: GPIO 3 (Pin 5)");
    println!("   - GND: Pin 6");
    println!("   - Timebase: 200ms/div (to see rhythm pattern)");
    println!("   - Trigger: SDA falling edge");
    println!("");

    // Initialize I2C
    let mut i2c = RppalI2c::with_bus(1)?;
    println!("📡 I2C bus 1 initialized");
    
    // Show I2C speed if available
    if let Ok(speed) = i2c.clock_speed() {
        println!("🔧 I2C speed: {} Hz", speed);
    }
    
    // Auto-detect I2C address
    let mut working_address = None;
    println!("🔍 Scanning for LCD I2C controller...");
    
    for &addr in &COMMON_ADDRESSES {
        println!("   Testing address 0x{:02X}...", addr);
        i2c.set_slave_address(addr as u16)?;
        match i2c.write(&[0x00]) {
            Ok(_) => {
                println!("   ✅ Found working device at 0x{:02X}!", addr);
                working_address = Some(addr);
                break;
            },
            Err(_) => {
                println!("   ❌ No response at 0x{:02X}", addr);
            }
        }
    }
    
    let target_address = working_address.unwrap_or(COMMON_ADDRESSES[0]);
    if working_address.is_none() {
        println!("⚠️  No I2C device found, using 0x{:02X} anyway for scope analysis", target_address);
    }
    
    let mut transmitter = SimpleI2cTransmitter::new(i2c, target_address)?;
    
    println!("🎯 Starting dynamic rhythm transmission...");
    println!("📍 Target address: 0x{:02X}", target_address);
    println!("⏱️  Total duration: 2 seconds");
    println!("");

    // Dynamic rhythm pattern for 2 seconds
    let start_time = Instant::now();
    let total_duration = Duration::from_secs(2);
    let mut message_count = 0;
    
    println!("🎵 Starting rhythm pattern...");
    
    while start_time.elapsed() < total_duration {
        let remaining_time = total_duration - start_time.elapsed();
        
        message_count += 1;
        println!("⏰ Rhythm cycle {} (Remaining: {:.1}s)", message_count, remaining_time.as_secs_f32());
        
        // Send message and measure how long it takes
        let transmission_time = transmitter.send_message(message_count)?;
        
        // Wait for the same duration as the transmission took
        let wait_time = transmission_time;
        println!("⏳ Waiting {:.1}ms (same as transmission time)...", wait_time.as_millis());
        
        // Check if we have enough time for both wait and next transmission
        let time_needed = wait_time + transmission_time; // Estimate for next transmission
        if start_time.elapsed() + time_needed >= total_duration {
            println!("⏰ Not enough time for complete cycle, stopping...");
            break;
        }
        
        thread::sleep(wait_time);
    }
    
    let actual_duration = start_time.elapsed();
    println!("🏁 Rhythm pattern complete!");
    println!("");
    println!("📊 Summary:");
    println!("   - Messages sent: {}", message_count);
    println!("   - Actual duration: {:.2}s", actual_duration.as_secs_f32());
    println!("   - Characters per message: 14 ('Happy Birthday')");
    println!("   - Pattern: Send → Wait(same time) → Repeat");
    println!("");
    println!("🔍 Oscilloscope Analysis:");
    println!("   📍 Look for rhythmic bursts of I2C activity");
    println!("   📍 Each burst followed by quiet period of same duration");
    println!("   📍 {} complete cycles in 2 seconds", message_count);
    println!("   📍 Address: 0x{:02X} (0b{:08b})", target_address << 1, target_address << 1);
    if working_address.is_some() {
        println!("   ✅ Should see ACK responses (SDA low on 9th clock)");
    } else {
        println!("   ❌ Will see NACK responses (SDA high on 9th clock)");
    }
    println!("");
    println!("📝 ASCII values in each message:");
    for ch in "Happy Birthday".chars() {
        println!("   '{}' = 0x{:02X}", ch, ch as u8);
    }
    println!("   Start marker = 0xFF");
    println!("   End marker = 0x00");
    
    Ok(())
}