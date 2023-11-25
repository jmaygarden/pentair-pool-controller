use hal::{
    clock::Clocks,
    gpio::{AnyPin, Output, PushPull},
    peripherals::{GPIO, IO_MUX, UART0},
    prelude::_embedded_hal_digital_v2_OutputPin,
    uart::{
        self,
        config::{Config, DataBits, StopBits},
        TxRxPins,
    },
    Uart, IO,
};

#[derive(Debug)]
pub enum Error {
    Uart(uart::Error),
}

pub struct Controller<'a> {
    tx_enable: AnyPin<Output<PushPull>>,
    uart: Uart<'a, UART0>,
}

impl<'a> Controller<'a> {
    pub fn new(clocks: Clocks<'a>, gpio: GPIO, io_mux: IO_MUX, uart: UART0) -> Self {
        let config = Config::default()
            .baudrate(115_200)
            .data_bits(DataBits::DataBits8)
            .parity_none()
            .stop_bits(StopBits::STOP1);
        let io = IO::new(gpio, io_mux);
        let tx_enable = io.pins.gpio18.into_push_pull_output().into();
        let tx = io.pins.gpio21.into_push_pull_output();
        let rx = io.pins.gpio20.into_push_pull_output();
        let pins = TxRxPins::new_tx_rx(tx, rx);
        let uart = Uart::new_with_config(uart, config, Some(pins), &clocks);

        Self { tx_enable, uart }
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        use embedded_io_async::Read;
        self.uart.read(buf).await.map_err(Error::Uart)
    }

    pub async fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        use embedded_io_async::Write;
        self.tx_enable.set_high().expect("infallible");
        let result = match self.uart.write(buf).await {
            Ok(len) => self.uart.flush().await.and(Ok(len)),
            Err(err) => Err(err),
        };
        self.tx_enable.set_low().expect("infallible");

        result.map_err(Error::Uart)
    }
}
