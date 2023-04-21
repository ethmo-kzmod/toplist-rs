use crate::pages::BASE_API_URL;
use sycamore::prelude::*;
use sycamore::futures::spawn_local_scoped;
use sycamore::suspense::Suspense;
use serde::{Serialize, Deserialize};

//NOTE: Structs for SteamAPI response
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct Response
{
    response: Players, 
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
struct Players
{
    players: Vec<Player>,           //NOTE: We are fetching data for only one SteamID at a time but the Steam API accepts multiple SteamIDs and returns an array of players
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
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


#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct PlayerRecords
{
    records_count: usize,
    records: Vec<PlayerRecord>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct PlayerRecord
{
    map_name: String,
    course_name: String,
    course_time: String,
    date: String,
}

async fn get_player_data(steamid64: &str) -> Result<Vec<Player>, reqwest::Error>
{
    let url = format!("{}/player/info/{}", BASE_API_URL, steamid64);
    let request = reqwest::get(&url).await?.json::<Response>().await?.response.players;
    Ok(request)
}

async fn get_player_records(checkpoints: bool, steamid64: &str) -> Result<PlayerRecords, reqwest::Error>
{
    let url = format!("{}/player/records/{}/{}", BASE_API_URL, checkpoints, steamid64);
    let request = reqwest::get(&url).await?.json::<PlayerRecords>().await?;
    Ok(request)
}

#[component(inline_props)]
async fn ProfileComponent<'a, G: Html>(cx: BoundedScope<'a, 'a>, steamid: String) -> View<G>
{
    let data = get_player_data(&steamid).await.unwrap_or_default();
    let avatar = data[0].clone().avatarfull;
    let profile_url = data[0].clone().profileurl;
    let records_count = use_context::<RcSignal<u32>>(cx);
    let cp_signal = use_context::<RcSignal<bool>>(cx);

    view!
    {
        cx,
        img(src=avatar) {}
        a(href=profile_url)
        {
            p(class="text-primary font-bold uppercase pt-2 text-2xl")
            {
                (data[0].personaname)
            }
        }
        p(class="text-primary font-bold uppercase pt-2 text-base")
        {
            "Records (" (records_count.get()) ")" 
        }
        label(class="ml-0 mt-4 mb-6 mr-6 relative inline-flex items-center cursor-pointer")
        {
            input(on:change=move |_| cp_signal.set(!*cp_signal.get()), type="checkbox", value="", class="sr-only peer") {}
            div(class="w-11 h-6 bg-gray-300 rounded-full peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-0.5 after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary") {}
            span(class="ml-3 text-sm font-bold text-primary") { "Checkpoints" }
        }
    }
}

#[component(inline_props)]
async fn PlayerRecordsComponent<'a, G: Html>(cx: BoundedScope<'a, 'a>, steamid: String) -> View<G>
{
    let records_count = use_context::<RcSignal<u32>>(cx);
    let cp_signal = use_context::<RcSignal<bool>>(cx);
    let data = create_signal(cx, PlayerRecords { records_count: 0, records: vec![PlayerRecord { map_name: String::from(""), course_name: String::from(""), course_time: String::from(""), date: String::from("") }] });
    let records = create_signal(cx, data.get().records.clone());

    create_effect(cx, move ||
    {
        let steamid_clone = steamid.clone();

        cp_signal.track();
        spawn_local_scoped(cx, async move
        {
            data.set(get_player_records(*cp_signal.get(), &steamid_clone).await.unwrap_or_default());
            records_count.set(data.get().records_count as u32);
            records.set(data.get().records.clone());
        });
    });
    
    view!
    {
        cx,
        (
            {
                let views = View::new_fragment(records.get().as_ref().clone().into_iter().enumerate().map(|(i, record)|
                {
                    let i = i + 1;

                    //NOTE: Alternating background for each row
                    let bg = if i % 2 == 0
                    {
                        "bg-ternary"
                    }
                    else
                    {
                        "bg-secondary"
                    };
                    
                    let map = record.map_name.clone();

                    view!
                    {
                        cx,
                        tr(class=bg)
                        {
                            td(class="px-8 py-4 whitespace-nowrap text-sm xl:text-base 3xl:text-xl font-bold text-white")
                            {
                                (i)
                            }
                            a(href=format!("/{}", map))
                            {
                                td(class="max-w-[300px] text-ellipsis overflow-hidden hover:scale-110 hover:text-primary duration-150 hover:cursor-pointer text-sm xl:text-base 3xl:text-xl text-gray-400 font-bold px-6 py-4 whitespace-nowrap")
                                {
                                    (record.map_name)
                                }
                            }
                            td(class="max-w-[300px] text-ellipsis overflow-hidden hover:scale-110 hover:text-primary duration-150 hover:cursor-pointer text-sm xl:text-base 3xl:text-xl text-gray-400 font-bold px-6 py-4 whitespace-nowrap")
                            {
                                (record.course_name)
                            }
                            td(class="max-w-[300px] text-ellipsis overflow-hidden hover:scale-110 hover:text-primary duration-150 hover:cursor-pointer text-sm xl:text-base 3xl:text-xl text-gray-400 font-bold px-6 py-4 whitespace-nowrap")
                            {
                                (record.course_time)
                            }
                            td(class="max-w-[300px] text-ellipsis overflow-hidden hover:scale-110 hover:text-primary duration-150 hover:cursor-pointer text-sm xl:text-base 3xl:text-xl text-gray-400 font-bold px-6 py-4 whitespace-nowrap")
                            {
                                (record.date)
                            }
                        }
                    }
                }).collect());

                view!
                {
                    cx,
                    (views)
                }
            }
        )
    }
}

#[component(inline_props)]
pub fn ProfilePage<G: Html>(cx: Scope, steamid: String) -> View<G>
{
    let records_count = create_rc_signal(u32::MIN);
    provide_context(cx, records_count);
    let cp_signal = create_rc_signal(false);
    provide_context(cx, cp_signal);
    let steamid_clone = steamid.clone();
    
    view!
    {
        cx,
        Suspense(fallback=view! {cx, })
        {
            div(class="mt-40 scale-75 lg:scale-90 xl:scale-100 ml-auto mr-auto flex justify-center items-start gap-4")
            {
                div(class="shadow-2xl h-full text-sm bg-card p-6 xl:p-10 max-w-[264px] sticky top-0")
                {
                    ProfileComponent(steamid=steamid)
                }
                div(class="shadow-2xl flex flex-col")
                {
                    table(class="min-w-full")
                    {
                        thead(class="bg-primary")
                        {
                            tr
                            {
                                th(class="text-sm xl:text-base 3xl:text-xl font-bold text-secondary px-6 py-4 text-left")
                                {
                                    "#"
                                }
                                th(class="text-sm xl:text-base 3xl:text-xl font-bold text-secondary px-6 py-4 text-left")
                                {
                                    "Map"
                                }
                                th(class="text-sm xl:text-base 3xl:text-xl font-bold text-secondary px-6 py-4 text-left")
                                {
                                    "Course"
                                }
                                th(class="text-sm xl:text-base 3xl:text-xl font-bold text-secondary px-6 py-4 text-left")
                                {
                                    "Time"
                                }
                                th(class="text-sm xl:text-base 3xl:text-xl font-bold text-secondary px-6 py-4 text-left")
                                {
                                    "Date"
                                }
                            }
                        }
                        tbody
                        {
                            PlayerRecordsComponent(steamid=steamid_clone)
                        }
                    }
                }
            }
        }
    }
}
