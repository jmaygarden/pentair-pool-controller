#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_executor::Spawner;
use embassy_net::{Config, DhcpConfig, Stack, StackResources};
use embassy_time::Timer;
use embedded_svc::wifi::{ClientConfiguration, Configuration, Wifi};
use esp_backtrace as _;
use esp_println::println;
use esp_wifi::wifi::{WifiController, WifiDevice, WifiEvent, WifiStaDevice, WifiState};
use hal::{
    clock::ClockControl, embassy, peripherals::Peripherals, prelude::*, systimer::SystemTimer,
};
use static_cell::make_static;

const HOSTNAME: &str = env!("DHCP_HOSTNAME");
const PASSWORD: &str = env!("WIFI_PASSWORD");
const SSID: &str = env!("WIFI_SSID");

#[derive(Debug)]
enum InitError {
    WifiError,
}

fn init(spawner: &Spawner) -> Result<(), InitError> {
    esp_println::logger::init_logger_from_env();
    log::info!("Logger is setup");

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
        InitError::WifiError
    })?;

    let device = peripherals.WIFI;
    let (wifi_interface, controller) =
        esp_wifi::wifi::new_with_mode(&inited, device, WifiStaDevice).map_err(|error| {
            log::error!("Wifi setup failed: {error:?}");
            InitError::WifiError
        })?;

    let config = {
        let mut config = DhcpConfig::default();
        config.hostname = Some(HOSTNAME.into());
        Config::dhcpv4(config)
    };
    let seed = 0xDEADBEEFBABECAFEu64;
    let network_stack = &*make_static!(Stack::new(
        wifi_interface,
        config,
        make_static!(StackResources::<3>::new()),
        seed
    ));

    spawner.spawn(network_task(network_stack)).ok();
    spawner.spawn(wifi_task(controller)).ok();

    Ok(())
}

#[main]
async fn main(spawner: Spawner) {
    init(&spawner).expect("platform initialization should succeed");

    loop {
        println!("Loop...");
        Timer::after_secs(10u64).await;
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
                ssid: SSID.into(),
                password: PASSWORD.into(),
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
