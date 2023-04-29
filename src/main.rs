use std::env;

use axum::{
    extract::State,
    http::StatusCode,
    response::Html,
    routing::{get, get_service},
    Router,
};

// use migration::{Migrator, MigratorTrait};
use sea_orm::{Database, DatabaseConnection};
use tera::Tera;
use tower_http::services::ServeDir;

#[derive(Clone)]
struct AppState {
    view: Tera,
    db: DatabaseConnection,
}

async fn home(state: State<AppState>) -> Result<Html<String>, (StatusCode, &'static str)> {
    let view = state
        .view
        .render("index.html", &tera::Context::new())
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Template error"))?;

    Ok(Html(view))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let current_dir = match env::current_dir() {
        Ok(cwd) => cwd.to_str().unwrap().to_string(),
        Err(e) => {
            println!("Could not retrieve CWD: {}", e);
            ::std::process::exit(1);
        }
    };

    let db_file = env::var("DATABASE_FILE").expect("DATABASE_FILE is not set in .env file");
    let host = env::var("HOST").expect("HOST is not set in .env file");
    let port = env::var("PORT").expect("PORT is not set in .env file");

    let db: DatabaseConnection = Database::connect(format!("sqlite://{}/{}", current_dir, db_file))
        .await
        .expect("Failed to connect to database");
    // Migrator::up(&db, None).await?;

    let mut view = match Tera::new(&format!("{}/resources/templates/**/*", current_dir)) {
        Ok(t) => t,
        Err(e) => {
            println!("Parsing error(s): {}", e);
            ::std::process::exit(1);
        }
    };

    match view.full_reload() {
        Ok(_) => {}
        Err(e) => {
            println!("Could not start full reload: {}", e);
            ::std::process::exit(1);
        }
    }

    let state = AppState { view, db };

    let app = Router::new()
        .route("/", get(home))
        .nest_service(
            "/public",
            get_service(ServeDir::new(format!("{}/public", current_dir))).handle_error(
                |error| async move {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled internal error: {error}"),
                    )
                },
            ),
        )
        .with_state(state);

    axum::Server::bind(&format!("{}:{}", host, port).parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
