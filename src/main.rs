#[macro_use]
extern crate redis_async;

use std::io;

use actix::prelude::*;
use actix_redis::{ RedisActor, Command };
use actix_web::{ HttpServer, App, web, get, HttpResponse, Error };

use redis_async::resp::{ RespValue, FromResp };
use futures::future::join_all;

#[get("/")]
async fn index() -> String {
    "Hello World".to_owned()
}

#[get("/set/{value}")]
async fn set(redis: web::Data<Addr<RedisActor>>, value: web::Path<String>) -> Result<HttpResponse, Error> {
    let set_value = redis.send(Command(resp_array!["SET", "hello", value.to_owned()]));

    // In-case of multiple command
    let res: Vec<Result<RespValue, Error>> =
        join_all(vec![set_value].into_iter())
            .await
            .into_iter()
            .map(|item| {
                item.map_err(Error::from)
                    .and_then(|res| res.map_err(Error::from))
            })
            .collect();

    // If all command are successful
    if res.iter().all(|res| match res {
        Ok(RespValue::SimpleString(x)) if x == "OK" => true,
        _ => false,
    }) {
        Ok(
            HttpResponse::Ok()
                .body(
                    format!("Set {}", value.to_owned()
                )
            )
        )
    } else {
        Ok(HttpResponse::InternalServerError()
            .body("Something went wrong")
        )
    }
}

#[get("/get")]
async fn get(redis: web::Data<Addr<RedisActor>>) -> Result<HttpResponse, Error> {
    let res = redis
        .send(Command(resp_array![
            "GET",
            "hello"
        ]))
        .await?;

    // Require declared String to convert resp_value with FromResp::from_resp
    let data: String = match res {
        Ok(resp_value) => match FromResp::from_resp(resp_value) {
            Ok(parsed_value) => parsed_value,
            Err(_) => return Ok(HttpResponse::InternalServerError()
                .body(
                    "Key not exist"
                )
            )
        }
        Err(_) => return Ok(HttpResponse::InternalServerError()
            .body(
                "Unable to connect"
            )
        )
    };

    Ok(
        HttpResponse::Ok()
            .body(
                format!("Get {}", data)
            )
    )
}

#[get("/delete")]
async fn delete(redis: web::Data<Addr<RedisActor>>) -> Result<HttpResponse, Error> {
    let res = redis
        .send(Command(resp_array![
            "DEL",
            "hello"
        ]))
        .await?;

    match res {
        // Total success command. Depend on total DEL <requests>
        Ok(RespValue::Integer(total_request)) if total_request == 1 => {
            Ok(HttpResponse::Ok().body("Deleted"))
        }
        _ => {
            Ok(HttpResponse::InternalServerError().finish())
        }
    }
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    HttpServer::new(move || {
        let redis = RedisActor::start("127.0.0.1:6379");

        App::new()
            .data(redis)
            .service(index)
            .service(set)
            .service(get)
            .service(delete)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}