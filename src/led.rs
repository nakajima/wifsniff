use core::iter::once;

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    gpio::{Io, Output, OutputPin},
    peripheral::Peripheral,
    peripherals::Peripherals,
    prelude::*,
    rmt::{PulseCode, Rmt, TxChannel, TxChannelConfig, TxChannelCreator},
    Blocking,
};
use esp_println::println;

const PULSES_PER_LED: usize = 24;
const CLOCK_MHZ: u32 = 80;
const T0H_NS: u32 = 350;
const T0L_NS: u32 = 800;
const T1H_NS: u32 = 700;
const T1L_NS: u32 = 600;

const T0: PulseCode = PulseCode {
    level1: true,
    length1: (T0H_NS * CLOCK_MHZ / 1000) as u16,
    level2: false,
    length2: (T0L_NS * CLOCK_MHZ / 1000) as u16,
};
const T1: PulseCode = PulseCode {
    level1: true,
    length1: (T1H_NS * CLOCK_MHZ / 1000) as u16,
    level2: false,
    length2: (T1L_NS * CLOCK_MHZ / 1000) as u16,
};

pub fn start_leds(io: Io, rmt: Rmt<Blocking>) {
    println!("starting LEDs");
    _ = Output::new_typed(io.pins.gpio20, esp_hal::gpio::Level::High);
    let delay = Delay::new();

    println!("starting cycle");
    let mut leds = NeoPixelDriver::<1, _>::new(rmt.channel0, io.pins.gpio9);
    (0..360).cycle().for_each(|hue| {
        println!("Hue: {}", hue);
        leds.write([Color::hsv(hue, 100, 20)]).unwrap();
        delay.delay_nanos(10_000_000);
    });

    unreachable!();
}

struct NeoPixelDriver<const LED_COUNT: usize, TX: TxChannel>
where
    [(); LED_COUNT * PULSES_PER_LED]:,
{
    channel: Option<TX>,
    buffer: [u32; LED_COUNT * PULSES_PER_LED],
}

impl<'pin, const LED_COUNT: usize, TX: TxChannel> NeoPixelDriver<LED_COUNT, TX>
where
    [(); LED_COUNT * PULSES_PER_LED]:,
{
    pub fn new<C, O>(channel: C, pin: impl Peripheral<P = O> + 'pin) -> Self
    where
        O: OutputPin + 'pin,
        C: TxChannelCreator<'pin, TX, O>,
    {
        let tx_config = TxChannelConfig {
            clk_divider: 1,
            ..TxChannelConfig::default()
        };
        let channel = channel.configure(pin, tx_config).unwrap();

        Self {
            channel: Some(channel),
            buffer: [0; LED_COUNT * PULSES_PER_LED],
        }
    }

    pub fn write<I>(&mut self, iterator: I) -> Result<(), esp_hal::rmt::Error>
    where
        I: IntoIterator<Item = Color>,
    {
        let mut channel = self.channel.take().unwrap();
        let mut chunks = iterator.into_iter().array_chunks::<LED_COUNT>();
        for chunk in chunks.by_ref() {
            for (code, color) in self
                .buffer
                .array_chunks_mut::<PULSES_PER_LED>()
                .zip(chunk.into_iter())
            {
                color.write_pulses(code);
            }
            // info!("Sending chunk");
            match channel.transmit(&self.buffer).wait() {
                Ok(ch) => channel = ch,
                Err((err, ch)) => {
                    self.channel = Some(ch);
                    log::error!("Error: {:?}", err);
                    return Err(err);
                }
            };
        }
        if let Some(color) = chunks.into_remainder() {
            self.buffer.fill(PulseCode::default().into());
            for (code, color) in self
                .buffer
                .array_chunks_mut::<PULSES_PER_LED>()
                .zip(color.into_iter())
            {
                color.write_pulses(code);
            }
            // info!("Sending remainder");
            channel = channel
                .transmit(&self.buffer)
                .wait()
                .map_err(|error| error.0)?;
        }
        self.channel = Some(channel);
        Ok(())
    }
}

#[derive(Clone, Default, Debug)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn hsv(h: u32, s: u32, v: u32) -> Self {
        if h > 360 || s > 100 || v > 100 {
            log::error!("Invalid HSV values");
            panic!("Invalid HSV values");
        }
        let s = s as f64 / 100.0;
        let v = v as f64 / 100.0;
        let c = s * v;
        let mut x = ((h as f64 / 60.0) % 2.0) - 1.0;
        if x < 0.0 {
            x = -x
        };
        let x = c * (1.0 - x);
        let m = v - c;
        let (r, g, b) = match h {
            0..=59 => (c, x, 0.0),
            60..=119 => (x, c, 0.0),
            120..=179 => (0.0, c, x),
            180..=239 => (0.0, x, c),
            240..=299 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        Self {
            r: ((r + m) * 255.0) as u8,
            g: ((g + m) * 255.0) as u8,
            b: ((b + m) * 255.0) as u8,
        }
    }

    pub fn write_pulses(&self, buf: &mut [u32; PULSES_PER_LED]) {
        const POSITIONS: [u8; 8] = [128, 64, 32, 16, 8, 4, 2, 1];
        let channels = [self.g, self.r, self.b];
        for (idx_channel, channel) in channels.iter().enumerate() {
            for (idx_position, position) in POSITIONS.iter().enumerate() {
                buf[POSITIONS.len() * idx_channel + idx_position] =
                    (if channel & position == 0 { T0 } else { T1 }).into();
            }
        }
    }
}
