use leptos::*;
use leptos_meta::*;
use leptos_router::*;

#[path="./routes/home.rs"]
mod home;
use home::Home;

#[path="./routes/docs.rs"]
mod docs;
use docs::Docs;

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <main>
            <Title formatter=|text| format!("Zippy • {text}")/>
            <Stylesheet id="leptos" href="/pkg/tailwind.css"/>
            <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
            <Meta charset="utf-8"/>
            <Meta name="description" content="Zippy is an opensource, cross-platform http server."/>
        </main>
        <Router>
            <Routes>
                <Route path="/" view=  move || view! { <Home/> }/>
                <Route path="/docs" view=Docs/>
                // <Route path="/users/:id" view=UserProfile/>
                <Route path="/*any" view=|| view! { <h1>"Not Found"</h1> }/>
            </Routes>
        </Router>
    }
}