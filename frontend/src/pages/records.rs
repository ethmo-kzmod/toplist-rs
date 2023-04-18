use crate::pages::BASE_API_URL;
use sycamore::futures::spawn_local_scoped;
use sycamore::suspense::Suspense;
use sycamore::prelude::*;
use serde::{Serialize, Deserialize};
use web_sys::HtmlElement;

//NOTE: Structs copied over from the backend
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct CourseNames
{
    course_count: usize,
    course_names: Vec<CourseName>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct CourseName
{
    course_name: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct Courses
{
    course_count: usize,
    courses: Vec<Course>,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
struct Course
{
    course_id: i32,
    course_name: String,
    mapfk: String,
    reverse: i8,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct Records
{
    records_count: usize,
    records: Vec<Record>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct Record
{
    player_name: String,
    steamid: String,
    course_time: String,
    date: String,
    diff: String,
}

async fn get_course_names(map_name: &str) -> Result<CourseNames, reqwest::Error>
{
    let url = format!("{}/{}", BASE_API_URL, map_name);
    let request = reqwest::get(&url).await?.json::<CourseNames>().await?;
    Ok(request)
}

async fn get_course_data(map_name: &str, course_name: &str) -> Result<Course, reqwest::Error>
{
    let url = format!("{}/{}/{}", BASE_API_URL, map_name, course_name);
    let request = reqwest::get(&url).await?.json::<Course>().await?;
    Ok(request)
}

async fn get_records(checkpoints: bool, map_name: &str, course_name: &str, courseid: i32, rflag: i8) -> Result<Records, reqwest::Error>
{
    let url = format!("{}/records/{}/{}/{}/{}/{}", BASE_API_URL, checkpoints, map_name, course_name, courseid, rflag);
    let request = reqwest::get(&url).await?.json::<Records>().await?;
    Ok(request)
}

#[component(inline_props)]
async fn CoursesComponent<'a, G: Html>(cx: BoundedScope<'a, 'a>, map_name: String) -> View<G>
{
    let state = use_context::<RcSignal<String>>(cx);
    let data = get_course_names(&map_name).await.unwrap_or_default();
    let first = create_node_ref(cx);

    let views = View::new_fragment(data.course_names.into_iter().enumerate().map(|(i, course)| 
    {   
        let course_name = course.course_name.clone();

        if i == 0
        {
            view!
            {
                cx,
                div(ref=first, on:click=move |_| state.set(course_name.clone()), class="font-bold text-primary hover:scale-110 hover:bg-primary hover:text-secondary duration-150 hover:cursor-pointer bg-transparent border-2 border-solid border-primary px-2 py-4")
                {
                        (course.course_name)
                }
            }
        }
        else
        {
            view!
            {
                cx,
                div(on:click=move |_| state.set(course_name.clone()), class="font-bold text-primary hover:scale-110 hover:bg-primary hover:text-secondary duration-150 hover:cursor-pointer bg-transparent border-2 border-solid border-primary px-2 py-4")
                {
                    (course.course_name)
                } 
            }
        }
    }).collect());

    first.get::<DomNode>().unchecked_into::<HtmlElement>().click();

    view!
    {
        cx,
        (views)
    }
}

#[component(inline_props)]
async fn RecordsComponent<G: Html>(cx: Scope<'_>, map_name: String) -> View<G>
{
    let state = use_context::<RcSignal<String>>(cx);
    let checkpoints = use_context::<RcSignal<bool>>(cx);
    let course_data = create_signal(cx, Course { course_id: -1, course_name: String::from(""), mapfk: String::from(""), reverse: -1 });
    let data = create_signal(cx, Records {records_count:0, records: vec![Record { player_name: String::from(""), steamid: String::from(""), course_time: String::from(""), date: String::from(""), diff: String::from("") }]});
    let records = create_signal(cx, data.get().records.clone());

    create_effect(cx, move ||
    {
        let map = map_name.clone(); //NOTE: needed due to move in spawn_local_scoped
        state.track(); //NOTE: we need to explicitly track reactive variable or it won't be tracked inside spawn_local_scoped
        spawn_local_scoped(cx, async move
        {
            if !state.get().is_empty()
            {
                course_data.set(get_course_data(&map, &state.get().as_ref().clone()).await.unwrap_or_default());
            }
        });
    });

    create_effect(cx, move ||
    {
        course_data.track();
        checkpoints.track();
        spawn_local_scoped(cx, async move
        {
            if course_data.get().course_id != -1
            {
                data.set(get_records(*checkpoints.get(), &course_data.get().mapfk.to_string(), &course_data.get().course_name.to_string(), course_data.get().course_id, course_data.get().reverse).await.unwrap_or_default());
                records.set(data.get().records.clone());
            }
        });
    });

    view!
    {
        cx,
        tbody
        {
            (
                {
                    let views = View::new_fragment(records.get().as_ref().clone().into_iter().enumerate().map(|(i, record)| 
                    {   
                        let i = i + 1;

                        //NOTE: Alternating background color for each row
                        let bg = if i % 2 == 0 
                        {
                            "bg-ternary"
                        }
                        else {
                            "bg-secondary"
                        };
                        
                        view!
                        {
                            cx,
                           tr(class=bg)
                           {
                                td(class="px-8 py-4 whitespace-nowrap text-sm xl:text-base 3xl:text-xl font-bold text-white")
                                {
                                    (i)
                                }
                                a(href="/#") //TODO: Implement player profiles and link to them
                                {
                                    td(class="font-bold max-w-[300px] text-ellipsis overflow-hidden hover:scale-110 hover:text-primary duration-150 hover:cursor-pointer text-sm xl:text-base 3xl:text-xl text-gray-400 px-8 py-4")
                                    {
                                        (record.player_name)
                                    }
                                }
                                td(class="font-bold max-w-[300px] text-ellipsis overflow-hidden hover:scale-110 hover:text-primary duration-150 hover:cursor-pointer text-sm xl:text-base 3xl:text-xl text-gray-400 px-8 py-4")
                                {
                                    (record.course_time)
                                }
                                td(class="font-bold max-w-[300px] text-ellipsis overflow-hidden hover:scale-110 hover:text-primary duration-150 hover:cursor-pointer text-sm xl:text-base 3xl:text-xl text-gray-400 px-8 py-4")
                                {
                                    (record.date)
                                }
                                td(class="font-bold max-w-[300px] text-ellipsis overflow-hidden hover:scale-110 hover:text-primary duration-150 hover:cursor-pointer text-sm xl:text-base 3xl:text-xl text-gray-400 px-8 py-4")
                                {
                                   (record.diff)
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
}

#[component(inline_props)]
pub fn RecordsPage<G: Html>(cx: Scope, map_name: String) -> View<G>
{
    let course_signal = create_rc_signal(String::new());
    let cp_signal = create_rc_signal(false);
    provide_context(cx, cp_signal.clone());
    provide_context(cx, course_signal.clone());
    let map_name_clone = map_name.clone();

    view!
    {
        cx,
        Suspense(fallback=view! { cx, } )
        {
            h1(class="mb-0 2xl:mb-6 text-4xl font-bold pt-12 pb-6 bg-background text-center text-primary")
            {
                //NOTE: It may be a good idea to display map name without .ugc123456789
                (map_name)
            }
            div(class="scale-75 lg:scale-90 xl:scale-100 flex justify-center items-start gap-4")
            {
                div(class="shadow-2xl h-full text-center text-sm bg-card p-4 sticky top-0 items-start")
                {
                    label(class="ml-0 mt-6 mb-6 mr-6 relative inline-flex items-center cursor-pointer")
                    {
                        input(on:change=move |_| cp_signal.set(!*cp_signal.get()), type="checkbox", value="", class="sr-only peer")
                        {
                        }
                        div(class="w-11 h-6 bg-gray-300 rounded-full peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-0.5 after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-primary")
                        {
                        }
                        span(class="ml-3 text-sm font-bold text-primary")
                        {
                            "Checkpoints"
                        }
                    }
                    div(class="flex flex-col gap-6")
                    {
                        CoursesComponent(map_name=map_name)
                    }
                }
                div(class="max-w-[1140px] flex flex-col")
                {
                    div(class="shadow-2xl overflow-x-auto")
                    {
                        div(class="inline-block min-w-full")
                        {
                            div(class="overflow-hidden", id="records")
                            {
                                table(class="rounded-lg min-w-full")
                                {
                                    thead(class="bg-primary")
                                    {
                                        tr 
                                        {
                                            th(scope="col", class="text-sm xl:text-base 3xl:text-xl font-bold text-secondary px-8 py-4 text-left"){"#"}
                                            th(scope="col", class="text-sm xl:text-base 3xl:text-xl font-bold text-secondary px-8 py-4 text-left"){"Player"}
                                            th(scope="col", class="text-sm xl:text-base 3xl:text-xl font-bold text-secondary px-8 py-4 text-left"){"Time"}
                                            th(scope="col", class="text-sm xl:text-base 3xl:text-xl font-bold text-secondary px-8 py-4 text-left"){"Date"}
                                            th(scope="col", class="text-sm xl:text-base 3xl:text-xl font-bold text-secondary px-8 py-4 text-left"){"WR"}
                                        }
                                    }
                                    RecordsComponent(map_name=map_name_clone)
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
