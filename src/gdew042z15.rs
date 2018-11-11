use crate::{Color, Display, Error, Hertz};

enum InitState {
    Uninitialized,
    Resetting1,
    Resetting2,
    Resetting3,
    // TODO
    Initialized,
}

pub struct GDEW042Z15<SPI, Busy, Reset, DataCmd, CS, Timer> {
    spi: SPI,
    busy: Busy,
    reset: Reset,
    data_cmd: DataCmd,
    cs: CS,
    timer: Timer,
    init_state: InitState,
    current_pixels: u8,
    current_pixel_count: u8,
}

impl<SPI, Busy, Reset, DataCmd, CS, Timer> GDEW042Z15<SPI, Busy, Reset, DataCmd, CS, Timer>
where
    SPI: embedded_hal::spi::FullDuplex<u8>,
    Busy: embedded_hal::digital::InputPin,
    Reset: embedded_hal::digital::OutputPin,
    DataCmd: embedded_hal::digital::OutputPin,
    CS: embedded_hal::digital::OutputPin,
    Timer: embedded_hal::timer::CountDown<Time = Hertz>,
{
    pub fn new(
        spi: SPI,
        busy: Busy,
        mut reset: Reset,
        mut data_cmd: DataCmd,
        mut cs: CS,
        timer: Timer,
    ) -> Self {
        reset.set_high();
        data_cmd.set_high();
        cs.set_high();
        GDEW042Z15 {
            spi: spi,
            busy: busy,
            reset: reset,
            data_cmd: data_cmd,
            cs: cs,
            timer: timer,
            init_state: InitState::Uninitialized,
            current_pixels: 0,
            current_pixel_count: 0,
        }
    }

    pub fn init(&mut self) -> nb::Result<(), Error> {
        match self.init_state {
            InitState::Uninitialized => {
                // Wait until the display is idle (busy pin = 1) before
                // continuing.
                if self.busy.is_low() {
                    return Err(nb::Error::WouldBlock);
                }

                // Assert reset for 100ms.
                self.reset.set_low();
                self.timer.start(Hertz(10));

                self.init_state = InitState::Resetting1;
                self.init()
            }
            InitState::Resetting1 => {
                if let Err(nb::Error::WouldBlock) = self.timer.wait() {
                    return Err(nb::Error::WouldBlock);
                }

                // Wait 100ms for reinitialization.
                self.reset.set_high();
                self.timer.start(Hertz(10));

                self.init_state = InitState::Resetting2;
                self.init()
            }
            InitState::Resetting2 => {
                if let Err(nb::Error::WouldBlock) = self.timer.wait() {
                    return Err(nb::Error::WouldBlock);
                }

                // Booster soft start.
                self.send_command(DisplayCommand::BoosterSoftStart);
                self.send_data(0x17);
                self.send_data(0x17);
                self.send_data(0x17);
                self.send_command(DisplayCommand::PowerOn);
                self.init_state = InitState::Resetting3;
                self.init()
            }
            InitState::Resetting3 => {
                // Wait until idle.
                if self.busy.is_low() {
                    return Err(nb::Error::WouldBlock);
                }
                self.send_command(DisplayCommand::PanelSetting);
                self.send_data(0x0f);
                self.init_state = InitState::Initialized;
                Ok(())
            }
            InitState::Initialized => Ok(()),
        }
    }

    pub fn destroy(self) -> (SPI, Busy, Reset, DataCmd, CS, Timer) {
        (
            self.spi,
            self.busy,
            self.reset,
            self.data_cmd,
            self.cs,
            self.timer,
        )
    }

    fn send_command(&mut self, command: DisplayCommand) {
        self.data_cmd.set_low();
        self.cs.set_low();
        let command = command as u8;
        // TODO: Error handling?
        block!(self.spi.send(command)).ok();
        block!(self.spi.read()).ok();
        self.cs.set_high();
    }

    fn send_data(&mut self, data: u8) {
        self.data_cmd.set_high();
        self.cs.set_low();
        // TODO: Error handling?
        block!(self.spi.send(data)).ok();
        block!(self.spi.read()).ok();
        self.cs.set_high();
    }

    fn delay_2ms(&mut self) {
        self.timer.start(Hertz(500));
        block!(self.timer.wait()).ok();
    }

    fn delay_10ms(&mut self) {
        self.timer.start(Hertz(100));
        block!(self.timer.wait()).ok();
    }
}

