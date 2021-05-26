pub mod demo;
pub mod parser;
pub mod data_table;
pub mod entity;
pub mod prop;
pub mod prop_types;
pub mod math;
pub mod weapon;
pub mod gsi;
pub mod db;
pub mod schema;
pub mod rabbitmq;
pub mod summary;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all="camelCase")]
pub struct CsgoListQuery {
    modes: Option<Vec<String>>,
    maps: Option<Vec<String>>,
    has_vod: Option<bool>,
    has_demo: Option<bool>,
}