use rppal::i2c::I2c as RppalI2c;
use std::error::Error;
use std::thread;
use std::time::Duration;

// I2C Target address 
const TARGET_ADDRESS: u8 = 0x27; // Try 0x3F if 0x27 doesn't work

pub struct SimpleI2cTransmitter {
    i2c: RppalI2c,
    address: u8,
}

impl SimpleI2cTransmitter {
    pub fn new(i2c: RppalI2c, address: u8) -> Self {
        SimpleI2cTransmitter { i2c, address }
    }

    /// Send single byte with minimal logging
    fn send_byte(&mut self, data: u8, description: &str) -> Result<(), Box<dyn Error>> {
        print!("📡 TX: 0x{:02X} {} ", data, description);
        
        match self.i2c.write(&[data]) {
            Ok(_) => println!("✅ ACK"),
            Err(_) => println!("❌ NACK (expected if no device)"),
        }
        
        Ok(())
    }

    /// Send "Happy Birthday" message
    pub fn send_message(&mut self, message_number: u8) -> Result<(), Box<dyn Error>> {
        println!("\n🎉 MESSAGE {} - Sending 'Happy Birthday'", message_number);
        
        // Start marker
        self.send_byte(0xFF, "START")?;
        thread::sleep(Duration::from_millis(100));
        
        // Send each character
        let text = "Happy Birthday";
        for (i, ch) in text.chars().enumerate() {
            let ascii = ch as u8;
            self.send_byte(ascii, &format!("'{}'", ch))?;
            thread::sleep(Duration::from_millis(50)); // 50ms between characters
        }
        
        // End marker
        self.send_byte(0x00, "END")?;
        
        println!("✅ Message {} complete\n", message_number);
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("🚀 Simple I2C 'Happy Birthday' Transmitter");
    println!("📊 3 messages, 3 seconds apart, then stop");
    println!("");
    println!("🔧 Oscilloscope Setup:");
    println!("   - SDA: GPIO 2 (Pin 3)");
    println!("   - SCL: GPIO 3 (Pin 5)");
    println!("   - GND: Pin 6");
    println!("   - Timebase: 20µs/div");
    println!("   - Trigger: SDA falling edge");
    println!("");

    // Initialize I2C
    let i2c = RppalI2c::with_bus(1)?;
    println!("📡 I2C bus 1 initialized");
    
    // Show I2C speed if available
    if let Ok(speed) = i2c.clock_speed() {
        println!("🔧 I2C speed: {} Hz", speed);
    }
    
    let mut transmitter = SimpleI2cTransmitter::new(i2c, TARGET_ADDRESS);
    
    println!("🎯 Starting transmission sequence...");
    println!("📍 Target address: 0x{:02X}", TARGET_ADDRESS);
    println!("");

    // Send 3 messages with 3-second spacing
    for message_num in 1..=3 {
        println!("⏰ Message {} in progress...", message_num);
        transmitter.send_message(message_num)?;
        
        if message_num < 3 {
            println!("⏳ Waiting 3 seconds before next message...");
            for countdown in (1..=3).rev() {
                println!("   {}...", countdown);
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
    
    println!("🏁 All messages sent!");
    println!("");
    println!("📊 Summary:");
    println!("   - Messages sent: 3");
    println!("   - Characters per message: 14 ('Happy Birthday')");
    println!("   - Total I2C transactions: ~48 (3 × 16 bytes each)");
    println!("   - Spacing: 3 seconds between messages");
    println!("");
    println!("🔍 What to look for on oscilloscope:");
    println!("   📍 3 distinct bursts of I2C activity");
    println!("   📍 Each burst contains 16 I2C transactions");
    println!("   📍 START condition: SDA low while SCL high");
    println!("   📍 Address: 0x{:02X} (0b{:08b})", TARGET_ADDRESS << 1, TARGET_ADDRESS << 1);
    println!("   📍 NACK responses: SDA high on 9th clock");
    println!("   📍 STOP condition: SDA high while SCL high");
    println!("");
    println!("📝 ASCII values transmitted:");
    for ch in "Happy Birthday".chars() {
        println!("   '{}' = 0x{:02X}", ch, ch as u8);
    }
    println!("   Start marker = 0xFF");
    println!("   End marker = 0x00");
    
    Ok(())
}