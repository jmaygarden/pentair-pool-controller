#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy_executor::Spawner;
use embassy_time::Timer;
use esp_backtrace as _;
use esp_println::println;
use hal::{
    clock::ClockControl, embassy, peripherals::Peripherals, prelude::*, systimer::SystemTimer,
};

#[main]
async fn main(_spawner: Spawner) {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();
    let systimer = SystemTimer::new(peripherals.SYSTIMER);
    let timer_group0 = hal::timer::TimerGroup::new(peripherals.TIMG0, &clocks);

    esp_println::logger::init_logger_from_env();
    log::info!("Logger is setup");

    let _init = esp_wifi::initialize(
        esp_wifi::EspWifiInitFor::Wifi,
        systimer.alarm0,
        hal::Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();

    embassy::init(&clocks, timer_group0.timer0);

    loop {
        println!("Loop...");
        Timer::after_secs(1u64).await;
    }
}
