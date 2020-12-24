#![no_std]
#![no_main]

use core::fmt::Write;
use core::sync::atomic::compiler_fence;
use core::sync::atomic::Ordering::SeqCst;
use cortex_m::interrupt;
use embedded_hal::digital::v1_compat::OldOutputPin;
use embedded_hal::digital::v2::OutputPin;
use enc28j60::Enc28j60;
use rtt_target::{rprintln, rtt_init, ChannelMode, CriticalSectionFunc, UpChannel};
use stm32f0xx_hal::{delay::Delay, pac::Peripherals};
use stm32f0xx_hal::{prelude::*, spi::Spi};

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    interrupt::disable();

    if let Some(mut channel) = unsafe { UpChannel::conjure(0) } {
        channel.set_mode(ChannelMode::BlockIfFull);

        writeln!(channel, "{}", info).ok();
    }

    loop {
        compiler_fence(SeqCst);
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let channels = rtt_init! {
        up: {
            0: {
                size: 1024
                mode: NoBlockSkip
                name: "Terminal"
            }
        }
    };
    unsafe {
        rtt_target::set_print_channel_cs(
            channels.up.0,
            &((|arg, f| interrupt::free(|_| f(arg))) as CriticalSectionFunc),
        );
    }

    rprintln!("RTT initialized");

    let cp = cortex_m::Peripherals::take().unwrap();
    let mut dp = Peripherals::take().unwrap();
    let mut rcc = dp.RCC.configure().sysclk(8.mhz()).freeze(&mut dp.FLASH);
    let gpioa = dp.GPIOA.split(&mut rcc);

    let (spi1_pins, mut cs, mut rst) = cortex_m::interrupt::free(move |cs| {
        (
            (
                gpioa.pa5.into_alternate_af0(cs), // SCK
                gpioa.pa6.into_alternate_af0(cs), // MISO
                gpioa.pa7.into_alternate_af0(cs), // MOSI
            ),
            gpioa.pa4.into_push_pull_output(cs), // CS
            gpioa.pa3.into_push_pull_output(cs), // RST
        )
    });
    cs.set_high().unwrap();
    rst.set_high().unwrap();

    let mut spi = Spi::spi1(dp.SPI1, spi1_pins, enc28j60::MODE, 10.khz(), &mut rcc);
    cs.set_low().unwrap();
    spi.write(&[0x12, 0x34, 0x56, 0x78]);
    cs.set_high().unwrap();

    loop {
        compiler_fence(SeqCst);
    }
}
