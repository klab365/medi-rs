#![no_std]
#![no_main]

use core::{
    cell::RefCell,
    sync::atomic::{AtomicU32, Ordering},
};

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::{
    bind_interrupts,
    gpio::{Input, Level, Output, OutputDrive, Pull},
};
use embassy_sync::{
    blocking_mutex::{CriticalSectionMutex, raw::CriticalSectionRawMutex},
    mutex::Mutex as AsyncMutex,
};
use embassy_time::{Duration, Timer};
use medi_rs::{MediCommand, medi_handler, medi_module, medi_task, mediator};
use panic_probe as _;
use static_cell::StaticCell;

bind_interrupts!(
    struct Irqs {}
);

#[derive(MediCommand)]
#[medi_command(return_type = u32, error_type = medi_rs::Error)]
struct ButtonPressed;

#[derive(Clone)]
struct ButtonObserved {
    count: u32,
}

trait BoardPort: Sync {
    fn toggle_activity_led(&self);
    fn record_observed_press(&self, count: u32);
}

/// The board API made available to mediator handlers and tasks.
type BoardApi = &'static dyn BoardPort;
type ButtonInput = &'static AsyncMutex<CriticalSectionRawMutex, Input<'static>>;

struct EmbassyBoard {
    // P0.28 is an LED matrix column and P0.21 is a row on the micro:bit v2.
    activity_led: CriticalSectionMutex<RefCell<(Output<'static>, Output<'static>)>>,
    observed_count: &'static AtomicU32,
}

impl EmbassyBoard {
    fn new(column: Output<'static>, row: Output<'static>, observed_count: &'static AtomicU32) -> Self {
        Self {
            activity_led: CriticalSectionMutex::new(RefCell::new((column, row))),
            observed_count,
        }
    }
}

impl BoardPort for EmbassyBoard {
    fn toggle_activity_led(&self) {
        self.activity_led.lock(|leds| {
            let mut leds = leds.borrow_mut();
            if leds.0.is_set_low() {
                // A matrix pixel is on with its column low and row high.
                leds.0.set_high();
                leds.1.set_low();
            } else {
                leds.1.set_high();
                leds.0.set_low();
            }
        });
    }

    fn record_observed_press(&self, count: u32) {
        self.observed_count.store(count, Ordering::Relaxed);
    }
}

static PRESS_COUNT: AtomicU32 = AtomicU32::new(0);
static OBSERVED_COUNT: AtomicU32 = AtomicU32::new(0);
static BOARD: StaticCell<EmbassyBoard> = StaticCell::new();
static BUTTON_A: StaticCell<AsyncMutex<CriticalSectionRawMutex, Input<'static>>> = StaticCell::new();
static MEDIATOR: StaticCell<AppMediator> = StaticCell::new();

#[medi_handler]
async fn count_button_press(mediator: &AppMediator, board: BoardApi, _req: ButtonPressed) -> medi_rs::Result<u32> {
    board.toggle_activity_led();
    let count = PRESS_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
    mediator.publish(ButtonObserved { count }).await?;
    Ok(count)
}

#[medi_handler]
async fn observe_button_press(board: BoardApi, event: ButtonObserved) -> medi_rs::Result<()> {
    board.record_observed_press(event.count);
    Ok(())
}

#[medi_task]
async fn button_monitor(mediator: &AppMediator, button: ButtonInput) {
    loop {
        button.lock().await.wait_for_low().await;
        let count = mediator.send(ButtonPressed).await.unwrap();
        info!(
            "button A pressed: {}, observed: {}",
            count,
            OBSERVED_COUNT.load(Ordering::Relaxed)
        );
        Timer::after(Duration::from_millis(200)).await;
        button.lock().await.wait_for_high().await;
    }
}

#[medi_task]
async fn display(_mediator: &AppMediator, board: BoardApi) {
    loop {
        board.toggle_activity_led();
        Timer::after(Duration::from_millis(500)).await;
    }
}

medi_module! {
    manifest buttons_manifest;
    resources { BoardApi; ButtonInput; }
    tasks { button_monitor; display; }
    commands { ButtonPressed => count_button_press; }
    events { ButtonObserved => [observe_button_press]; }
}

mediator! {
    pub struct AppMediator {
        event_queue_capacity: 8;
        event_workers: 1;
        modules: [buttons_manifest];
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    let button: ButtonInput = BUTTON_A.init(AsyncMutex::new(Input::new(p.P0_14, Pull::Up)));
    let board: BoardApi = BOARD.init(EmbassyBoard::new(
        Output::new(p.P0_28, Level::High, OutputDrive::Standard),
        Output::new(p.P0_21, Level::Low, OutputDrive::Standard),
        &OBSERVED_COUNT,
    ));
    let mediator = MEDIATOR.init(AppMediator::new((board, button)));
    mediator.start(spawner);

    info!("medi-rs Embassy micro:bit v2 example started");
    core::future::pending::<()>().await;
}
