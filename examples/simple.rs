use std::sync::Arc;
use std::time::Duration;
use futures::executor::block_on;
use tokio::sync::Mutex;
use ifconnect::*;
use ifconnect::connection::Connection;

fn main() {
    // Create the runtime
    let rt  = tokio::runtime::Runtime::new().unwrap();

    // Spawn the root task
    rt.block_on(async {
        let mut conn = Connection::new();
        let instance = conn.listen_udp(&UDP_PORT, Some(Duration::from_secs(20))).unwrap(); // this hangs the process
        if instance.addresses.is_empty() { return; } // cancel if no addresses were supplied by the manifest

        /*let async_conn = Arc::<Mutex<Connection>>::new(Mutex::new(conn));
        tokio::spawn(async move {
            let mut conn = async_conn.clone().lock().await;

            loop {
                conn.update().await;
            }
        });*/

        conn.start_tcp(TCP_PORT_V2.clone(), instance.addresses[0].clone()).await;
        conn.run_event_loop();

        /*let conn = async_conn.clone().lock().await;
        conn.send_get_state(-1);*/

        let mut buf = String::from("");
        std::io::stdin().read_line(&mut buf);
    });
}