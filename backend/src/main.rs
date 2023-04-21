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

#[derive(Deserialize)]
struct PlayerParams
{
    checkpoints: bool,
    steamid: String,
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

#[derive(Serialize, Deserialize)]
struct PlayerRecords
{
    records_count: usize,
    records: Vec<PlayerRecord>,
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
struct PlayerRecord
{
    map_name: String,
    course_name: String,
    course_time: String,
    date: Option<String>,
    //diff: String,         //NOTE: Calculating difference to WR for every course and map a player may have a record on seems wasteful
}

#[derive(Serialize, Deserialize)]
struct SteamResponse 
{
    response: Players,
}

#[derive(Serialize, Deserialize,)]
struct Players
{
    players: Vec<Player>,           //NOTE: We are fetching data for only one SteamID at a time but the Steam API accepts multiple SteamIDs and returns an array of players
}

#[derive(Serialize, Deserialize)]
struct Player
{
    steamid: String,                //NOTE: 64 bit SteamID
    communityvisibilitystate: u32,  //NOTE: Profile visibility. 1 - private, 2 - friends, 3 - friends of friends, 4 - logged in Steam users, 5 - public
    profilestate: u32,              //NOTE: If set, indicates the user has a community profile configured (will be set to 1)
    personaname: String,            //NOTE: Display name
    lastlogoff: Option<u32>,        //NOTE: Unix timestamp of last logoff
    profileurl: String,             //NOTE: Full URL to the Steam profile
    avatar: String,                 //NOTE: Full URL to the 32x32 pixels version of the avatar
    avatarmedium: String,           //NOTE: 64x64 version of the avatar
    avatarfull: String,             //NOTE: 184x184 version of the avatar
    avatarhash: String,             //NOTE: Hash of the avatar (used in URLs in the avatar/avatarmedium/avatarfull fields) 
    personastate: u32,              //NOTE: User's status: 0 - offline, 1 - online, 2 - busy,3 - away, 4 -Snooze, 5 - looking to trade, 6 - looking to play
    commentpermission: Option<u32>, //NOTE: Are comments allowed on the profile?
    realname: Option<String>,       //NOTE: Name of the player
    primaryclanid: Option<String>,  //NOTE: 64 bit ID of the user's primary group
    timecreated: Option<u32>,       //NOTE: Unix timestamp of when the profile was created
    personastateflags: Option<u32>, //NOTE: ?????
    loccountrycode: Option<String>, //NOTE: ISO 3166 country code
    locstatecode: Option<String>,   //NOTE: Code of the area of the country
    loccityid: Option<u32>,         //NOTE: ID of the city the player is from
    gameid: Option<String>,         //NOTE: ID of the game the player is currently playing
    gameextrainfo: Option<String>,  //NOTE: Name of the game the player is currently playing
    gameserverip: Option<String>,   //NOTE: IP of the server the player is currently on
}

enum ApiError
{
    NotFound,
    DatabaseError(sqlx::Error),
    ReqwestError(reqwest::Error),
}

impl IntoResponse for ApiError
{
    fn into_response(self) -> Response
    {
        let (status, err_msg) = match self
        {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "These aren't the droids you're looking for."),
            ApiError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Database error."),
            ApiError::ReqwestError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Steam API error."),
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

impl From<reqwest::Error> for ApiError
{
    fn from(e: reqwest::Error) -> Self
    {
        ApiError::ReqwestError(e)
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
        .route("/api/player/info/:steamid", get(get_player_info))
        .route("/api/player/records/:checkpoints/:steamid", get(get_player_records))                                 //NOTE: We will need to convert back to steamid3 for DB query
        .layer(cors)
        .layer(Extension(pool));

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("Failed to start server");
}

//NOTE: Conversions of SteamID3 to SteamID64 through addition and SteamID64 to SteamID3 through subtraction
//For example: [U:1:19432566] -> 19432566 + 76561197960265728 = 76561197979698294
//76561197979698294 -> 76561197979698294 - 76561197960265728 = 19432566
async fn convert_steamid3(steamid3: &str) -> String
{
    let steam_id_base = 76561197960265728;              
    let last = steamid3.split(':').last().unwrap();
    let number = last[..last.len()-1].parse::<i64>().unwrap();
    let steamid64 = number + steam_id_base;
    steamid64.to_string()
}

async fn convert_steamid64(steamid64: &str) -> String
{
    let steam_id_base = 76561197960265728;
    let steamid3 = format!("[U:1:{}]", (steamid64.parse::<i64>().unwrap() - steam_id_base).to_string());    //NOTE: Assuming SteamID3 will always start with [U:1:... as all IDs in the database follow this convention
    steamid3
}

#[axum_macros::debug_handler]
async fn get_player_info(Path(param): Path<String>) -> Result<Json<SteamResponse>, ApiError>
{
    let steamid = param;
    let steam_api_key = &std::env::var("STEAM_API_KEY").unwrap(); 
    let url = format!("http://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key={}&steamids={}", steam_api_key, steamid);
    let request = reqwest::get(&url).await?.json::<SteamResponse>().await?.response;
    Ok(Json(SteamResponse {
        response: request
    }))
}

#[axum_macros::debug_handler]
async fn get_player_records(Extension(pool): Extension<MySqlPool>, Path(PlayerParams { checkpoints, steamid }): Path<PlayerParams>) -> Result<Json<PlayerRecords>, ApiError>
{

    let steamid3 = convert_steamid64(&steamid).await;

    let player_records: Vec<PlayerRecord> = if checkpoints
    {
        sqlx::query_as!(PlayerRecord, r#"SELECT m.map_name, c.course_name, r.course_time, DATE_FORMAT(r.date_set, '%Y-%m-%d') AS date
                                      FROM map m, course c, record_cp r
                                      WHERE c.mapfk=m.map_name AND r.courseidfk=c.course_id AND r.steamidfk=?
                                      ORDER BY r.date_set DESC, r.record_key DESC"#, steamid3).fetch_all(&pool).await?
    }
    else
    {

        sqlx::query_as!(PlayerRecord, r#"SELECT m.map_name, c.course_name, r.course_time, DATE_FORMAT(r.date_set, '%Y-%m-%d') AS date
                                      FROM map m, course c, record r
                                      WHERE c.mapfk=m.map_name AND r.courseidfk=c.course_id AND r.steamidfk=?
                                      ORDER BY r.date_set DESC, r.record_key DESC"#, steamid3).fetch_all(&pool).await?
    };

    Ok(Json(PlayerRecords {
        records_count: player_records.len(),
        records: player_records,
    }))
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
    //NOTE: Reverse courses are unlikely to be played with checkpoints
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
            record.steamid = convert_steamid3(&record.steamid).await;
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

