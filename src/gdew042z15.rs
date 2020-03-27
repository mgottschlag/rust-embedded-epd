use crate::{Display, Error, Hertz};

enum InitState {
    Uninitialized,
    Resetting1,
    Resetting2,
    Resetting3,
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
}

impl<SPI, Busy, Reset, DataCmd, CS, Timer> GDEW042Z15<SPI, Busy, Reset, DataCmd, CS, Timer>
where
    SPI: embedded_hal::spi::FullDuplex<u8>,
    Busy: embedded_hal::digital::v2::InputPin,
    Reset: embedded_hal::digital::v2::OutputPin,
    DataCmd: embedded_hal::digital::v2::OutputPin,
    CS: embedded_hal::digital::v2::OutputPin,
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
        reset.set_high().ok();
        data_cmd.set_high().ok();
        cs.set_high().ok();
        GDEW042Z15 {
            spi: spi,
            busy: busy,
            reset: reset,
            data_cmd: data_cmd,
            cs: cs,
            timer: timer,
            init_state: InitState::Uninitialized,
        }
    }

    pub fn init(&mut self) -> nb::Result<(), Error> {
        match self.init_state {
            InitState::Uninitialized => {
                // Wait until the display is idle (busy pin = 1) before
                // continuing.
                if self.busy.is_low().ok().unwrap() {
                    return Err(nb::Error::WouldBlock);
                }

                // Assert reset for 100ms.
                self.reset.set_low().ok();
                self.timer.start(Hertz(10));

                self.init_state = InitState::Resetting1;
                self.init()
            }
            InitState::Resetting1 => {
                if let Err(nb::Error::WouldBlock) = self.timer.wait() {
                    return Err(nb::Error::WouldBlock);
                }

                // Wait 100ms for reinitialization.
                self.reset.set_high().ok();
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
                if self.busy.is_low().ok().unwrap() {
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
        self.data_cmd.set_low().ok();
        self.cs.set_low().ok();
        let command = command as u8;
        // TODO: Error handling?
        block!(self.spi.send(command)).ok();
        block!(self.spi.read()).ok();
        self.cs.set_high().ok();
    }

    fn send_data(&mut self, data: u8) {
        self.data_cmd.set_high().ok();
        self.cs.set_low().ok();
        // TODO: Error handling?
        block!(self.spi.send(data)).ok();
        block!(self.spi.read()).ok();
        self.cs.set_high().ok();
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
    Busy: embedded_hal::digital::v2::InputPin,
    Reset: embedded_hal::digital::v2::OutputPin,
    DataCmd: embedded_hal::digital::v2::OutputPin,
    CS: embedded_hal::digital::v2::OutputPin,
    Timer: embedded_hal::timer::CountDown<Time = Hertz>,
{
    const WIDTH: u32 = 400;
    const HEIGHT: u32 = 300;

    fn start_frame(&mut self) -> nb::Result<(), Error> {
        if self.busy.is_low().ok().unwrap() {
            return Err(nb::Error::WouldBlock);
        }
        self.send_command(DisplayCommand::DataStartTransmission1);
        self.delay_2ms();
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

    fn draw_row(&mut self, row: &[u8]) {
        assert!(row.len() >= Self::WIDTH as usize / 8);
        for i in 0..Self::WIDTH as usize / 8 {
            self.send_data(row[i]);
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
