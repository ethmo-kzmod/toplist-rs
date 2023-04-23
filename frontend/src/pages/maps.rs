use crate::pages::BASE_API_URL;
use sycamore::suspense::Suspense;
use sycamore::prelude::*;
use serde::{Serialize, Deserialize};

//NOTE: Structs copied over from the backend
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct Maps
{
    maps: Vec<Map>
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct Map
{
    map_name: String,
    times_played: i32,
    time_added: String
}

async fn get_maps() -> Result<Maps, reqwest::Error>
{
    let url = format!("{}/maps", BASE_API_URL);
    let request = reqwest::get(&url).await?.json::<Maps>().await?;
    Ok(request)
}

#[component]
async fn MapListComponent<G: Html>(cx: Scope<'_>) -> View<G>
{
    let data = get_maps().await.unwrap_or_default();

    let views = View::new_fragment(data.maps.into_iter().skip(1).enumerate().map(|(i, map)| 
    {   
        let i = i + 1;

        //NOTE: Alternating background color for each row
        let bg = if i % 2 == 0 
        {
            "bg-ternary"
        }
        else
        {
            "bg-secondary"
        };

        //NOTE: Separate variable for formatting <a> tags as using map.map_name directly causes a move
        let map_name = map.map_name.clone();

        view!
        {
            cx,
            tr(class=bg)
            {
                td(class="px-8 py-4 whitespace-nowrap text-sm xl:text-base 3xl:text-xl font-bold text-white")
                {
                    (i)
                }
                a(href=format!("/map/{}",map_name))
                {
                    td(class="font-bold max-w-[300px] text-ellipsis overflow-hidden hover:scale-110 hover:text-primary duration-150 hover:cursor-pointer text-sm xl:text-base 3xl:text-xl text-gray-400 px-8 py-4")
                    {
                        (map.map_name)
                    }
                }
                td(class="font-bold max-w-[300px] text-ellipsis overflow-hidden hover:scale-110 hover:text-primary duration-150 hover:cursor-pointer text-sm xl:text-base 3xl:text-xl text-gray-400 px-8 py-4")
                {
                    (map.time_added)
                }
                td(class="font-bold max-w-[300px] text-ellipsis overflow-hidden hover:scale-110 hover:text-primary duration-150 hover:cursor-pointer text-sm xl:text-base 3xl:text-xl text-gray-400 px-8 py-4 text-right")
                {
                    (map.times_played)
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

#[component]
pub fn MapList<G: Html>(cx: Scope) -> View<G>
{
    view!
    {
        cx,
        Suspense(fallback=view! {cx, } )
        {
            h1(class="mb-0 2xl:mb-6 text-4xl font-bold pt-12 pb-6 bg-background text-center text-primary") { "Maps" }
            div(class="scale-75 lg:scale-90 xl:scale-100 flex justify-center items-start gap-4")
            {
                div(class="max-w-[1140px] flex flex-col")
                {
                    div(class="shadow-2xl overflow-x-auto")
                    {
                        div(class="inline-block min-w-full")
                        {
                            div(class="overflow-hidden")
                            {
                                table(class="rounded-lg min-w-full")
                                {
                                    thead(class="bg-primary")
                                    {
                                        tr
                                        {
                                            th(scope="col", class="text-sm xl:text-base 3xl:text-xl font-bold text-secondary px-8 py-4 text-left")
                                            {
                                                "#"
                                            }
                                            th(scope="col", class="text-sm xl:text-base 3xl:text-xl font-bold text-secondary px-8 py-4 text-left")
                                            {
                                                "Map name"
                                            }
                                            th(scope="col", class="text-sm xl:text-base 3xl:text-xl font-bold text-secondary px-8 py-4 text-left")
                                            {
                                                "Date added"
                                            }
                                            th(scope="col", class="text-sm xl:text-base 3xl:text-xl font-bold text-secondary px-8 py-4 text-left")
                                            {
                                                "Times played"
                                            }
                                        }
                                    }
                                    tbody
                                    {
                                        MapListComponent {}
                                    }
                                }
                            }
                        }
                    }
                    p(class="text-right text-sm font-bold text-primary pt-2 pb-4")
                    {
                        "Toplist by Menko and GoldenNinja"
                    }
                }
            }
        }
    }
} 
