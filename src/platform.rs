use crate::controller::Controller;
use embassy_executor::Spawner;
use embassy_net::{Config, DhcpConfig, Stack, StackResources};
use embassy_time::Timer;
use embedded_svc::wifi::{ClientConfiguration, Configuration, Wifi};
use esp_wifi::wifi::{WifiController, WifiDevice, WifiEvent, WifiStaDevice, WifiState};
use hal::{
    clock::ClockControl, embassy, peripherals::Peripherals, prelude::*, systimer::SystemTimer,
};
use static_cell::make_static;

const DHCP_HOSTNAME: &str = env!("DHCP_HOSTNAME");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");
const WIFI_SSID: &str = env!("WIFI_SSID");

pub type NetworkStack = &'static Stack<WifiDevice<'static, WifiStaDevice>>;

#[derive(Debug)]
pub enum PlatformError {
    WifiError,
}

pub struct Platform<'a> {
    controller: Controller<'a>,
    network_stack: NetworkStack,
}

impl<'a> Platform<'a> {
    pub fn new(spawner: &Spawner) -> Result<Self, PlatformError> {
        let peripherals = Peripherals::take();
        let system = peripherals.SYSTEM.split();
        let clocks = ClockControl::max(system.clock_control).freeze();

        let timer_group0 = hal::timer::TimerGroup::new(peripherals.TIMG0, &clocks);
        embassy::init(&clocks, timer_group0.timer0);

        let systimer = SystemTimer::new(peripherals.SYSTIMER);
        let inited = esp_wifi::initialize(
            esp_wifi::EspWifiInitFor::Wifi,
            systimer.alarm0,
            hal::Rng::new(peripherals.RNG),
            system.radio_clock_control,
            &clocks,
        )
        .map_err(|error| {
            log::error!("Wifi initialization failed: {error:?}");
            PlatformError::WifiError
        })?;

        let device = peripherals.WIFI;
        let (wifi_interface, wifi_controller) =
            esp_wifi::wifi::new_with_mode(&inited, device, WifiStaDevice).map_err(|error| {
                log::error!("Wifi setup failed: {error:?}");
                PlatformError::WifiError
            })?;

        let config = {
            let mut config = DhcpConfig::default();
            config.hostname = Some(DHCP_HOSTNAME.into());
            Config::dhcpv4(config)
        };
        let seed = 0xDEADBEEFBABECAFEu64;
        let network_stack = &*make_static!(Stack::new(
            wifi_interface,
            config,
            make_static!(StackResources::<3>::new()),
            seed
        ));

        let controller = Controller::new(
            clocks,
            peripherals.GPIO,
            peripherals.IO_MUX,
            peripherals.UART0,
        );

        spawner
            .spawn(network_task(network_stack))
            .expect("task should start");
        spawner
            .spawn(wifi_task(wifi_controller))
            .expect("task should start");

        Ok(Self {
            controller,
            network_stack,
        })
    }

    pub fn get_controller(&mut self) -> &mut Controller<'a> {
        &mut self.controller
    }

    pub fn get_network_stack(&self) -> NetworkStack {
        self.network_stack
    }
}

#[embassy_executor::task]
async fn network_task(stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>) {
    stack.run().await
}

#[embassy_executor::task]
async fn wifi_task(mut controller: WifiController<'static>) {
    loop {
        // when connected, block until disconnected
        if matches!(esp_wifi::wifi::get_wifi_state(), WifiState::StaConnected) {
            // wait until we're no longer connected
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
            Timer::after_secs(5).await
        }

        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: WIFI_SSID.into(),
                password: WIFI_PASSWORD.into(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            log::info!("Starting wifi...");
            controller.start().await.unwrap();
            log::info!("Wifi started!");
        }

        log::info!("Connecting to wifi...");
        match controller.connect().await {
            Ok(()) => log::info!("Wifi connected!"),
            Err(e) => {
                log::error!("Failed to connect to wifi: {e:?}");
                Timer::after_secs(5).await
            }
        }
    }
}
