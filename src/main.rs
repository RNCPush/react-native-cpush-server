use actix_web::{middleware::Logger, web, App, HttpServer};
use react_native_cpush_server::{api, log_conf, mdb_connection, CPUSH_CONFIG};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (_g1, _g2) = log_conf::init_tracing_subscriber(&["actix_web", "react_native_cpush_server"]);
    log_panics::init();
    let mdb = mdb_connection().await.unwrap();
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(mdb.clone()))
            .configure(api::configure)
    })
    .bind(("0.0.0.0", CPUSH_CONFIG.server_port))?
    .run()
    .await
}
