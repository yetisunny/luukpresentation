use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use minijinja::Environment;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::OnceLock;

struct Slide {
    title: &'static str,
    body: &'static str,
}

const SLIDES: &[Slide] = &[
    Slide {
        title: "Hello",
        body: "A tiny presentation served with Axum, Minijinja, and HTMX.",
    },
    Slide {
        title: "Axum",
        body: "Handles routes and returns HTML fragments for each slide.",
    },
    Slide {
        title: "Minijinja",
        body: "Renders templates on the server — full page or partials.",
    },
    Slide {
        title: "HTMX",
        body: "Back and forward buttons swap slides in place without a full reload.",
    },
];

fn env() -> &'static Environment<'static> {
    static ENV: OnceLock<Environment<'static>> = OnceLock::new();
    ENV.get_or_init(|| {
        let mut env = Environment::new();
        env.add_template("index.html", include_str!("../templates/index.html"))
            .expect("index template");
        env.add_template("slide.html", include_str!("../templates/slide.html"))
            .expect("slide template");
        env
    })
}

fn render(template: &str, ctx: minijinja::Value) -> Result<Html<String>, (StatusCode, String)> {
    env()
        .get_template(template)
        .and_then(|tmpl| tmpl.render(ctx))
        .map(Html)
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, format!("template error: {err}")))
}

#[derive(Deserialize)]
struct SlideQuery {
    dir: Option<String>,
}

fn slide_context(index: usize, dir: Option<&str>) -> Option<minijinja::Value> {
    let slide = SLIDES.get(index)?;
    let dir = dir.filter(|d| *d == "next" || *d == "prev");
    Some(minijinja::context! {
        index => index,
        total => SLIDES.len(),
        title => slide.title,
        body => slide.body,
        prev => index.checked_sub(1),
        next => (index + 1 < SLIDES.len()).then_some(index + 1),
        dir => dir,
    })
}

async fn other_screen() -> impl IntoResponse {
    return "<!DOCTYPE html>"
}

async fn index() -> impl IntoResponse {
    let Some(slide) = slide_context(0, None) else {
        return (StatusCode::NOT_FOUND, "no slides".to_string()).into_response();
    };

    match render(
        "index.html",
        minijinja::context! {
            page_title => "Slides",
            ..slide
        },
    ) {
        Ok(html) => html.into_response(),
        Err(err) => err.into_response(),
    }
}

async fn slide(Path(index): Path<usize>, Query(query): Query<SlideQuery>) -> impl IntoResponse {
    let Some(ctx) = slide_context(index, query.dir.as_deref()) else {
        return (StatusCode::NOT_FOUND, format!("slide {index} not found")).into_response();
    };

    match render("slide.html", ctx) {
        Ok(html) => html.into_response(),
        Err(err) => err.into_response(),
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(index))
        .route("/slides/{index}", get(slide));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind");
    axum::serve(listener, app).await.expect("server error");
}
