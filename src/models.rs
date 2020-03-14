use serde::Serialize;
use serde::Deserialize;
use tokio_pg_mapper_derive::PostgresMapper; //pg_mapper
use tokio_pg_mapper::FromTokioPostgresRow; //from_row_ref
use deadpool_postgres::Client;
use std::io;
use actix_web::error::ResponseError; //trait
use actix_web::http::StatusCode; //struct
use actix_web::HttpResponse; //struct

use config::ConfigError;

#[derive(Serialize)]
pub struct Status {
    pub status: String
}

#[derive(Serialize, Deserialize, PostgresMapper)]
#[pg_mapper(table="todo_list")]
pub struct TodoList {
    pub id:i32,
    pub title: String
}

#[derive(Serialize, Deserialize, PostgresMapper)]
#[pg_mapper(table="todo_item")]
pub struct TodoItem {
    pub id:i32,
    pub title:String,
    pub checked: bool,
    pub list_id: i32,
}

#[derive(Deserialize)]
pub struct CreateTodoList {
    pub title:String,
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub host :String,
    pub port :i32,
}

#[derive(Deserialize)]
pub struct Config {
    pub server :ServerConfig,
    pub pg: deadpool_postgres::Config,
}

#[derive(Serialize)]
pub struct ResultResponse {
    pub success : bool
}

#[derive(Debug)]
pub enum AppErrorType {
    DataBaseError,
    NotFoundError,
}

#[derive(Debug)]
pub struct AppError {
    pub message: Option<String>,
    pub cause :  Option<String>,
    pub error_type : AppErrorType
}

#[derive(Serialize)]
pub struct AppErrorHttpResponse {
    pub error: String
}

pub struct AppState {
    pub db_pool: deadpool_postgres::Pool,
    pub logger: slog::Logger,
}

impl AppError {
    fn message(&self) -> String {
        match self {
            AppError {
                message: Some(message),
                cause: _,
                error_type: _
            } => message.clone(),
            AppError {
                message: None,
                cause: _,
                error_type: AppErrorType::NotFoundError,
            } => "The requested item was not found".to_string(),
            _ => "An unexpected error has occured".to_string(),
        }
    
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(),std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self.error_type {
            AppErrorType::DataBaseError => StatusCode::INTERNAL_SERVER_ERROR,
            AppErrorType::NotFoundError => StatusCode::NOT_FOUND,
        }
    
    }
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .json(AppErrorHttpResponse{ error: self.message() })
    }
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut cfg = config::Config::new();
        cfg.merge(config::Environment::new())?;
        cfg.try_into()
    }
}

pub async fn get_items(client: &Client, list_id: i32) -> Result<Vec<TodoItem>, io::Error> {
    let statement = client.prepare("select * from todo_item where list_id = $1 order by id").await.unwrap();
    let items = client.query(&statement, &[&list_id])
        .await
        .expect("Error Getting items lists")
        .iter()
        .map(|row| TodoItem::from_row_ref(&row).unwrap())
        .collect::<Vec<TodoItem>>();
    Ok(items)
}

pub async fn get_todos(client: &Client) -> Result<Vec<TodoList>, AppError> {
    let statement = client.prepare("select * from todo_list order by id desc").await.unwrap();
    let todos = client.query(&statement, &[])
        .await
        .expect("Error Getting todo lists")
        .iter()
        .map(|row| TodoList::from_row_ref(&row).unwrap())
        .collect::<Vec<TodoList>>();
    Ok(todos)
}

pub async fn create_todo(client: &Client, title: String) -> Result<TodoList, io::Error> {
    let statement = client.prepare("insert into todo_list (title) values ($1) returning id, title").await.unwrap();
    client.query(&statement, &[&title])
        .await
        .expect("Error Creating todo lists")
        .iter()
        .map(|row| TodoList::from_row_ref(&row).unwrap())
        .collect::<Vec<TodoList>>()
        .pop()
        .ok_or(io::Error::new(io::ErrorKind::Other, "Error creating todo list"))
}

pub async fn check_item(client: &Client, list_id: i32, item_id: i32) -> Result<(), io::Error> {
    let statement = client.prepare("update todo_item set checked = true where list_id = $1 and id = $2 and checked = false").await.unwrap();
    let result = client.execute(&statement, &[&list_id, &item_id])
        .await
        .expect("Error checking todo list and items");
    match result {
        ref updated if *updated == 1 => Ok(()),
        _ => Err(io::Error::new(io::ErrorKind::Other, "Faile to check id"))
    }
}

#[cfg(test)]
mod model_test {
    #[test]
    fn error_message() {
        
    }
}
