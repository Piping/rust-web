mod models;
use crate::models::Status;
use crate::models::AppErrorType;
use crate::models::AppError;
use crate::models::AppState;

use dotenv::dotenv;
use tokio_postgres::NoTls;
use actix_web::{HttpServer, App, web, Responder, HttpResponse};
use std::io;

use slog::{Logger, o, info, crit, Drain};
use slog_term;
use slog_async;

type ServiceState = actix_web::web::Data<AppState>;

fn get_term_logger() -> Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let console_output = slog_term::FullFormat::new(decorator).build().fuse();
    let console_output = slog_async::Async::new(console_output).build().fuse();
    slog::Logger::root(console_output, o!("v" => env!("CARGO_PKG_VERSION")))
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    dotenv().ok();
    let config = crate::models::Config::from_env().unwrap();
    let pool = config.pg.create_pool(NoTls).unwrap();
    let logger = get_term_logger();
    info!(logger,"Server WILL listen at http://{}:{}", config.server.host, config.server.port);
    HttpServer::new(move || {
        App::new()
            .data(AppState {
                db_pool: pool.clone(),
                logger: logger.clone(),
            })
            .route("/", web::get().to(status))
            .route("/todos{_:/?}", web::get().to(get_todos))
            .route("/todos{_:/?}", web::post().to(create_todo))
            .route("/todos/{list_id}/items{_:/?}", web::get().to(get_items))
            .route("/todos/{list_id}/items/{item_id}{_:/?}", web::put().to(check_item))
    })
    .bind(format!("{}:{}", config.server.host, config.server.port))?
    .run()
    .await
}

async fn status() -> impl Responder {
    web::HttpResponse::Ok()
        .json(Status{ status: "Normal".to_string()})
}

async fn get_todos(app: ServiceState) ->  Result<impl Responder, AppError> {
    let log = app.logger.new(o!("handler" => "get_todos"));
    let client = app.db_pool.get().await.map_err(|err|{
        let sublog = log.new(o!("cause" => err.to_string()));
        crit!(sublog, "Error creating client");
        AppError { message: None, cause: Some(err.to_string()), error_type: AppErrorType::DataBaseError}
    })?;
    models::get_todos(&client).await.map(|todos| HttpResponse::Ok().json(todos))
}

async fn get_items(app: ServiceState, path: web::Path<(i32,)>) -> impl Responder {
    let client = app.db_pool.get().await.expect("Error connecting to the database");
    let result = models::get_items(&client, path.0).await;
    match result {
        Ok(items) => HttpResponse::Ok().json(items),
        Err(_) => HttpResponse::InternalServerError().into()
    }
}

async fn create_todo(app: ServiceState, json_body: web::Json<crate::models::CreateTodoList>) -> impl Responder {
    let client = app.db_pool.get().await.expect("Error connecting to the database");
    let result = models::create_todo(&client, json_body.title.clone()).await;
    match result {
        Ok(todo) => HttpResponse::Ok().json(todo),
        Err(_) => HttpResponse::InternalServerError().into()
    }
}

async fn check_item(app: ServiceState, path: web::Path<(i32,i32)>) -> impl Responder {
    let client = app.db_pool.get().await.expect("Error connecting to the database");
    let result = models::check_item(&client, path.0, path.1).await;
    match result {
        Ok(()) => HttpResponse::Ok().json(models::ResultResponse{success: true}),
        Err(ref e) if e.kind() == io::ErrorKind::Other => HttpResponse::Ok().json(models::ResultResponse{success: false}),
        Err(_) => HttpResponse::InternalServerError().into()
    }
}



#[cfg(test)]
mod main {
    #[test]
    fn it_works() -> Result<(), String> {
        if 2 + 3 == 4 {
            Ok(())
        } else {
            Err(String::from("two plus two does not equal four"))
        }
    }
}

