use rocket::tokio::time::{sleep, Duration};

#[get("/delay/<seconds>")]
pub async fn delay(seconds: u64) -> String {
    sleep(Duration::from_secs(seconds)).await;
    format!("Waited for {} seconds", seconds)
}

#[get("/")]
pub async fn index() -> &'static str {
    "Hello, world!"
}

#[get("/<name>")]
pub fn hello(name: &str) -> String {
    format!("Hello, {}!", name)
}
