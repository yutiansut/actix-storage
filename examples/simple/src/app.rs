use std::collections::HashMap;

use actix_storage::{Format, Storage};
use actix_web::{web, App, Error, HttpServer};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Person {
    name: String,
    points: HashMap<String, u16>,
}

#[derive(Serialize)]
struct PersonOut {
    new: bool,
    previous_point: Option<u16>,
    name: String,
    points: HashMap<String, u16>,
}

/// We get a name, a lesson and a point for the combination. If the name was not defined, we set new to true,
/// if the lesson for the name already had a point, we return that point with output, and we return all the data,
/// for that name at the end.
#[actix_web::get("/{name}/{lesson}/{point}")]
async fn index(
    path: web::Path<(String, String, u16)>,
    storage: Storage,
) -> Result<web::Json<PersonOut>, Error> {
    let new: bool;
    let (name, lesson, point) = path.into_inner();
    let mut previous_point: Option<u16> = None;

    let person = if let Some(mut person) = storage.get::<_, Person>(&name).await? {
        new = false;
        if let Some(point) = person.points.insert(lesson, point) {
            previous_point = Some(point);
        }

        person
    } else {
        new = true;
        let mut person = Person {
            name: name.clone(),
            points: HashMap::new(),
        };
        person.points.insert(lesson, point);

        person
    };

    // Setting back the data to storage
    storage.set(&name, &person).await?;

    let out = PersonOut {
        name: person.name,
        points: person.points,
        new,
        previous_point,
    };

    Ok(web::Json(out))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let store = actix_storage_dashmap::DashMapStore::new();
    // OR
    // let store = actix_storage_hashmap::HashMapStore::new();
    // OR
    // let store = actix_storage_redis::RedisBackend::connect_default().await.unwrap();
    // OR
    // let store = actix_storage_sled::SledStore::from_db(
    //     actix_storage_sled::SledConfig::default()
    //         .temporary(true)
    //         .open()?,
    // );

    let storage = Storage::build().store(store).format(Format::Json).finish();

    let server = HttpServer::new(move || App::new().app_data(storage.clone()).service(index));
    server.bind("localhost:5000")?.run().await
}
