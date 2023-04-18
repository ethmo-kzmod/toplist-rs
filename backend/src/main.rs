use axum::
{
    extract::{Extension, Path},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::mysql::MySqlPool;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Deserialize)]
struct Params
{
    checkpoints: bool,
    map: String,
    course: String,
    courseid: i32,
    rflag: i8,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
struct Maps
{
    maps: Vec<Map>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
struct Map
{
    map_name: String,
    times_played: Option<i32>,
    time_added: Option<String>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
struct CourseNames
{
    course_count: usize,
    course_names: Vec<CourseName>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
struct CourseName
{
    course_name: String,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
struct Courses
{
    course_count: usize,
    courses: Vec<Course>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
struct Course
{
    course_id: i32,
    course_name: String,
    mapfk: String,
    reverse: i8,
}

#[derive(Serialize, Deserialize)]
struct Records
{
    records_count: usize,
    records: Vec<Record>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
struct Record
{
    player_name: String,
    steamid: String,
    course_time: String,
    date: Option<String>,
    diff: String,
}

enum ApiError
{
    NotFound,
    DatabaseError(sqlx::Error),
}

impl IntoResponse for ApiError
{
    fn into_response(self) -> Response
    {
        let (status, err_msg) = match self
        {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "These aren't the droids you're looking for."),
            ApiError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error."),
        };
        (status, Json(json!({ "Error": err_msg }))).into_response()
    }
}

impl From<sqlx::Error> for ApiError
{
    fn from(e: sqlx::Error) -> Self
    {
        ApiError::DatabaseError(e)
    }
}

async fn test() -> Result<(), ApiError>
{
    Err(ApiError::NotFound)
}

#[tokio::main]
async fn main()
{
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "toplist_api=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cors = CorsLayer::new().allow_origin(Any);
    let pool = MySqlPool::connect(&std::env::var("DATABASE_URL").unwrap())
        .await
        .expect("Unable to connect to database.");
    let app = Router::new()
        .fallback(test)
        .route("/", get(|| async { "Hello, Sailor!" }))
        .route("/api/maps", get(get_maps))
        .route("/api/:map", get(get_course_names))
        .route("/api/:map/:course", get(get_course_data))
        .route("/api/records/:checkpoints/:map/:course/:courseid/:rflag", get(get_records))
        .layer(cors)
        .layer(Extension(pool));

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("Failed to start server");
}

#[axum_macros::debug_handler]
async fn get_records(Extension(pool): Extension<MySqlPool>, Path(Params { checkpoints, map, course, courseid, rflag }): Path<Params>) -> Result<Json<Records>, ApiError>
{
    let mut map_records: Vec<Record> = if rflag == 0 && !checkpoints
    {
        sqlx::query_as!(Record, r#"SELECT p.playername AS player_name, p.steamid AS steamid, r.course_time AS course_time, DATE_FORMAT(r.date_set, '%Y-%m-%d') AS date, r.course_time AS diff
                                FROM player p, record r, course c, map m
                                WHERE r.steamidfk = p.steamid AND c.course_name=? AND r.courseidfk=? AND m.map_name=? AND c.mapfk=?
                                ORDER BY r.course_time ASC, r.date_set ASC, r.record_key ASC"#, course, courseid, map, map).fetch_all(&pool).await?
    }
    else if rflag == 0 && checkpoints
    {
        sqlx::query_as!(Record, r#"SELECT p.playername AS player_name, p.steamid AS steamid, r.course_time AS course_time, DATE_FORMAT(r.date_set, '%Y-%m-%d') AS date, r.course_time AS diff
                                FROM player p, record_cp r, course c, map m
                                WHERE r.steamidfk = p.steamid AND c.course_name=? AND r.courseidfk=? AND m.map_name=? AND c.mapfk=?
                                ORDER BY r.course_time ASC, r.date_set ASC, r.record_key ASC"#, course, courseid, map, map).fetch_all(&pool).await?
    }
    //NOTE: Reverse course are unlikely to be played with checkpoints
    else
    {
        sqlx::query_as!(Record, r#"SELECT p.playername AS player_name, p.steamid AS steamid, r.course_time AS course_time, DATE_FORMAT(r.date_set, '%Y-%m-%d') AS date, r.course_time AS diff
                                FROM player p, record r, course c, map m
                                WHERE r.steamidfk = p.steamid AND c.course_name=? AND r.courseidfk=? AND m.map_name=? AND c.mapfk=?
                                ORDER BY r.course_time DESC, r.date_set ASC, r.record_key ASC"#, course, courseid, map, map).fetch_all(&pool).await?
    };

    if !map_records.is_empty()
    {
        let record_time = NaiveTime::parse_from_str(&map_records[0].course_time, "%H:%M:%S.%f").unwrap();

        for (index, record) in map_records.iter_mut().enumerate()
        {
            let time = NaiveTime::parse_from_str(&record.course_time, "%H:%M:%S.%f").unwrap();
            let duration = NaiveTime::signed_duration_since(time, record_time).to_std().unwrap();
            let hours = (duration.as_secs() / 60) / 60;
            let minutes = (duration.as_secs() / 60) % 60;
            let seconds = duration.as_secs() % 60;
            let milliseconds = duration.subsec_nanos() as u64 % 999999900;
            let difference = format!("+{:02}:{:02}:{:02}.{:02}", hours, minutes, seconds, milliseconds);

            if index == 0
            {
                record.diff = "WR".to_string();
            }
            else
            {
                record.diff = difference;
            }
        }
    }

    Ok(Json(Records {
        records_count: map_records.len(),
        records: map_records,
    }))
}

#[axum_macros::debug_handler]
async fn get_maps(Extension(pool): Extension<MySqlPool>) -> Result<Json<Maps>, ApiError>
{
    let maps_list : Vec<Map> = sqlx::query_as!(Map, r#"SELECT map_name, times_played, DATE_FORMAT(map.time_added, '%Y-%m-%d') AS time_added FROM map"#).fetch_all(&pool).await?;

    Ok(Json(Maps { maps: maps_list }))
}

#[axum_macros::debug_handler]
async fn get_course_names(Extension(pool): Extension<MySqlPool>, Path(param): Path<String>) -> Result<Json<CourseNames>, ApiError>
{
    let map = param;
    //NOTE: Fetch courses in alphabetic order?
    let map_course_names: Vec<CourseName> = sqlx::query_as!(CourseName, r#"SELECT course_name FROM course WHERE mapfk=?"#, map).fetch_all(&pool).await?;

    Ok(Json(CourseNames {
        course_count: map_course_names.len(),
        course_names: map_course_names,
    }))
}

#[axum_macros::debug_handler]
async fn get_course_data(Extension(pool): Extension<MySqlPool>, Path(param): Path<(String, String)>) -> Result<Json<Course>, ApiError>
{
    let map_name = param.0;
    let course_name = param.1;

    let course_data = sqlx::query_as!(Course, r#"SELECT * FROM course WHERE course_name=? AND mapfk=?"#, course_name, map_name).fetch_one(&pool).await?;

    Ok(Json(Course {
        course_id: course_data.course_id,
        course_name: course_data.course_name,
        mapfk: course_data.mapfk,
        reverse: course_data.reverse,
    }))
}

