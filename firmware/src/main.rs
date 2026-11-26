//! RP2040 を USB Audio 1.0（スピーカ）+ CDC ACM の複合デバイスにし、
//! 受けた音声を CDC でハーネスへ返すファーム。
//!
//! 振る舞いはすべて `capture_core::device::Device` にある（ADR-0005）。
//! ここは USB のイベントを Device に渡し、Device が作ったリンクフレームを CDC へ書くだけ。
//! 設計: doc/system-context.md、doc/context/audio-in/、doc/context/capture-link/

#![no_std]
#![no_main]

use core::cell::RefCell;

use capture_core::audio_in::{FEEDBACK_48K, HostControls};
use capture_core::device::Device;
use capture_core::link::MAX_FRAME_LEN;
use embassy_executor::Spawner;
use embassy_futures::select::select;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{self, CdcAcmClass};
use embassy_usb::class::uac1::speaker::{self, Speaker, Volume};
use embassy_usb::class::uac1::{Channel, FeedbackRefresh, SampleWidth};
use embassy_usb::driver::EndpointError;
use embassy_usb::{Builder, Handler, UsbDevice};
use static_cell::StaticCell;
use {panic_halt as _, embassy_rp as _};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

type UsbDriver = Driver<'static, USB>;

/// pid.codes の試験用 VID/PID。配布する前に正式な番号へ替える（Q-0004）。
const VID: u16 = 0x1209;
const PID: u16 = 0x0001;
/// 48 kHz / 2 ch / 16 bit は 1 ms あたり通常 192 B、フィードバックの調整で最大 196 B。余裕を見て倍にする。
const AUDIO_MAX_PACKET: u16 = 392;
const CDC_MAX_PACKET: u16 = 64;

static CHANNELS: [Channel; 2] = [Channel::LeftFront, Channel::RightFront];

static DEVICE: Mutex<CriticalSectionRawMutex, RefCell<Device>> = Mutex::new(RefCell::new(Device::new()));
/// Device に送るものができたかもしれない
static TX_READY: Signal<CriticalSectionRawMutex, ()> = Signal::new();

fn with_device<R>(f: impl FnOnce(&mut Device) -> R) -> R {
    DEVICE.lock(|d| f(&mut d.borrow_mut()))
}

/// USB リセットを Device に伝える。
struct UsbEvents;

