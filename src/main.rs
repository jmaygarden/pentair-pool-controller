#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

mod controller;
mod handler;
mod platform;
mod server;

use crate::{platform::Platform, server::server_task};
use core::mem::MaybeUninit;
use embassy_executor::Spawner;
use esp_alloc::EspHeap;
use esp_backtrace as _;
use hal::prelude::*;

#[global_allocator]
static ALLOCATOR: EspHeap = EspHeap::empty();

fn setup_allocator() {
    const HEAP_SIZE: usize = 65_536;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

fn setup_logging() {
    esp_println::logger::init_logger_from_env();
    log::info!("Logger is setup");
}

#[main]
async fn main(spawner: Spawner) {
    setup_allocator();
    setup_logging();

    let platform = Platform::new(&spawner).expect("platform should initialize");

    server_task(platform).await;
}
