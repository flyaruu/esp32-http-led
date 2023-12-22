#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(result_flattening)]
#![feature(async_fn_in_trait)]

extern crate alloc;
use core::mem::MaybeUninit;
use embassy_executor::{Executor, task};
use embassy_net::{Config, Stack, StackResources};
use embassy_time::Duration;
use embedded_graphics::{pixelcolor::{Rgb565, RgbColor}, mono_font::{MonoTextStyle, ascii::FONT_10X20}, text::{Text, Alignment}, geometry::Point, Drawable, draw_target::DrawTarget};
use esp_backtrace as _;
use esp_println::println;
use esp_wifi::{EspWifiInitFor, initialize, wifi::WifiStaDevice};
use hal::{clock::ClockControl, peripherals::Peripherals, prelude::*, IO, timer::TimerGroup, embassy, Rng, gdma::Gdma, spi::master::Spi, gpio::{NO_PIN, Output, PushPull, Gpio6}, dma::DmaPriority, Delay};
use static_cell::make_static;
use esp_backtrace as _;
use hal::spi::master::prelude::*;
use t_display_s3_amoled::rm67162::{Orientation, dma::RM67162Dma};


use crate::{net::{net_task, connection}, web::web_task};

mod net;
mod web;
mod shape;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();
    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}


#[entry]
fn main() -> ! {
    init_heap();
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();
    // let mut delay = Delay::new(&clocks);

    // setup logger
    // To change the log_level change the env section in .cargo/config.toml
    // or remove it and set ESP_LOGLEVEL manually before running cargo run
    // this requires a clean rebuild because of https://github.com/rust-lang/cargo/issues/10358
    esp_println::logger::init_logger_from_env();
    log::info!("Logger is setup");

    let io = IO::new(peripherals.GPIO,peripherals.IO_MUX);

    let mut delay = Delay::new(&clocks);
    let mut led = io.pins.gpio38.into_push_pull_output();
    //let user_btn = io.pins.gpio21.into_pull_down_input();
    //let boot0_btn = io.pins.gpio0.into_pull_up_input(); // default pull up

    led.set_high().unwrap();

    println!("GPIO init OK");

    println!("init display");

    let sclk = io.pins.gpio47;
    let rst = io.pins.gpio17;
    let cs = io.pins.gpio6;

    let d0 = io.pins.gpio18;
    let d1 = io.pins.gpio7;
    let d2 = io.pins.gpio48;
    let d3 = io.pins.gpio5;

    let mut cs = cs.into_push_pull_output();
    cs.set_high().unwrap();

    let mut rst = rst.into_push_pull_output();

    let dma = Gdma::new(peripherals.DMA);
    let dma_channel = dma.channel0;

    // Descriptors should be sized as (BUFFERSIZE / 4092) * 3
    let descriptors = make_static!([0u32; 12]);
    let spi = Spi::new_half_duplex(
        peripherals.SPI2, // use spi2 host
        Some(sclk),
        Some(d0),
        Some(d1),
        Some(d2),
        Some(d3),
        NO_PIN,       // Some(cs), NOTE: manually control cs
        75_u32.MHz(), // max 75MHz
        hal::spi::SpiMode::Mode0,
        &clocks,
    )
    .with_dma(dma_channel.configure(false, descriptors, &mut [], DmaPriority::Priority0));

    let mut display = t_display_s3_amoled::rm67162::dma::RM67162Dma::new(spi, cs);
    display.reset(&mut rst, &mut delay).unwrap();
    display.init(&mut delay).unwrap();
    display
        .set_orientation(Orientation::LandscapeFlipped)
        .unwrap();

    display.clear(Rgb565::YELLOW).unwrap();
    println!("screen init ok");

    let character_style = MonoTextStyle::new(&FONT_10X20, Rgb565::RED);
    Text::with_alignment(
        "Hello,\nRust World!",
        Point::new(300, 20),
        character_style,
        Alignment::Center,
    )
    .draw(&mut display)
    .unwrap();


    hal::interrupt::enable(hal::peripherals::Interrupt::GPIO, hal::interrupt::Priority::Priority1).unwrap();
    let executor = make_static!(Executor::new());
    let timer_group = TimerGroup::new(peripherals.TIMG0, &clocks);    
    // let timer = TimerGroup::new(peripherals.TIMG1, &clocks).timer0;
    let timer_group2 = TimerGroup::new(peripherals.TIMG1, &clocks);    

    embassy::init(&clocks,timer_group.timer0);

    let init = initialize(
        EspWifiInitFor::Wifi,
        timer_group2.timer0,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();

    let wifi = peripherals.WIFI;
    let (wifi_interface, controller) =
        esp_wifi::wifi::new_with_mode(&init, wifi, WifiStaDevice).unwrap();
    let config = Config::dhcpv4(Default::default());

    let seed = 1234; // very random, very secure seed

    // Init network stack
    let stack = &*make_static!(Stack::new(
        wifi_interface,
        config,
        make_static!(StackResources::<3>::new()),
        seed
    ));
    let pico_config = make_static!(picoserve::Config {
        start_read_request_timeout: Some(Duration::from_secs(5)),
        read_request_timeout: Some(Duration::from_secs(1)),
        write_timeout:  Some(Duration::from_secs(1)),
    });

    executor.run(|spawner| {
        spawner.spawn(connection(controller)).unwrap();
        spawner.spawn(net_task(stack)).unwrap();
        spawner.spawn(web_task(stack,pico_config)).unwrap();
        spawner.spawn(graphics(display)).unwrap();
    })
}


#[task]
async fn graphics(mut display: RM67162Dma<'static,Gpio6<Output<PushPull>>>) {

}