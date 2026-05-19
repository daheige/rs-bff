use serde::{Deserialize, Serialize};

// 定义路由 handler
pub mod user;
pub mod order;

#[derive(Deserialize, Serialize, Debug)]
pub struct Reply<T> {
    pub code: i32,
    pub message: String,
    pub data: Option<T>,
}

// empty object,like {}
#[derive(Deserialize, Serialize, Debug)]
pub struct EmptyObject {}

// empty array,like:[]
type EmptyArray = Vec<EmptyObject>;
