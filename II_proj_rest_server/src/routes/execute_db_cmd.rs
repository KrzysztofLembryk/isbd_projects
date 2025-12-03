use crate::db::db_client::DbClient;
use crate::db::manager::messages::{DbClientMsg, ResMsg};

use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use uuid::Uuid;
use actix_web::{web};

pub async fn execute_db_command(
    db_client: &web::Data<tokio::sync::RwLock<DbClient>>,
    conn_id: &Uuid,
    msg: DbClientMsg,
) -> UnboundedReceiver<ResMsg>
{
    // TODO: add correct error handling
    let (tx_conn, rx_conn) = unbounded_channel::<ResMsg>();

    let client_lock = db_client.read().await;

    client_lock
        .send_msg(DbClientMsg::Register(conn_id.clone(), tx_conn)).unwrap();
    client_lock
        .send_msg(msg).unwrap();

    drop(client_lock);

    rx_conn
}