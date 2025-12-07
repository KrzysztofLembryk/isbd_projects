use crate::schemas::system_information::SystemInformation;
use actix_web::{HttpResponse, Responder, get};

#[get("/system/info")]
async fn get_sys_info() -> impl Responder
{
    return HttpResponse::Ok()
        .json(
            SystemInformation {
                interface_version: String::from("1.0.1"),
                version: String::from("1.0.0"),
                author: String::from("Krzysztof Lembryk"),
                uptime: 0
            }
        );
}