use embedded_hal::delay::DelayNs;
use embedded_hal::i2c::I2c;
use rppal::hal::Delay;
use rppal::i2c::I2c as RppalI2c;
use std::error::Error;
use std::thread;
use std::time::Duration;

// LCD 1602A I2C Constants
const LCD_ADDRESS: u8 = 0x27; // Common I2C address (could also be 0x3F)
const LCD_WIDTH: u8 = 16;
const LCD_HEIGHT: u8 = 2;

// PCF8574 I2C Expander Pin Mapping for LCD
const LCD_RS: u8 = 0x01;     // P0 - Register Select
const LCD_RW: u8 = 0x02;     // P1 - Read/Write (unused, tied low)
const LCD_EN: u8 = 0x04;     // P2 - Enable
const LCD_BACKLIGHT: u8 = 0x08; // P3 - Backlight control
// P4-P7 are used for LCD data lines DB4-DB7

// LCD Commands (HD44780/ST7066U compatible)
const LCD_CLEAR_DISPLAY: u8 = 0x01;
const LCD_RETURN_HOME: u8 = 0x02;
const LCD_ENTRY_MODE_SET: u8 = 0x04;
const LCD_DISPLAY_CONTROL: u8 = 0x08;
const LCD_CURSOR_SHIFT: u8 = 0x10;
const LCD_FUNCTION_SET: u8 = 0x20;
const LCD_SET_CGRAM_ADDR: u8 = 0x40;
const LCD_SET_DDRAM_ADDR: u8 = 0x80;

// Entry mode flags
const LCD_ENTRY_INCREMENT: u8 = 0x02;
const LCD_ENTRY_SHIFT_DISPLAY: u8 = 0x01;

// Display control flags
const LCD_DISPLAY_ON: u8 = 0x04;
const LCD_CURSOR_ON: u8 = 0x02;
const LCD_BLINK_ON: u8 = 0x01;

// Function set flags
const LCD_4BIT_MODE: u8 = 0x00;
const LCD_8BIT_MODE: u8 = 0x10;
const LCD_1LINE: u8 = 0x00;
const LCD_2LINE: u8 = 0x08;
const LCD_5x8DOTS: u8 = 0x00;
const LCD_5x10DOTS: u8 = 0x04;

pub struct Lcd1602<I2C> {
    i2c: I2C,
    address: u8,
    backlight: u8,
    delay: Delay,
}