impl Handler for UsbEvents {
    fn reset(&mut self) {
        with_device(Device::usb_reset);
        TX_READY.signal(());
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let driver = Driver::new(p.USB, Irqs);

    let mut config = embassy_usb::Config::new(VID, PID);
    config.manufacturer = Some("E2E-Audio-Test-Module");
    config.product = Some("E2E Audio Capture");
    config.serial_number = Some("E2EAC-0001");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    static CONFIG_DESCRIPTOR: StaticCell<[u8; 512]> = StaticCell::new();
    static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
    let mut builder = Builder::new(
        driver,
        config,
        CONFIG_DESCRIPTOR.init([0; 512]),
        BOS_DESCRIPTOR.init([0; 256]),
        &mut [],
        CONTROL_BUF.init([0; 64]),
    );

    static EVENTS: StaticCell<UsbEvents> = StaticCell::new();
    builder.handler(EVENTS.init(UsbEvents));

    static SPEAKER_STATE: StaticCell<speaker::State> = StaticCell::new();
    let (stream, feedback, monitor) = Speaker::new(
        &mut builder,
        SPEAKER_STATE.init(speaker::State::new()),
        AUDIO_MAX_PACKET,
        SampleWidth::Width2Byte,
        &[48_000],
        &CHANNELS,
        FeedbackRefresh::Period8Frames,
    );

    static CDC_STATE: StaticCell<cdc_acm::State> = StaticCell::new();
    let cdc = CdcAcmClass::new(&mut builder, CDC_STATE.init(cdc_acm::State::new()), CDC_MAX_PACKET);
    let (tx, rx) = cdc.split();

    let usb = builder.build();

    spawner.spawn(usb_task(usb).unwrap());
    spawner.spawn(audio_task(stream).unwrap());
    spawner.spawn(feedback_task(feedback).unwrap());
    spawner.spawn(control_task(monitor).unwrap());
    spawner.spawn(cdc_rx_task(rx).unwrap());
    spawner.spawn(cdc_tx_task(tx).unwrap());
}

#[embassy_executor::task]
async fn usb_task(mut usb: UsbDevice<'static, UsbDriver>) -> ! {
    usb.run().await
}

/// AUD-1, AUD-2: ストリームの開閉と音声パケットの受信。
#[embassy_executor::task]
async fn audio_task(mut stream: speaker::Stream<'static, UsbDriver>) -> ! {
    let mut buf = [0u8; AUDIO_MAX_PACKET as usize];
    loop {
        stream.wait_connection().await;
        with_device(|d| d.set_alt_setting(1)).ok();
        loop {
            match stream.read_packet(&mut buf).await {
                Ok(n) => {
                    with_device(|d| d.iso_out(&buf[..n]));
                    TX_READY.signal(());
                }
                Err(EndpointError::Disabled) => break,
                // 最大長を超えるパケットは受け取れない。捨てて次を待つ。
                Err(EndpointError::BufferOverflow) => {}
            }
        }
        with_device(|d| d.set_alt_setting(0)).ok();
    }
}

/// AUD-3: フィードバック値を返し続ける。
#[embassy_executor::task]
async fn feedback_task(mut feedback: speaker::Feedback<'static, UsbDriver>) -> ! {
    loop {
        feedback.wait_connection().await;
        loop {
            let value = with_device(|d| d.feedback()).unwrap_or(FEEDBACK_48K);
            if feedback.write_packet(&value).await.is_err() {
                break;
            }
        }
    }
}

/// ホストの音量とミュートを記録する（適用はしない、ADR-0003）。
#[embassy_executor::task]
async fn control_task(monitor: speaker::ControlMonitor<'static>) -> ! {
    let mut controls = HostControls::default();
    loop {
        monitor.changed().await;
        for (i, &ch) in CHANNELS.iter().enumerate() {
            match monitor.volume(ch) {
                Some(Volume::Muted) => controls.muted[i] = true,
                Some(Volume::DeciBel(db)) => {
                    controls.muted[i] = false;
                    controls.volume_8q8_db[i] = (db * 256.0) as i16;
                }
                None => {}
            }
        }
        with_device(|d| d.set_host_controls(controls));
    }
}

/// CAP-1, CAP-3, CAP-4: ハーネスからのコマンド行。
#[embassy_executor::task]
async fn cdc_rx_task(mut rx: cdc_acm::Receiver<'static, UsbDriver>) -> ! {
    let mut buf = [0u8; CDC_MAX_PACKET as usize];
    loop {
        rx.wait_connection().await;
        while let Ok(n) = rx.read_packet(&mut buf).await {
            with_device(|d| d.cdc_rx(&buf[..n]));
            TX_READY.signal(());
        }
    }
}

/// CAP-2: Device が作ったリンクフレームを CDC へ書く。DTR の変化も Device に伝える。
#[embassy_executor::task]
async fn cdc_tx_task(mut tx: cdc_acm::Sender<'static, UsbDriver>) -> ! {
    let mut frame = [0u8; MAX_FRAME_LEN];
    loop {
        tx.wait_connection().await;
        'connected: loop {
            let dtr = tx.dtr();
            let n = with_device(|d| {
                d.set_dtr(dtr);
                if dtr { d.poll_tx(&mut frame) } else { 0 }
            });
            if n == 0 {
                // 送るものが無い。新しい入力か、DTR を見直す時間まで待つ。
                select(TX_READY.wait(), Timer::after_millis(5)).await;
                continue;
            }
            for chunk in frame[..n].chunks(CDC_MAX_PACKET as usize) {
                if tx.write_packet(chunk).await.is_err() {
                    break 'connected;
                }
            }
            // ちょうど最大パケット長で終わったら、転送の終わりを ZLP で知らせる。
            if n % CDC_MAX_PACKET as usize == 0 && tx.write_packet(&[]).await.is_err() {
                break 'connected;
            }
        }
    }
}
