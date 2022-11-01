use rocket::fairing::AdHoc;
use rocket::serde::{Serialize, Deserialize, json::Json, json::Value};
use std::borrow::Cow;
use rocket::serde::json::serde_json::json;

// The type to represent the ID of a message.
type Id = usize;

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct Message<'r> {
    id: Option<Id>,
    func: Cow<'r, str>,
    params: Cow<'r, str>
}

#[get("/")]
async fn pin() -> &'static str {
    "Hello, world!"
}

#[post("/", format = "json", data = "<message>")]
async fn call(message: Json<Message<'_>>) -> Value {
    let func = message.func.to_string();
    let params = message.params.to_string();
    let param_list: Vec<&str> = params.split(",").collect();
    json!({ "status": "ok", "func": func, "param_list": param_list })
}


pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Requests stage", |rocket| async {
        rocket
            .mount("/call_function", routes![call])
            .mount("/pin", routes![pin])
    })
}