impl<SPI, Busy, Reset, DataCmd, CS, Timer> Display
    for GDEW042Z15<SPI, Busy, Reset, DataCmd, CS, Timer>
where
    SPI: embedded_hal::spi::FullDuplex<u8>,
    Busy: embedded_hal::digital::InputPin,
    Reset: embedded_hal::digital::OutputPin,
    DataCmd: embedded_hal::digital::OutputPin,
    CS: embedded_hal::digital::OutputPin,
    Timer: embedded_hal::timer::CountDown<Time = Hertz>,
{
    const WIDTH: u32 = 400;
    const HEIGHT: u32 = 300;

    fn start_frame(&mut self) -> nb::Result<(), Error> {
        if self.busy.is_low() {
            return Err(nb::Error::WouldBlock);
        }
        self.send_command(DisplayCommand::DataStartTransmission1);
        self.delay_2ms();
        self.current_pixels = 0;
        self.current_pixel_count = 0;
        Ok(())
    }

    fn end_frame(&mut self) {
        self.delay_2ms();
        self.send_command(DisplayCommand::DataStartTransmission2);
        self.delay_2ms();
        // We need to send the red color frame, even though we do not support it
        // yet.
        for _ in 0..Self::WIDTH * Self::HEIGHT / 8 {
            self.send_data(0xff);
        }
        self.delay_2ms();

        self.send_command(DisplayCommand::DisplayRefresh);
        // Give the display some time to deassert the busy pin.
        // TODO: Get rid of this blocking delay?
        self.delay_10ms();
    }

    fn fill(&mut self, mut width: u32, color: Color) {
        if self.current_pixel_count as u32 + width < 8 {
            self.current_pixels <<= width;
            if let Color::Black = color {
                self.current_pixels |= 0xff >> (8 - width);
            }
            self.current_pixel_count += width as u8;
        } else {
            // Complete the first byte.
            let remainder = 8 - self.current_pixel_count;
            self.current_pixels <<= 8 - self.current_pixel_count;
            if let Color::Black = color {
                self.current_pixels |= 0xff >> self.current_pixel_count;
            }
            self.send_data(self.current_pixels);
            width -= remainder as u32;
            // Send as many full bytes as possible.
            if let Color::Black = color {
                while width >= 8 {
                    self.send_data(0xff);
                    width -= 8;
                }
            } else {
                while width >= 8 {
                    self.send_data(0x0);
                    width -= 8;
                }
            }
            // Last partial byte.
            self.current_pixels = 0xff >> (8 - width);
            self.current_pixel_count = width as u8;
        }
    }

    fn pixel(&mut self, color: Color) {
        self.current_pixels = (self.current_pixels << 1) | color as u8;
        self.current_pixel_count += 1;
        if self.current_pixel_count == 8 {
            self.send_data(self.current_pixels);
            self.current_pixels = 0;
            self.current_pixel_count = 0;
        }
    }
}

enum DisplayCommand {
    PanelSetting = 0x00,
    _PowerSetting = 0x01,
    _PowerOff = 0x02,
    _PowerOffSequenceSetting = 0x03,
    PowerOn = 0x04,
    _PowerOnMeasure = 0x05,
    BoosterSoftStart = 0x06,
    _DeepSleep = 0x07,
    DataStartTransmission1 = 0x10,
    _DataStop = 0x11,
    DisplayRefresh = 0x12,
    DataStartTransmission2 = 0x13,
    _LUTForVCOM = 0x20,
    _LUTWhiteToWhite = 0x21,
    _LUTBlackToWhite = 0x22,
    _LUTWhiteToBlack = 0x23,
    _LUTBlackToBlack = 0x24,
    _PllControl = 0x30,
    _TemperatureSensorCommand = 0x40,
    _TemperatureSensorSelection = 0x41,
    _TemperatureSensorWrite = 0x42,
    _TemperatureSensorRead = 0x43,
    _VCOMAndDataIntervalSetting = 0x50,
    _LowPowerDetection = 0x51,
    _TCONSetting = 0x60,
    _ResolutionSetting = 0x61,
    _GSSTSetting = 0x65,
    _GetStatus = 0x71,
    _AutoMeasurementVCOM = 0x80,
    _ReadVCOMValue = 0x81,
    _VCMDCSetting = 0x82,
    _PartialWindow = 0x90,
    _PartialIn = 0x91,
    _PartialOut = 0x92,
    _ProgramMode = 0xA0,
    _ActiveProgramming = 0xA1,
    _ReadOTP = 0xA2,
    _PowerSaving = 0xE3,
}
