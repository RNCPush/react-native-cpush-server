#![feature(lazy_cell)]
use std::sync::LazyLock;

use serde::Deserialize;

#[macro_export]
macro_rules! skip_serialize_none {
    ( @ {$name:ident {}} -> ($($result:tt)*) ) => (
        #[derive(Debug, Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct $name {
            $($result)*
        }
    );
    ( @ {$name:ident { $v:vis $param:ident : Option<$type:ty>, $($rest:tt)* }} -> ($($result:tt)*) ) => (
        skip_serialize_none!(@ {$name { $($rest)* }} -> (
            $($result)*
            #[serde(skip_serializing_if = "Option::is_none")]
            $v $param : Option<$type>,
        ));
    );
    ( @ { $name:ident { $v:vis $param:ident : $type:ty, $($rest:tt)* }} -> ($($result:tt)*) ) => (
        skip_serialize_none!(@ {$name { $($rest)* }} -> (
            $($result)*
            $v $param : $type,
        ));
    );
    ( $name:ident { $( $v:vis $param:ident : $($type:tt)* );* } ) => (
        skip_serialize_none!(@ { $name { $( $v $param : $($type)*),* } } -> ());
    );
}

pub mod api;
pub mod log_conf;
mod pkg;

#[derive(Deserialize)]
pub struct Db {
    pub mdb_url: String,
}

#[derive(Deserialize)]
pub struct CpushConfig {
    pub server_port: u16,
    pub db: Db,
}

fn config_load() -> anyhow::Result<CpushConfig> {
    let config = std::fs::read_to_string("cpush.toml")?;
    let config: CpushConfig = toml::from_str(&config)?;
    Ok(config)
}

pub static CPUSH_CONFIG: LazyLock<CpushConfig> =
    LazyLock::new(|| config_load().expect("配置文件读取失败"));

pub async fn mdb_connection() -> anyhow::Result<mongodb::Database> {
    let client_options = mongodb::options::ClientOptions::parse(&CPUSH_CONFIG.db.mdb_url).await?;
    let client = mongodb::Client::with_options(client_options)?;
    let db = client.database("cpush");
    pkg::UpdatePkg::create_index(&db).await?;
    Ok(db)
}
