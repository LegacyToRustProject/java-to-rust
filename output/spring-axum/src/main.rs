/// Rust/Axum equivalent of gs-rest-service (Spring Boot greeting service).
///
/// Java original:
///   @RestController
///   public class GreetingController {
///     @GetMapping("/greeting")
///     public Greeting greeting(@RequestParam(defaultValue = "World") String name) {
///       return new Greeting(counter.incrementAndGet(), template.formatted(name));
///     }
///   }
use axum::{
    extract::Query,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Equivalent of Greeting.java record/POJO
#[derive(Serialize)]
struct Greeting {
    id: u64,
    content: String,
}

/// Equivalent of @RequestParam
#[derive(Deserialize)]
struct GreetingParams {
    #[serde(default = "default_name")]
    name: String,
}

fn default_name() -> String {
    "World".to_string()
}

/// Equivalent of GreetingController.greeting()
/// @GetMapping("/greeting") → .route("/greeting", get(greeting_handler))
async fn greeting_handler(
    axum::extract::State(counter): axum::extract::State<Arc<AtomicU64>>,
    Query(params): Query<GreetingParams>,
) -> Json<Greeting> {
    let id = counter.fetch_add(1, Ordering::SeqCst);
    let content = format!("Hello, {}!", params.name);
    Json(Greeting { id, content })
}

#[tokio::main]
async fn main() {
    let counter = Arc::new(AtomicU64::new(1));

    // Equivalent of Spring Boot's auto-configured router
    let app = Router::new()
        .route("/greeting", get(greeting_handler))
        .with_state(counter);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("Listening on http://0.0.0.0:8080");
    axum::serve(listener, app).await.unwrap();
}
