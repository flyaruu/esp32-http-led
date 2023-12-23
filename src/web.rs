use alloc::sync::Arc;
use embassy_net::Stack;
use embassy_sync::{pubsub::Publisher, blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use embassy_time::{Duration, Timer};
use esp_println::println;
use esp_wifi::wifi::{WifiDevice, WifiStaDevice};
use picoserve::{Router, routing::{get, post}, response::IntoResponse, extract::State};

use crate::{shape::{Shape, Shapes}, QUEUE_CAP, SUBS_CAP, PUBS_CAP};

struct EmbassyTimer;

impl picoserve::Timer for EmbassyTimer {
    type Duration = embassy_time::Duration;
    type TimeoutError = embassy_time::TimeoutError;

    async fn run_with_timeout<F: core::future::Future>(
        &mut self,
        duration: Self::Duration,
        future: F,
    ) -> Result<F::Output, Self::TimeoutError> {
        embassy_time::with_timeout(duration, future).await
    }
}


#[derive(Clone)]
pub struct WebState {
    publisher: Arc<Mutex<NoopRawMutex,Publisher<'static, NoopRawMutex,Shape, QUEUE_CAP,SUBS_CAP,PUBS_CAP>>>,
}

#[embassy_executor::task]
pub async fn web_task(
    stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>,
    config: &'static picoserve::Config<Duration>,
    publisher: Publisher<'static, NoopRawMutex,Shape, QUEUE_CAP,SUBS_CAP,PUBS_CAP>,
) -> ! {
    let mut rx_buffer = [0; 1024];
    let mut tx_buffer = [0; 1024];

    let web_state = WebState{
        publisher: Arc::new(Mutex::new(publisher))
    };

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    println!("Waiting to get IP address...");
    loop {
        if let Some(config) = stack.config_v4() {
            println!("Got IP: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    loop {
        let mut socket = embassy_net::tcp::TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);

        log::info!("Listening on TCP:80...");
        if let Err(e) = socket.accept(80).await {
            log::warn!("accept error: {:?}", e);
            continue;
        }

        log::info!(
            "Received connection from {:?}",
            socket.remote_endpoint()
        );

        let (socket_rx, socket_tx) = socket.split();

        let app = Router::new()
            .route("/", get(get_root))
            .route("/shape", post(post_shape))
            .route("/shapes", post(post_shapes))
        ;

        match picoserve::serve_with_state(
            &app,
            EmbassyTimer,
            config,
            &mut [0; 2048],
            socket_rx,
            socket_tx,
            &web_state
            )
        .await
        {
            Ok(handled_requests_count) => {
                log::info!(
                    "{handled_requests_count} requests handled from {:?}",
                    socket.remote_endpoint()
                );
                socket.close();
            }
            Err(err) => log::error!("{err:?}"),
        }
    }
}

async fn get_root()-> impl IntoResponse {
    (("Connection","Close"),"hello world!")
}

async fn post_shape(State(state): State<WebState>, shape: Shape)-> impl IntoResponse {
    println!("Shape was: {:?}",shape);
    state.publisher
        .lock()
        .await
        .publish(shape)
        .await;
    (("Connection","Close"),"hello shape!")
}

async fn post_shapes(State(state): State<WebState>, Shapes(shapes): Shapes)-> impl IntoResponse {
    println!("Shape was: {:?}",shapes);
    for shape in shapes {
        state.publisher
        .lock()
        .await
        .publish(shape)
        .await;
    }
    (("Connection","Close"),"hello shape!")
}
