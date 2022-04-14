use server_lib::server;

#[rocket::main]
async fn main() {
    let _ = server::get_server().launch().await;
}
