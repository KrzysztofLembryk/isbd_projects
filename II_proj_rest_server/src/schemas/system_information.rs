use serde;

#[derive(serde::Serialize)]
struct SystemInformation
{
    #[serde(rename = "interfaceVersion")]
    interface_version: Option<String>,
    version: String,
    author: Option<String>,
}