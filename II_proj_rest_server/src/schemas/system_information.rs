use serde;

#[derive(serde::Serialize)]
pub struct SystemInformation
{
    #[serde(rename = "interfaceVersion")]
    pub interface_version: String,
    pub version: String,
    pub author: String,
    pub uptime: i64
}