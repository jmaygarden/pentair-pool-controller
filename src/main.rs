#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

mod controller;
mod platform;
mod server;

use crate::{platform::Platform, server::server_task};
use embassy_executor::Spawner;
use esp_backtrace as _;
use hal::prelude::*;

#[main]
async fn main(spawner: Spawner) {
    esp_println::logger::init_logger_from_env();
    log::info!("Logger is setup");

    let platform = Platform::setup(&spawner).expect("platform should initialize");

    server_task(platform.get_network_stack()).await;
}
