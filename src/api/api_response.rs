use actix_web::HttpResponse;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ResultOk<T> {
    pub code: u16,
    pub data: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResultErr<'r> {
    pub code: u16,
    pub err_msg: &'r str,
}

pub fn ok<T: Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok().json(ResultOk { code: 200, data })
}

pub fn err(code: u16, err_msg: impl AsRef<str>) -> HttpResponse {
    HttpResponse::Ok().json(ResultErr {
        code,
        err_msg: err_msg.as_ref(),
    })
}
