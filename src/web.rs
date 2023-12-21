use embassy_net::Stack;
use embassy_time::{Duration, Timer};
use esp_println::println;
use esp_wifi::wifi::{WifiDevice, WifiStaDevice};
use picoserve::{Router, routing::get, response::IntoResponse};

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


#[embassy_executor::task]
pub async fn web_task(
    stack: &'static Stack<WifiDevice<'static, WifiStaDevice>>,
    config: &'static picoserve::Config<Duration>,
) -> ! {
    let mut rx_buffer = [0; 1024];
    let mut tx_buffer = [0; 1024];

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
        ;
        match picoserve::serve(
            &app,
            EmbassyTimer,
            config,
            &mut [0; 2048],
            socket_rx,
            socket_tx,
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