impl<I2C: I2c> Lcd1602<I2C> 
where
    I2C::Error: std::error::Error + 'static,
{
    pub fn new(i2c: I2C, address: u8) -> Result<Self, Box<dyn Error>> {
        let mut lcd = Lcd1602 {
            i2c,
            address,
            backlight: LCD_BACKLIGHT,
            delay: Delay::new(),
        };
        
        println!("🔧 Initializing LCD 1602A...");
        lcd.init()?;
        Ok(lcd)
    }

    /// Initialize LCD with slow, observable sequences
    fn init(&mut self) -> Result<(), Box<dyn Error>> {
        println!("📡 I2C Init Sequence - Watch for 0x{:02X} address on oscilloscope", self.address);
        
        // Wait for LCD power stabilization
        self.long_delay(50_000_000); // 50ms
        
        // === OSCILLOSCOPE ANALYSIS POINT 1: Initial Reset Sequence ===
        println!("🔍 SCOPE POINT 1: LCD Reset sequence (watch for repeated 0x30 commands)");
        
        // Send reset sequence (visible as three identical I2C transactions)
        self.write_4bits(0x03)?; // First reset
        self.long_delay(4_500_000); // 4.5ms
        
        self.write_4bits(0x03)?; // Second reset  
        self.long_delay(4_500_000); // 4.5ms
        
        self.write_4bits(0x03)?; // Third reset
        self.long_delay(150_000); // 150µs
        
        // === OSCILLOSCOPE ANALYSIS POINT 2: Mode Setting ===
        println!("🔍 SCOPE POINT 2: 4-bit mode setting (watch for 0x02 command)");
        self.write_4bits(0x02)?; // Set 4-bit mode
        self.long_delay(150_000); // 150µs
        
        // === OSCILLOSCOPE ANALYSIS POINT 3: Function Set Command ===
        println!("🔍 SCOPE POINT 3: Function set command (2-line, 5x8 dots)");
        self.command(LCD_FUNCTION_SET | LCD_4BIT_MODE | LCD_2LINE | LCD_5x8DOTS)?;
        
        // === OSCILLOSCOPE ANALYSIS POINT 4: Display Control ===
        println!("🔍 SCOPE POINT 4: Display control (display on, cursor off, blink off)");
        self.command(LCD_DISPLAY_CONTROL | LCD_DISPLAY_ON)?;
        
        // === OSCILLOSCOPE ANALYSIS POINT 5: Clear Display ===
        println!("🔍 SCOPE POINT 5: Clear display command (longest execution time)");
        self.command(LCD_CLEAR_DISPLAY)?;
        self.long_delay(2_000_000); // 2ms - clear display takes longer
        
        // === OSCILLOSCOPE ANALYSIS POINT 6: Entry Mode ===
        println!("🔍 SCOPE POINT 6: Entry mode set (increment, no shift)");
        self.command(LCD_ENTRY_MODE_SET | LCD_ENTRY_INCREMENT)?;
        
        println!("✅ LCD initialization complete");
        Ok(())
    }

    /// Send command to LCD (visible as command pattern on I2C)
    fn command(&mut self, cmd: u8) -> Result<(), Box<dyn Error>> {
        println!("📤 CMD: 0x{:02X} (binary: {:08b})", cmd, cmd);
        self.send(cmd, 0)?; // RS=0 for command
        self.short_delay(2_000_000); // 2ms delay for clear observation
        Ok(())
    }

    /// Send data to LCD (visible as data pattern on I2C)
    fn write_data(&mut self, data: u8) -> Result<(), Box<dyn Error>> {
        println!("📤 DATA: 0x{:02X} '{}' (binary: {:08b})", data, data as char, data);
        self.send(data, LCD_RS)?; // RS=1 for data
        self.short_delay(1_000_000); // 1ms delay
        Ok(())
    }

    /// Core I2C communication function
    fn send(&mut self, value: u8, mode: u8) -> Result<(), Box<dyn Error>> {
        let high_nibble = value & 0xF0;
        let low_nibble = (value << 4) & 0xF0;
        
        // Send high nibble
        self.write_4bits(high_nibble | mode)?;
        // Send low nibble  
        self.write_4bits(low_nibble | mode)?;
        
        Ok(())
    }

    /// Write 4 bits with enable pulse (creates distinctive I2C pattern)
    fn write_4bits(&mut self, data: u8) -> Result<(), Box<dyn Error>> {
        let byte_val = data | self.backlight;
        
        // Write data with enable LOW
        self.i2c_write(byte_val)?;
        self.short_delay(1_000); // 1µs
        
        // Write data with enable HIGH (creates pulse on oscilloscope)
        self.i2c_write(byte_val | LCD_EN)?;
        self.short_delay(1_000); // 1µs - enable pulse width
        
        // Write data with enable LOW (complete the pulse)
        self.i2c_write(byte_val & !LCD_EN)?;
        self.short_delay(50_000); // 50µs setup time
        
        Ok(())
    }

    /// Raw I2C write operation
    fn i2c_write(&mut self, data: u8) -> Result<(), Box<dyn Error>> {
        self.i2c.write(self.address, &[data]).map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    /// Print string to LCD with observable I2C patterns
    pub fn print(&mut self, text: &str) -> Result<(), Box<dyn Error>> {
        println!("🔍 SCOPE ANALYSIS: Printing '{}' - watch for ASCII patterns", text);
        for (i, ch) in text.chars().enumerate() {
            if i < LCD_WIDTH as usize {
                println!("  Character {}: '{}' = 0x{:02X}", i + 1, ch, ch as u8);
                self.write_data(ch as u8)?;
                // Extra delay between characters for clear oscilloscope separation
                self.long_delay(5_000_000); // 5ms between characters
            }
        }
        Ok(())
    }

    /// Clear display with observable delay
    pub fn clear(&mut self) -> Result<(), Box<dyn Error>> {
        println!("🔍 SCOPE POINT: Clear display command");
        self.command(LCD_CLEAR_DISPLAY)?;
        self.long_delay(2_000_000); // 2ms
        Ok(())
    }

    /// Set cursor position with observable DDRAM address setting
    pub fn set_cursor(&mut self, col: u8, row: u8) -> Result<(), Box<dyn Error>> {
        let row_offset = if row == 0 { 0x00 } else { 0x40 };
        let address = LCD_SET_DDRAM_ADDR | (col + row_offset);
        println!("🔍 SCOPE POINT: Set cursor to ({}, {}) - DDRAM addr 0x{:02X}", col, row, address);
        self.command(address)?;
        Ok(())
    }

    /// Backlight control with observable I2C difference
    pub fn set_backlight(&mut self, on: bool) -> Result<(), Box<dyn Error>> {
        self.backlight = if on { LCD_BACKLIGHT } else { 0 };
        println!("🔍 SCOPE POINT: Backlight {} - watch for backlight bit change", 
                if on { "ON" } else { "OFF" });
        // Send a dummy command to update backlight state
        self.i2c_write(self.backlight)?;
        self.long_delay(1_000_000); // 1ms
        Ok(())
    }

    /// Short delay for timing-critical operations
    fn short_delay(&mut self, nanoseconds: u32) {
        self.delay.delay_ns(nanoseconds);
    }

    /// Long delay for oscilloscope observation
    fn long_delay(&mut self, nanoseconds: u32) {
        self.delay.delay_ns(nanoseconds);
        // Additional thread sleep for very clear separation on scope
        if nanoseconds > 1_000_000 {
            thread::sleep(Duration::from_nanos(nanoseconds as u64));
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("🚀 LCD 1602A I2C Driver - Oscilloscope Analysis Mode");
    println!("📊 Connect oscilloscope to:");
    println!("   - SDA1 (GPIO 2, Pin 3)");
    println!("   - SCL1 (GPIO 3, Pin 5)");
    println!("   - Set timebase to 100µs/div initially");
    println!("   - Trigger on SDA falling edge");
    println!("");

    // Initialize I2C bus 1 (standard for RPi4)
    let i2c = RppalI2c::with_bus(1)?;
    println!("📡 I2C bus 1 initialized");
    println!("⚙️  Note: For 50kHz clock speed, add 'dtparam=i2c_arm_baudrate=50000' to /boot/firmware/config.txt and reboot");
    
    // Display current I2C clock speed
    match i2c.clock_speed() {
        Ok(speed) => println!("🔧 Current I2C clock speed: {} Hz", speed),
        Err(e) => println!("⚠️  Could not read I2C clock speed: {}", e),
    }
    
    // Test if LCD is present
    println!("🔍 Testing I2C connection to LCD at address 0x{:02X}...", LCD_ADDRESS);
    
    let mut lcd = Lcd1602::new(i2c, LCD_ADDRESS)?;
    
    println!("🎯 Starting oscilloscope-friendly demonstration...");
    println!("");

    // === Demo Sequence with Clear Oscilloscope Patterns ===
    
    // Pattern 1: Simple text
    println!("=== DEMO 1: Basic Text Display ===");
    lcd.clear()?;
    lcd.print("Hello, Scope!")?;
    thread::sleep(Duration::from_secs(3));
    
    // Pattern 2: Two-line display
    println!("=== DEMO 2: Two-Line Display ===");
    lcd.clear()?;
    lcd.set_cursor(0, 0)?;
    lcd.print("Line 1: I2C")?;
    lcd.set_cursor(0, 1)?;
    lcd.print("Line 2: Analysis")?;
    thread::sleep(Duration::from_secs(3));
    
    // Pattern 3: Backlight control demonstration
    println!("=== DEMO 3: Backlight Control ===");
    for i in 0..5 {
        println!("Backlight cycle {}/5", i + 1);
        lcd.set_backlight(false)?;
        thread::sleep(Duration::from_secs(1));
        lcd.set_backlight(true)?;
        thread::sleep(Duration::from_secs(1));
    }
    
    // Pattern 4: Character-by-character with timing
    println!("=== DEMO 4: Slow Character Display ===");
    lcd.clear()?;
    lcd.set_cursor(0, 0)?;
    let demo_text = "I2C Waveform";
    for ch in demo_text.chars() {
        lcd.write_data(ch as u8)?;
        thread::sleep(Duration::from_millis(500)); // Very slow for easy analysis
    }
    
    // Pattern 5: ASCII pattern demonstration
    println!("=== DEMO 5: ASCII Pattern Test ===");
    lcd.clear()?;
    lcd.set_cursor(0, 0)?;
    // Display ascending ASCII values for pattern recognition
    for i in 0x41..=0x50 { // 'A' to 'P'
        lcd.write_data(i)?;
        thread::sleep(Duration::from_millis(300));
    }
    
    println!("🎉 Demonstration complete!");
    println!("💡 Oscilloscope Analysis Tips:");
    println!("   - Each I2C transaction starts with START condition");
    println!("   - Address 0x{:02X} appears in every transaction", LCD_ADDRESS);
    println!("   - Enable pulses create distinctive patterns in data");
    println!("   - Backlight bit (bit 3) changes affect every transmission");
    println!("   - Commands vs data differ by RS bit (bit 0)");
    
    Ok(())
}
