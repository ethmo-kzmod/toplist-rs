mod pages;

use sycamore::prelude::*;
use sycamore_router::HistoryIntegration;
use sycamore_router::{Route, Router};

#[derive(Route)]
enum AppRoutes 
{
    #[to("/")]
    Maps,
    #[to("/map/<map_name>")]
    Records { map_name: String },
    #[to("/profile/<steamid>")]
    Profile { steamid: String },
    #[not_found]
    NotFound,
}

#[component(inline_props)]
async fn Switch<'a, G: Html>(cx: Scope<'a>, route: &'a ReadSignal<AppRoutes>) -> View<G>
{
    view! 
    { 
        cx,
        (match route.get().as_ref()
         {
            AppRoutes::Maps => view! { cx, pages::maps::MapList() },
            AppRoutes::Records { map_name } => view! { cx, pages::records::RecordsPage(map_name=map_name.clone()) },
            AppRoutes::Profile {steamid } => view! { cx, pages::profile::ProfilePage(steamid=steamid.clone()) },
            AppRoutes::NotFound => view! { cx, "404 Page Not Found"}
        })
    }
}

#[component]
fn App<G: Html>(cx: Scope) -> View<G>
{
    view!
    {
        cx,
        Router(
            integration=HistoryIntegration::new(),
            view=|cx: Scope, route: &ReadSignal<AppRoutes>| view!
            {
                cx,
                Switch(route=route)
            }
        )
    }
}

fn main()
{
    sycamore::render(|cx| 
    {
        view!
        {
            cx,
            App {}
        }
    });
}

