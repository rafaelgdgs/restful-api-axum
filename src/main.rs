use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use sqlx::{PgPool, postgres::PgPoolOptions};

use tokio::net::TcpListener;

use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Unable to access .env file.");

    let server_address: String = std::env::var("SERVER_ADDRESS").unwrap_or("127.0.0.".to_owned());
    let database_url: String =
        std::env::var("DATABASE_URL").expect("DATAABSE_URL not found in .env file");

    let db_pool = PgPoolOptions::new()
        .max_connections(16)
        .connect(&database_url)
        .await
        .expect("Can't connect to database.");

    let listener = TcpListener::bind(server_address)
        .await
        .expect("Couldn't create TCP Listener");

    println!("Listening on {}", listener.local_addr().unwrap());

    let app = Router::new()
        .route("/", get(|| async { "Hello, World " }))
        .route("/tasks", get(get_tasks).post(create_task))
        .route("/tasks/{task_id}", patch(update_task).delete(delete_task))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .with_state(db_pool);

    axum::serve(listener, app)
        .await
        .expect("Error serving the application.");
}

#[derive(OpenApi)]
#[openapi(paths(get_tasks, create_task, delete_task))]
pub struct ApiDoc;

#[derive(Serialize)]
struct TaskRow {
    task_id: i32,
    name: String,
    priority: Option<i32>,
}

#[utoipa::path(
    get,
    path = "/tasks",
    responses(
    (status = 200, description = "Show all tasks"),
    (status = 500, description = "Failed to get psql SELECT query")
    )
)]
async fn get_tasks(
    State(pg_pool): State<PgPool>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let rows = sqlx::query_as!(TaskRow, "SELECT * FROM tasks ORDER BY task_id")
        .fetch_all(&pg_pool)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "message": e.to_string()})),
            )
        })?;
    Ok((
        StatusCode::OK,
        Json(json!({ "success": true, "data": rows })),
    ))
}

#[derive(Deserialize, ToSchema)]
struct CreateTaskReq {
    name: String,
    priority: Option<i32>,
}

#[derive(Serialize)]
struct CreateTaskRow {
    task_id: i32,
}

#[utoipa::path(
    post,
    path = "/tasks",
    responses(
    (status = 201, description = "created user"),
    (status = 500, description = "failed to create user")
    )
)]
async fn create_task(
    State(pg_pool): State<PgPool>,
    Json(task): Json<CreateTaskReq>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let row = sqlx::query_as!(
        CreateTaskRow,
        "INSERT INTO tasks (name, priority) VALUES ($1, $2) RETURNING task_id",
        task.name,
        task.priority
    )
    .fetch_one(&pg_pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": e.to_string()})),
        )
    })?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "success": true, "data": row})),
    ))
}

async fn update_task(
    State(_pg_pool): State<PgPool>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    todo!()
}

#[utoipa::path(
    delete,
    path = "/tasks/{id}",
    responses(
        (status = 200, description = "Deleted user with success"),
        (status = 404, description = "Failed to find user id")
    )
)]
async fn delete_task(
    State(pg_pool): State<PgPool>,
    Path(task_id): Path<i32>,
) -> Result<(StatusCode, Json<Value>), (StatusCode, Json<Value>)> {
    let _row = sqlx::query!("DELETE FROM tasks WHERE task_id = $1", task_id)
        .execute(&pg_pool)
        .await
        .map_err(|e| {
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "success": false, "message": e.to_string()})),
            )
        })?;
    Ok((StatusCode::OK, Json(json!({ "success": true}))))
}
