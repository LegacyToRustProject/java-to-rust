use axum::{
    extract::{Path, State, Json},
    routing::{get, post, delete},
    Router,
};

async fn get_all_users(
    State(user_service): State<UserService>,
) -> Json<Vec<User>> {
    Json(user_service.find_all().await)
}

async fn get_user_by_id(
    State(user_service): State<UserService>,
    Path(id): Path<i64>,
) -> Json<User> {
    Json(user_service.find_by_id(id).await)
}

async fn create_user(
    State(user_service): State<UserService>,
    Json(user): Json<User>,
) -> Json<User> {
    Json(user_service.save(user).await)
}

async fn delete_user(
    State(user_service): State<UserService>,
    Path(id): Path<i64>,
) {
    user_service.delete_by_id(id).await;
}

fn user_routes() -> Router<UserService> {
    Router::new()
        .route("/api/users", get(get_all_users).post(create_user))
        .route("/api/users/{id}", get(get_user_by_id).delete(delete_user))
}
